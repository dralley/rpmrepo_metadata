// pub mod commands;

use anyhow::Result;
// use commands::handle_command;

// fn main() -> Result<()> {
//     handle_command()
// }

use rpmrepo_metadata::*;
use std::path::Path;
use std::io::{self, Write};

fn main() -> Result<()> {
    let repo_path = "./rpmrepo_metadata/tests/assets/external_repos/fedora35-updates/";
    let repo_path = "/home/dalley/devel/repos/rhel7/";

    let reader = rpmrepo_metadata::RepositoryReader::new_from_directory(Path::new(repo_path))?;
    let mut package_iter = reader.iter_packages()?;

    println!("Total packages: {}", package_iter.total_packages());

    for package in package_iter.map(|r| r.unwrap()) {

    }

    Ok(())
}
