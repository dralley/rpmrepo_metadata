use std::env;
use std::fs;

use anyhow::Result;
use dialoguer::Confirm;
use tempdir::TempDir;
use url::Url;

use super::DownloadCommand;

use rpmrepo::download::{DownloadConfig, RepoDownloader};

pub fn download(config: DownloadCommand) -> Result<()> {
    let url = Url::parse(&config.url)?;

    let mut download_config = DownloadConfig::new();

    if let Some(concurrency) = config.concurrency {
        download_config = download_config.with_concurrency(concurrency);
    }

    if let Some(client_cert) = config.tls_client_cert {
        let client_key = config.tls_client_cert_key.unwrap_or(client_cert.clone());
        download_config = download_config.with_client_certificate(client_cert, client_key);
    }

    if let Some(ca_cert) = config.tls_ca_cert {
        download_config = download_config.with_ca_cert(ca_cert);
    }

    download_config = download_config.verify_tls(!config.no_check_certificate);
    download_config = download_config.only_metadata(config.only_metadata);

    let repo_destination = env::current_dir()?.join(config.destination);

    if repo_destination.exists() {
        let overwrite = Confirm::new()
            .with_prompt("A directory with this name already exists. Overwrite it?")
            .interact()?;

        if !overwrite {
            std::process::exit(0);
        }
    }

    let parent_dir = repo_destination.parent().unwrap();
    let cachedir = repo_destination;
    // let cachedir = TempDir::new_in(parent_dir, ".rpmrepo_cache_")?;

    RepoDownloader::new(url, download_config).download_to(&cachedir)?;

    // std::fs::remove_dir_all(&repo_destination)?;
    // fs::rename(cachedir.into_path(), &repo_destination)?;

    Ok(())
}
