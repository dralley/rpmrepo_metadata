// Copyright (c) 2021 Daniel Alley
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::fs::{self, File};
use std::io::Write;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use dialoguer::Confirm;
use indicatif::{HumanBytes, ParallelProgressIterator, ProgressBar, ProgressFinish, ProgressStyle};
use io::BufReader;
use rayon::prelude::*;
use ring::digest;
use rustls::{
    self,
    internal::pemfile::{certs, rsa_private_keys},
};
use rustls_native_certs;
use thiserror::Error;
use ureq::{self, AgentBuilder};
use url::Url;

// use crate::metadata::RpmMetadata;
use crate::metadata::{Checksum, MetadataError, PrimaryXml, RepomdXml, Repository};

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
    download_config: DownloadConfig,
    base_url: Url,
    agent: ureq::Agent,
}

pub struct DownloadConfig {
    concurrency: u8,
    verify_tls: bool,
    only_metadata: bool,
    client_cert_path: Option<PathBuf>,
    client_key_path: Option<PathBuf>,
    ca_cert_path: Option<PathBuf>,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        DownloadConfig {
            concurrency: DEFAULT_CONCURRENCY,
            verify_tls: true,
            only_metadata: false,
            client_cert_path: None,
            client_key_path: None,
            ca_cert_path: None,
        }
    }
}

impl DownloadConfig {
    pub fn new() -> Self {
        Self::default()
    }

    fn build_agent(&self) -> ureq::Agent {
        let mut cfg = rustls::ClientConfig::default();
        cfg.root_store =
            rustls_native_certs::load_native_certs().expect("could not load platform certs");

        if let (Some(client_cert_path), Some(client_key_path)) = (
            self.client_cert_path.as_ref(),
            self.client_key_path.as_ref(),
        ) {
            let cert_file = &mut BufReader::new(File::open(client_cert_path).unwrap());
            let key_file = &mut BufReader::new(File::open(client_key_path).unwrap());

            let cert_chain = certs(cert_file).unwrap();
            let mut keys = rsa_private_keys(key_file).unwrap();

            cfg.set_single_client_cert(cert_chain, keys.pop().unwrap())
                .unwrap();
        }

        if let Some(ca_cert_path) = self.ca_cert_path.as_ref() {
            cfg.root_store
                .add_pem_file(&mut BufReader::new(File::open(ca_cert_path).unwrap()))
                .unwrap();
        }

        let default_redhat_path = Path::new("/etc/rhsm/ca/redhat-uep.pem");

        if default_redhat_path.exists() {
            cfg.root_store
                .add_pem_file(&mut BufReader::new(
                    File::open(default_redhat_path).unwrap(),
                ))
                .unwrap();
        }

        if !self.verify_tls {
            // TODO: rustls makes disabling verification a total pain in the ass (for good reason, but still...)
            unimplemented!();
        }

        ureq::AgentBuilder::new()
            .user_agent(concat!("rpmrepo_rs/", env!("CARGO_PKG_VERSION")))
            .tls_config(Arc::new(cfg))
            .build()
    }

    pub fn with_client_certificate<P: AsRef<Path>>(
        mut self,
        client_cert_path: P,
        client_key_path: P,
    ) -> Self {
        self.client_cert_path = Some(client_cert_path.as_ref().to_owned());
        self.client_key_path = Some(client_key_path.as_ref().to_owned());

        self
    }

    pub fn with_ca_cert<P: AsRef<Path>>(mut self, ca_cert_path: P) -> Self {
        self.ca_cert_path = Some(ca_cert_path.as_ref().into());

        self
    }

    pub fn only_metadata(self, val: bool) -> Self {
        DownloadConfig {
            only_metadata: val,
            ..self
        }
    }

    pub fn with_concurrency(self, threads: u8) -> Self {
        assert_eq!(
            threads.clamp(1, 10),
            threads,
            "Concurrency must be between 1 and 10"
        );

        DownloadConfig {
            concurrency: threads,
            ..self
        }
    }

    pub fn verify_tls(self, val: bool) -> Self {
        DownloadConfig {
            verify_tls: val,
            ..self
        }
    }
}

