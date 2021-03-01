use std::env;

use anyhow::Result;
use dialoguer::Confirm;
use url::Url;

use super::DownloadCommand;

use rpmrepo::download::{RepoDownloader, DEFAULT_CONCURRENCY};

pub fn download(config: DownloadCommand) -> Result<()> {
    let url = Url::parse(&config.url)?;
    let concurrency = config.concurrency.unwrap_or(DEFAULT_CONCURRENCY);

    let downloader = RepoDownloader::new(url).with_concurrency(concurrency);

    let repository_path = env::current_dir()?.join(config.destination);

    if repository_path.exists() {
        if Confirm::new()
            .with_prompt("A directory with this name already exists. Overwrite it?")
            .interact()?
        {
            std::fs::remove_dir_all(&repository_path)?;
        } else {
            std::process::exit(0);
        }
    }

    downloader.download_to(&repository_path)?;

    Ok(())
}
