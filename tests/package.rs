// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;

mod common;

pub const COMPLEX_PKG_PATH: &str = "./tests/assets/packages/complex-package-2.3.4-5.el8.x86_64.rpm";

#[test]
fn test_read_rpm_from_file() -> Result<(), MetadataError> {
    let mut pkg = Package::from_file_with_options(COMPLEX_PKG_PATH, Default::default())?;
    pkg.location_href = "complex-package-2.3.4-5.el8.x86_64.rpm".to_owned();
    assert_eq!(&pkg, &*common::COMPLEX_PACKAGE);

    Ok(())
}

#[test]
fn test_sort_packages_by_evr() {
    let mut packages: Vec<Package> = vec![
        ("foo", "0", "3.0", "1.el9", "x86_64"),
        ("foo", "0", "1.0", "1.el9", "x86_64"),
        ("foo", "1", "1.0", "1.el9", "x86_64"),
        ("foo", "0", "2.0", "1.el9", "x86_64"),
        ("foo", "0", "1.0", "2.el9", "x86_64"),
    ]
    .into_iter()
    .map(|(name, epoch, version, release, arch)| {
        let mut pkg = Package::default();
        pkg.set_name(name);
        pkg.set_evr(rpmrepo_metadata::Evr::new(
            epoch.to_owned(),
            version.to_owned(),
            release.to_owned(),
        ));
        pkg.set_arch(arch);
        pkg
    })
    .collect();

    packages.sort_by(|a, b| a.evr().cmp(b.evr()));

    let versions: Vec<&str> = packages.iter().map(|p| p.version()).collect();
    assert_eq!(versions, vec!["1.0", "1.0", "2.0", "3.0", "1.0"]);

    let releases: Vec<&str> = packages.iter().map(|p| p.release()).collect();
    assert_eq!(releases, vec!["1.el9", "2.el9", "1.el9", "1.el9", "1.el9"]);

    let epochs: Vec<u32> = packages.iter().map(|p| p.epoch()).collect();
    assert_eq!(epochs, vec![0, 0, 0, 0, 1]);
}

#[test]
fn test_sort_packages_by_nevra() {
    let mut packages: Vec<Package> = vec![
        ("zlib", "0", "1.0", "1.el9", "x86_64"),
        ("bash", "0", "5.0", "1.el9", "x86_64"),
        ("bash", "0", "4.0", "1.el9", "x86_64"),
        ("glibc", "0", "2.0", "1.el9", "i686"),
        ("glibc", "0", "2.0", "1.el9", "x86_64"),
    ]
    .into_iter()
    .map(|(name, epoch, version, release, arch)| {
        let mut pkg = Package::default();
        pkg.set_name(name);
        pkg.set_evr(rpmrepo_metadata::Evr::new(
            epoch.to_owned(),
            version.to_owned(),
            release.to_owned(),
        ));
        pkg.set_arch(arch);
        pkg
    })
    .collect();

    packages.sort_by(|a, b| a.nevra().cmp(&b.nevra()));

    let nevras: Vec<String> = packages.iter().map(|p| p.nvra()).collect();
    assert_eq!(
        nevras,
        vec![
            "bash-4.0-1.el9.x86_64",
            "bash-5.0-1.el9.x86_64",
            "glibc-2.0-1.el9.i686",
            "glibc-2.0-1.el9.x86_64",
            "zlib-1.0-1.el9.x86_64",
        ]
    );
}
