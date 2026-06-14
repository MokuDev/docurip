use crate::fetcher::http::HttpFetcher;
use crate::writer::fs::FsWriter;

#[derive(Clone)]
pub struct AssetDownloader {
    fetcher: HttpFetcher,
    writer: FsWriter,
}

impl AssetDownloader {
    pub fn new(fetcher: HttpFetcher, writer: FsWriter) -> Self {
        Self { fetcher, writer }
    }

    pub async fn download(&self, url: &str) -> anyhow::Result<String> {
        let data = self.fetcher.fetch_bytes(url).await?;
        self.writer.write_asset(url, &data).await
    }
}
