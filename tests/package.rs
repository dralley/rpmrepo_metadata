// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use std::io::{BufReader, Cursor};

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;

mod common;

pub const COMPLEX_PKG_PATH: &str = "./tests/assets/packages/complex-package-2.3.4-5.el8.x86_64.rpm";

#[test]
fn test_read_rpm_from_file() -> Result<(), MetadataError> {
    let mut pkg = Package::from_file_with_options(COMPLEX_PKG_PATH, Default::default())?;
    pkg.location_href = "complex-package-2.3.4-5.el8.x86_64.rpm".to_owned();
    // time_file is the RPM file's mtime on disk, which varies after git checkout
    pkg.set_time_file(common::COMPLEX_PACKAGE.time_file());
    assert_eq!(&pkg, &*common::COMPLEX_PACKAGE);

    Ok(())
}

fn package_source() -> PackageSource {
    PackageSource {
        checksum: Some(common::COMPLEX_PACKAGE.checksum().clone()),
        size_package: Some(common::COMPLEX_PACKAGE.size_package()),
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: common::COMPLEX_PACKAGE.location_base().map(str::to_owned),
    }
}

/// Converts metadata already parsed by rpm-rs without reparsing its headers.
#[test]
fn test_package_from_rpm_metadata() -> Result<(), MetadataError> {
    let bytes = std::fs::read(COMPLEX_PKG_PATH)?;
    let rpm_metadata = rpm::PackageMetadata::parse(&mut BufReader::new(Cursor::new(&bytes)))?;

    let package =
        Package::from_package_metadata(&rpm_metadata, package_source(), Default::default())?;
    assert_eq!(&package, &*common::COMPLEX_PACKAGE);

    Ok(())
}

/// Parses RPM headers from a buffer using caller-provided file metadata.
#[test]
fn test_package_from_buffer() -> Result<(), MetadataError> {
    let bytes = std::fs::read(COMPLEX_PKG_PATH)?;
    let rpm_metadata = rpm::PackageMetadata::parse(&mut BufReader::new(Cursor::new(&bytes)))?;
    let header_end = rpm_metadata.get_package_segment_offsets().payload as usize;
    let package = Package::from_buffer(&bytes[..header_end], package_source(), Default::default())?;
    assert_eq!(&package, &*common::COMPLEX_PACKAGE);

    Ok(())
}

/// Derives checksum and size when parsing a complete RPM buffer.
#[test]
fn test_package_from_buffer_derives_file_values() -> Result<(), MetadataError> {
    let bytes = std::fs::read(COMPLEX_PKG_PATH)?;
    let source = PackageSource {
        checksum: None,
        size_package: None,
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: common::COMPLEX_PACKAGE.location_base().map(str::to_owned),
    };

    let package = Package::from_buffer(&bytes, source, Default::default())?;
    assert_eq!(&package, &*common::COMPLEX_PACKAGE);

    Ok(())
}

/// Rejects a checksum without its corresponding full-file size.
#[test]
fn test_package_from_buffer_requires_file_values_together() {
    let bytes = std::fs::read(COMPLEX_PKG_PATH).unwrap();
    let source = PackageSource {
        checksum: Some(common::COMPLEX_PACKAGE.checksum().clone()),
        size_package: None,
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: None,
    };

    let error = Package::from_buffer(&bytes, source, Default::default()).unwrap_err();
    assert!(matches!(error, MetadataError::InconsistentMetadataError(_)));
}

/// Rejects a full-file size without its corresponding checksum.
#[test]
fn test_package_from_buffer_requires_checksum_with_size() {
    let bytes = std::fs::read(COMPLEX_PKG_PATH).unwrap();
    let source = PackageSource {
        checksum: None,
        size_package: Some(common::COMPLEX_PACKAGE.size_package()),
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: None,
    };

    let error = Package::from_buffer(&bytes, source, Default::default()).unwrap_err();
    assert!(matches!(error, MetadataError::InconsistentMetadataError(_)));
}

/// Requires full-file values when converting previously parsed RPM metadata.
#[test]
fn test_package_from_rpm_metadata_requires_file_values() -> Result<(), MetadataError> {
    let bytes = std::fs::read(COMPLEX_PKG_PATH)?;
    let rpm_metadata = rpm::PackageMetadata::parse(&mut BufReader::new(Cursor::new(&bytes)))?;
    let source = PackageSource {
        checksum: None,
        size_package: None,
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: None,
    };

    let error =
        Package::from_package_metadata(&rpm_metadata, source, Default::default()).unwrap_err();
    assert!(matches!(error, MetadataError::MissingFieldError(_)));
    Ok(())
}

/// Uses the configured checksum type when deriving values from a complete RPM buffer.
#[test]
fn test_package_from_buffer_uses_configured_checksum_type() -> Result<(), MetadataError> {
    let bytes = std::fs::read(COMPLEX_PKG_PATH)?;
    let source = PackageSource {
        checksum: None,
        size_package: None,
        time_file: common::COMPLEX_PACKAGE.time_file(),
        location_href: common::COMPLEX_PACKAGE.location_href().to_owned(),
        location_base: None,
    };
    let options = PackageOptions {
        checksum_type: ChecksumType::Sha512,
        ..Default::default()
    };

    let package = Package::from_buffer(&bytes, source, options)?;
    assert!(matches!(package.checksum(), Checksum::Sha512(_)));
    Ok(())
}

/// Keeps RPM header directory and basename components intact in repository metadata.
#[test]
fn test_file_path_components_are_preserved() {
    let mut pkg = Package::default();
    pkg.add_file_split(FileType::File, "/usr/libexec/example/", "tool");

    let file = pkg.files().iter().next().unwrap();
    assert_eq!(file.dir(), "/usr/libexec/example/");
    assert_eq!(file.basename(), "tool");
    assert_eq!(file.path(), "/usr/libexec/example/tool");
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

    packages.sort_by(|a, b| a.as_evr().cmp(b.as_evr()));

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
