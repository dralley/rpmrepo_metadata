use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{convert::TryInto, env};

use anyhow::Result;
use dialoguer::Confirm;
use rpm;
use rpmrepo::{
    metadata::{Package, RepositoryOptions},
    utils::copy_dir,
    Repository,
};

use super::CreateCommand;

pub fn create(config: CreateCommand) -> Result<()> {
    let temp_dir = std::env::temp_dir();
    let repository_path = env::current_dir()?.join(config.destination);

    let mut repo = Repository::new();

    if let Some(distro_tag) = config.distro_tag {
        let mut pieces = distro_tag.split(',');
        let name = pieces.next().unwrap().to_owned();
        let cpeid = pieces.next().map(|s| s.to_owned());
        repo.add_distro_tag(name, cpeid);
    }

    if let Some(content_tags) = config.content_tags {
        content_tags
            .split(',')
            .for_each(|t| repo.add_content_tag(t.to_owned()));
    }

    if let Some(repo_tags) = config.repo_tags {
        repo_tags
            .split(',')
            .for_each(|t| repo.add_repo_tag(t.to_owned()));
    }

    // if let Some(add_package_list) = config.add_package_list {
    //     let pkglist_path = Path::new(&add_package_list);
    //     let pkglist_file = File::open(&pkglist_path)?; // TODO pretty error handling
    //     for pkg_path in BufReader::new(pkglist_file).lines() {
    //         let rpm_file = File::open(pkg_path)?;
    //         let mut buf_reader = std::io::BufReader::new(rpm_file);
    //         let pkg = rpm::RPMPackage::parse(&mut buf_reader)?;
    //         let package: Package = pkg.into();
    //     }
    // }

    // TODO: enumerate RPMs, add to repo

    let mut options = RepositoryOptions::default();

    if let Some(compression_type) = config.metadata_compression_type {
        options = options.metadata_compression_type(compression_type.as_str().try_into()?);
    }

    // TODO: this is messy, also list valid compression options when user types invalid one

    // let options = if let Some(checksum_type) = config.metadata_compression_type {
    //     options.metadata_checksum_type(checksum_type.as_str().try_into()?)
    // } else {
    //     unreachable!()
    // };

    // repo.to_directory(&temp_dir, options)?;

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

    repo.write_to_directory(&repository_path, options)?;

    // copy_dir(&temp_dir, &repository_path)?;

    Ok(())
}
