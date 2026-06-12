pub struct HeadlessFetcher;

impl HeadlessFetcher {
    pub fn new() -> anyhow::Result<Self> {
        anyhow::bail!("headless feature not enabled in this build")
    }

    pub async fn fetch(&self, _url: &str) -> anyhow::Result<String> {
        anyhow::bail!("headless feature not enabled in this build")
    }
}
