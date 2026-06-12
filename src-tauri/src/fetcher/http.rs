use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct HttpFetcher {
    client: Client,
    user_agent: String,
}

impl Default for HttpFetcher {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpFetcher {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .expect("build reqwest client");
        Self {
            client,
            user_agent: String::from("Docurip/0.1.0 (+https://github.com/docurip)"),
        }
    }

    pub async fn fetch(&self, url: &str) -> anyhow::Result<String> {
        let resp = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {} for {}", status, url);
        }
        let text = resp.text().await?;
        Ok(text)
    }

    pub async fn fetch_with_status(&self, url: &str) -> anyhow::Result<(u16, String)> {
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

    pub async fn fetch_bytes(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self
            .client
            .get(url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {} for {}", status, url);
        }
        let bytes = resp.bytes().await?.to_vec();
        Ok(bytes)
    }
}
