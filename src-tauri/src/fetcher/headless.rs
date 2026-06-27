#[cfg(feature = "headless")]
mod inner {
    use headless_chrome::{Browser, LaunchOptions};

    pub struct HeadlessFetcher {
        browser: Browser,
    }

    impl HeadlessFetcher {
        pub fn new() -> anyhow::Result<Self> {
            let browser = Browser::new(LaunchOptions::default())?;
            Ok(Self { browser })
        }

        pub async fn fetch(&self, url: &str) -> anyhow::Result<String> {
            let tab = self.browser.new_tab()?;
            let url = url.to_string();
            let html = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
                tab.navigate_to(&url)?;
                tab.wait_until_navigated()?;
                let content = tab.get_content()?;
                let _ = tab.close(false);
                Ok(content)
            })
            .await??;
            Ok(html)
        }

    }
}

#[cfg(not(feature = "headless"))]
mod inner {
    pub struct HeadlessFetcher;

    impl HeadlessFetcher {
        pub fn new() -> anyhow::Result<Self> {
            anyhow::bail!("headless feature not enabled in this build")
        }

        pub async fn fetch(&self, _url: &str) -> anyhow::Result<String> {
            anyhow::bail!("headless feature not enabled in this build")
        }
    }
}

pub use inner::HeadlessFetcher;
