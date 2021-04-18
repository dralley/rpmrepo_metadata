use std::env;

use anyhow::Result;
use dialoguer::Confirm;
use url::Url;

use super::DownloadCommand;

use rpmrepo::download::{DownloadConfig, RepoDownloader};

pub fn download(config: DownloadCommand) -> Result<()> {
    let url = Url::parse(&config.url)?;

    let mut download_config = DownloadConfig::new();

    if let Some(concurrency) = config.concurrency {
        download_config = download_config.with_concurrency(concurrency);
    }

    if let Some(client_cert) = config.client_cert {
        let client_key = config.client_cert_key.unwrap_or(client_cert.clone());
        download_config = download_config.with_client_certificate(client_cert, client_key);
    }

    if let Some(ca_cert) = config.ca_cert {
        download_config = download_config.with_ca_cert(ca_cert);
    }

    download_config = download_config.verify_tls(!config.no_check_certificate);
    download_config = download_config.only_metadata(config.only_metadata);

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

    let repo_downloader = RepoDownloader::new(url, download_config);

    repo_downloader.download_to(&repository_path)?;

    Ok(())
}
