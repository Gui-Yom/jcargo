use std::path::Path;

use anyhow::{anyhow, Result};
use reqwest::Client;
use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};
use url::Url;

pub async fn download_file(client: &Client, url: Url, path: impl AsRef<Path>) -> Result<()> {
    let mut res = client.get(url).send().await?;

    if res.status().is_success() {
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .await?;
        let mut buf_file = BufWriter::new(file);
        while let Some(chunk) = res.chunk().await? {
            buf_file.write(&chunk).await?;
        }
        buf_file.flush().await?;
        Ok(())
    } else {
        Err(anyhow!("Url is probably incorrect"))
    }
}