impl RepoDownloader {
    pub fn new(url: Url, config: DownloadConfig) -> Self {
        let agent = config.build_agent();
        RepoDownloader {
            download_config: config,
            base_url: url,
            agent: agent,
        }
    }

    // pub fn with_metadata_cb(self, f: FnMut()) {

    // }

    // pub fn with_package_cb(self, f: FnMut()) {

    // }

    pub fn download_to<P: AsRef<Path>>(&self, repository_path: P) -> Result<(), RepoDownloadError> {
        let base_url = &self.base_url;
        let repository_path = repository_path.as_ref();

        let mut repo = Repository::new();

        let repomd_url = base_url.join("repodata/repomd.xml")?;
        let repomd_xml = &download_file(&self.agent, &repomd_url)?;
        repo.load_metadata_bytes::<RepomdXml>(repomd_xml)?;

        let repodata_path = repository_path.join("repodata");
        fs::create_dir_all(&repodata_path)?;

        let repomd_path = repodata_path.join("repomd.xml");
        save_metadata_file(&repomd_xml, &repomd_path)?;

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.download_config.concurrency.into())
            .build()
            .unwrap();

        let begin = Instant::now();
        pool.scope(|_| {
            repo.metadata_files.par_iter().for_each(|md| {
                let relative_path = md.location_href.as_str();
                let url = base_url.join(relative_path).unwrap();

                let fs_location = &repository_path.join(relative_path);
                let metadata_bytes = download_file(&self.agent, &url).unwrap();
                save_metadata_file(&metadata_bytes, fs_location).unwrap();
                verify_checksum(&fs_location, &md.checksum).unwrap();
            });
        });
        let end = Instant::now();

        println!(
            "Metadata downloaded in {} seconds",
            (end - begin).as_secs_f32()
        );

        if self.download_config.only_metadata {
            return Ok(());
        }

        let primary_href = repo.get_primary_data().location_href.as_str();
        let primary_path = repository_path.join(primary_href);
        repo.load_metadata_file::<PrimaryXml>(&primary_path)?;

        // let mut package_pb =
        //     ProgressBar::new(repo.packages().len() as u64).with_style(ProgressStyle::default_bar());

        let begin = Instant::now();
        pool.scope(|_| {
            repo.packages()
                .par_iter()
                // .progress_with(package_pb)
                .for_each(|(_, package)| {
                    let relative_path = package.location_href();
                    let url = base_url.join(relative_path).unwrap();

                    let fs_location = &repository_path.join(relative_path);
                    let package_bytes = download_file(&self.agent, &url).unwrap();
                    save_metadata_file(&package_bytes, fs_location).unwrap();
                    assert!(verify_checksum(&fs_location, package.checksum()).unwrap());
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

fn download_file(agent: &ureq::Agent, url: &Url) -> Result<Vec<u8>, RepoDownloadError> {
    let resp = agent.get(url.as_str()).call()?;

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

fn verify_checksum(path: &Path, checksum: &Checksum) -> Result<bool, RepoDownloadError> {
    let reader = &mut BufReader::new(File::open(path).unwrap());

    let (expected_checksum, mut context) = match checksum {
        Checksum::Sha1(chk) => (chk, digest::Context::new(&digest::SHA1_FOR_LEGACY_USE_ONLY)),
        Checksum::Sha256(chk) => (chk, digest::Context::new(&digest::SHA256)),
        Checksum::Sha384(chk) => (chk, digest::Context::new(&digest::SHA384)),
        Checksum::Sha512(chk) => (chk, digest::Context::new(&digest::SHA512)),
        Checksum::Unknown => unreachable!(),
    };
    let mut buffer = [0; 4096];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }
    let actual_checksum = context.finish();
    Ok(actual_checksum.as_ref() == decode_hex(expected_checksum).unwrap())
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

fn save_metadata_file(bytes: &[u8], path: &Path) -> Result<(), RepoDownloadError> {
    let prefix = path.parent().unwrap();
    std::fs::create_dir_all(prefix)?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)
        .expect("Failed to write bytes to file");
    Ok(())
}
