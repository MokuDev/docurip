use crate::fetcher::http::HttpFetcher;
use crate::writer::fs::FsWriter;

pub struct AssetDownloader {
    fetcher: HttpFetcher,
    writer: FsWriter,
}

impl AssetDownloader {
    pub fn new(fetcher: HttpFetcher, writer: FsWriter) -> Self {
        Self { fetcher, writer }
    }

    pub async fn download(&self, url: &str) -> anyhow::Result<()> {
        let data = self.fetcher.fetch_bytes(url).await?;
        self.writer.write_asset(url, &data).await?;
        Ok(())
    }
}
