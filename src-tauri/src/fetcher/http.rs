use reqwest::{Client, StatusCode};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct HttpFetcher {
    client: Client,
    user_agent: String,
    max_retries: u32,
    base_delay_ms: u64,
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self::new(30)
    }
}

impl HttpFetcher {
    pub fn new(timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            user_agent: String::from("Docurip/0.3.1 (+https://github.com/docurip)"),
            max_retries: 3,
            base_delay_ms: 1000,
        }
    }

    /// Determine if an HTTP status code is transient (retryable).
    fn is_transient_status(status: StatusCode) -> bool {
        status.is_server_error() || status == StatusCode::REQUEST_TIMEOUT
    }

    /// Determine if a network error is transient.
    fn is_transient_error(err: &anyhow::Error) -> bool {
        let err_str = err.to_string().to_lowercase();
        err_str.contains("timeout")
            || err_str.contains("timed out")
            || err_str.contains("connection refused")
            || err_str.contains("dns")
            || err_str.contains("connect")
    }

    async fn do_fetch(&self, url: &str) -> anyhow::Result<(u16, String)> {
        let resp = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        Ok((status, text))
    }

    pub async fn fetch(&self, url: &str) -> anyhow::Result<String> {
        let (_, body) = self.fetch_with_status(url).await?;
        Ok(body)
    }

    pub async fn fetch_with_status(&self, url: &str) -> anyhow::Result<(u16, String)> {
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 0..=self.max_retries {
            match self.do_fetch(url).await {
                Ok((status, body)) => {
                    if let Ok(code) = StatusCode::from_u16(status) {
                        if code.is_success() {
                            return Ok((status, body));
                        }

                        if !Self::is_transient_status(code) || attempt == self.max_retries {
                            return Err(anyhow::anyhow!("HTTP {} for {}", status, url));
                        }

                        last_error = Some(anyhow::anyhow!("HTTP {} for {}", status, url));
                    } else {
                        return Err(anyhow::anyhow!("Invalid HTTP status {} for {}", status, url));
                    }
                }
                Err(e) => {
                    if !Self::is_transient_error(&e) || attempt == self.max_retries {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }

            if attempt < self.max_retries {
                let delay = Duration::from_millis(self.base_delay_ms * 2_u64.pow(attempt));
                sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max retries exceeded for {}", url)))
    }

    pub async fn fetch_bytes(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self.client.get(url).header("User-Agent", &self.user_agent).send().await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {} for {}", status, url);
        }
        if let Some(len) = resp.content_length() {
            const MAX_ASSET_BYTES: u64 = 50 * 1024 * 1024;
            if len > MAX_ASSET_BYTES {
                anyhow::bail!("Asset too large ({} MB limit) for {}", MAX_ASSET_BYTES / 1024 / 1024, url);
            }
        }
        let bytes = resp.bytes().await?.to_vec();
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_transient_status() {
        assert!(HttpFetcher::is_transient_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(HttpFetcher::is_transient_status(StatusCode::BAD_GATEWAY));
        assert!(HttpFetcher::is_transient_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(HttpFetcher::is_transient_status(StatusCode::GATEWAY_TIMEOUT));
        assert!(HttpFetcher::is_transient_status(StatusCode::REQUEST_TIMEOUT));
        assert!(!HttpFetcher::is_transient_status(StatusCode::BAD_REQUEST));
        assert!(!HttpFetcher::is_transient_status(StatusCode::NOT_FOUND));
        assert!(!HttpFetcher::is_transient_status(StatusCode::OK));
    }

    #[test]
    fn test_is_transient_error() {
        let timeout_err = anyhow::anyhow!("operation timed out");
        assert!(HttpFetcher::is_transient_error(&timeout_err));

        let conn_err = anyhow::anyhow!("connection refused");
        assert!(HttpFetcher::is_transient_error(&conn_err));

        let parse_err = anyhow::anyhow!("invalid json");
        assert!(!HttpFetcher::is_transient_error(&parse_err));
    }

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use wiremock::ResponseTemplate;

    struct FlakyResponder {
        count: Arc<AtomicUsize>,
    }
    impl wiremock::Respond for FlakyResponder {
        fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
            let count = self.count.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_string("Success after retries")
            }
        }
    }

    struct CountResponder {
        count: Arc<AtomicUsize>,
    }
    impl wiremock::Respond for CountResponder {
        fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
            self.count.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(404)
        }
    }

    #[tokio::test]
    async fn test_fetch_success() {
        use wiremock::{MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;
        wiremock::Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
            .mount(&mock_server)
            .await;

        let fetcher = HttpFetcher::new(30);
        let result = fetcher.fetch(&format!("{}/hello", mock_server.uri())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_fetch_retries_transient_error_then_success() {
        use wiremock::MockServer;
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicUsize::new(0));

        wiremock::Mock::given(method("GET"))
            .and(path("/flaky"))
            .respond_with(FlakyResponder { count: call_count.clone() })
            .mount(&mock_server)
            .await;

        let fetcher = HttpFetcher::new(30);
        let result = fetcher.fetch(&format!("{}/flaky", mock_server.uri())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success after retries");
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_fetch_permanent_error_no_retry() {
        use wiremock::MockServer;
        use wiremock::matchers::{method, path};

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicUsize::new(0));

        wiremock::Mock::given(method("GET"))
            .and(path("/not-found"))
            .respond_with(CountResponder { count: call_count.clone() })
            .mount(&mock_server)
            .await;

        let fetcher = HttpFetcher::new(30);
        let result = fetcher.fetch(&format!("{}/not-found", mock_server.uri())).await;
        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
