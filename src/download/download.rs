use std::fs::{self, create_dir, File};
use std::io::Write;
use std::path::Path;
use std::{io, io::Read, time::Instant};

use indicatif::ProgressBar;
use rayon::prelude::*;
use thiserror::Error;
use ureq;
use url::Url;

// use crate::metadata::RpmMetadata;
use crate::metadata::{self, MetadataError, PrimaryXml, RepomdXml, Repository};

pub const DEFAULT_CONCURRENCY: u8 = 5;

// pub struct DownloadTarget {
//     relative_path: Url,
//     checksum: Option<Checksum>,
// }

// enum DownloadState {
//     Waiting,
//     Running,
//     Finished,
//     Failed,
// }

#[derive(Error, Debug)]
pub enum RepoDownloadError {
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    DownloadError(#[from] ureq::Error),
    #[error(transparent)]
    MetadataError(#[from] MetadataError),
}

pub struct RepoDownloader {
    base_url: Url,
    concurrency: u8,
    only_metadata: bool,
}

impl RepoDownloader {
    pub fn new(url: Url) -> Self {
        RepoDownloader {
            base_url: url,
            concurrency: DEFAULT_CONCURRENCY,
            only_metadata: false,
        }
    }

    pub fn with_concurrency(self, threads: u8) -> Self {
        assert_eq!(threads.clamp(1, 10), threads, "Concurrency must be between 1 and 10");

        RepoDownloader {
            concurrency: threads,
            ..self
        }
    }

    pub fn only_metadata(self, val: bool) -> Self {
        RepoDownloader {
            only_metadata: val,
            ..self
        }
    }

    pub fn download_to(&self, repository_path: &Path) -> Result<(), RepoDownloadError> {
        let base_url = &self.base_url;

        let mut repo = Repository::new();

        let repomd_url = base_url.join("repodata/repomd.xml")?;
        let repomd_xml = &download_file(&repomd_url)?;
        repo.load_metadata_bytes::<RepomdXml>(repomd_xml)?;

        let repodata_path = repository_path.join("repodata");
        create_dir(&repository_path)?;
        create_dir(&repodata_path)?;

        let repomd_path = repodata_path.join("repomd.xml");
        save_metadata_file(&repomd_xml, &repomd_path)?;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.concurrency.into())
            .build()
            .unwrap();

        let begin = Instant::now();
        pool.scope(|_| {
            repo.metadata_files.par_iter().for_each(|md| {
                let relative_path = md.location_href.as_str();
                let url = base_url.join(relative_path).unwrap();

                let fs_location = &repository_path.join(relative_path);
                let metadata_bytes = download_file(&url).unwrap();
                save_metadata_file(&metadata_bytes, fs_location).unwrap();
                // verify_checksum(&fs_location, &md.checksum).unwrap();
            });
        });
        let end = Instant::now();

        println!(
            "Metadata downloaded in {} seconds",
            (end - begin).as_secs_f32()
        );

        if self.only_metadata {
            return Ok(());
        }

        let primary_href = repo.get_primary_data().location_href.as_str();
        let primary_path = repository_path.join(primary_href);
        repo.load_metadata_file::<PrimaryXml>(&primary_path)?;

        // let progress_bar = ProgressBar::new(packages.len() as u64);

        let begin = Instant::now();
        pool.scope(|_| {
            repo.packages().par_iter().for_each(|(_, package)| {
                let relative_path = package.location_href.as_str();
                let url = base_url.join(relative_path).unwrap();

                let fs_location = &repository_path.join(relative_path);
                let package_bytes = download_file(&url).unwrap();
                save_metadata_file(&package_bytes, fs_location).unwrap();
                // verify_checksum(&fs_location, &package.checksum).unwrap();
            });
        });
        let end = Instant::now();

        println!(
            "Packages downloaded in {} seconds",
            (end - begin).as_secs_f32()
        );

        Ok(())
    }
}

fn download_file(url: &Url) -> Result<Vec<u8>, RepoDownloadError> {
    let resp = ureq::get(url.as_str()).call()?;

    assert!(resp.has("Content-Length"));
    let len = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap();
    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    resp.into_reader().read_to_end(&mut bytes)?;

    assert_eq!(bytes.len(), len);

    Ok(bytes)
}

// fn verify_checksum(file: &Path, checksum: &Checksum) -> Result<(), RepoDownloadError> {
//     Ok(())
// }

fn save_metadata_file(bytes: &[u8], path: &Path) -> Result<(), RepoDownloadError> {
    let prefix = path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)
        .expect("Failed to write bytes to file");
    Ok(())
}
