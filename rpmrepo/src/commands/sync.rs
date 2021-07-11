use std::{env, ffi::OsStr};

use anyhow::Result;
use dialoguer::Confirm;
use std::fs::File;
use std::path::Path;
use tini;
use url::Url;

use super::SyncCommand;

use rpmrepo::download::{DownloadConfig, RepoDownloadError, RepoDownloader};

struct RepoFile;

impl RepoFile {
    fn parse(path: &Path) -> Result<Vec<RepoDownloader>, Box<dyn std::error::Error>> {
        let repo_file = tini::Ini::from_file(path)?;

        let res = Vec::new();

        for (name, section) in repo_file.iter() {
            let mut config = DownloadConfig::new();

            for (key, value) in section {
                match key.as_str() {
                    "sslclientkey" => println!("sslclientkey => {}", value),
                    "sslclientcert" => println!("sslclientcert => {}", value),
                    "sslcacert" => config = config.with_ca_cert(value),
                    "sslverify" => config = config.verify_tls(value.parse()?),
                    "gpgcheck" => println!("gpgcheck => {}", value),
                    "repo_gpgcheck" => println!("repo_gpgcheck => {}", value),
                    _ => (),
                }
            }
        }

        Ok(res)
    }
}

// [fedora]
// name=Fedora $releasever - $basearch
// #baseurl=http://download.example/pub/fedora/linux/releases/$releasever/Everything/$basearch/os/
// metalink=https://mirrors.fedoraproject.org/metalink?repo=fedora-$releasever&arch=$basearch
// enabled=1
// countme=1
// metadata_expire=7d
// repo_gpgcheck=0
// type=rpm
// gpgcheck=1
// gpgkey=file:///etc/pki/rpm-gpg/RPM-GPG-KEY-fedora-$releasever-$basearch
// skip_if_unavailable=False

// [rhel-7-server-openstack-11-source-rpms]
// metadata_expire = 86400
// enabled_metadata = 0
// sslclientcert = /etc/pki/entitlement/7338795843348273596.pem
// baseurl = https://cdn.redhat.com/content/dist/rhel/server/7/7Server/$basearch/openstack/11/source/SRPMS
// ui_repoid_vars = basearch
// sslverify = 1
// name = Red Hat OpenStack Platform 11 for RHEL 7 (Source RPMs)
// sslclientkey = /etc/pki/entitlement/7338795843348273596-key.pem
// gpgkey = file:///etc/pki/rpm-gpg/RPM-GPG-KEY-redhat-release
// enabled = 0
// sslcacert = /etc/rhsm/ca/redhat-uep.pem
// gpgcheck = 1

pub fn sync(config: SyncCommand) -> Result<()> {
    let repo_file_dir = config.reposdir.unwrap_or("/etc/yum.repos.d/".into());

    for file in Path::new(&repo_file_dir).read_dir()? {
        let file = file?;
        if file.path().extension() == Some("repo".as_ref()) {
            RepoFile::parse(&file.path());
        }
    }

    // sslverify
    // sslcacert

    // download_config = download_config.verify_tls(!config.no_check_certificate);

    // let url = Url::parse(&config.url)?;

    // let mut download_config = DownloadConfig::new();

    // if let Some(concurrency) = config.concurrency {
    //     download_config = download_config.with_concurrency(concurrency);
    // }

    // let repo_downloader = RepoDownloader::new(url, download_config);

    // repo_downloader.download_to(&repository_path)?;

    Ok(())
}
