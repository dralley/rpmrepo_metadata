// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Visitor-based streaming parsers for RPM repository metadata.
//!
//! These parsers call visitor methods with borrowed `&str` references,
//! allowing callers to intern strings directly without intermediate allocations.
//!
//! # Example
//!
//! ```ignore
//! use rpmrepo_metadata::visitor::{PrimaryVisitor, RequirementData, parse_primary};
//!
//! struct MyVisitor;
//! impl PrimaryVisitor for MyVisitor {
//!     fn begin_package(&mut self, name: &str, arch: &str, checksum_type: &str, pkgid: &str) {
//!         println!("package: {name}-{arch} ({checksum_type}:{pkgid})");
//!     }
//!     fn add_require(&mut self, req: RequirementData<'_>) {
//!         println!("  requires: {}", req.name);
//!     }
//! }
//!
//! let mut xml_reader = rpmrepo_metadata::utils::xml_reader_from_file(path).unwrap();
//! let num_pkgs = parse_primary(&mut xml_reader, &mut MyVisitor).unwrap();
//! ```

use std::io::BufRead;

use crate::MetadataError;
use crate::metadata::{Changelog, FileType, Requirement, RequirementType};
use crate::{comps, filelist, other, primary, updateinfo};
use quick_xml;

// ── Borrowed data types passed to visitors ──────────────────────────────

/// Borrowed requirement data, valid only during the visitor callback.
#[derive(Debug)]
pub struct RequirementData<'a> {
    pub name: &'a str,
    pub flags: Option<RequirementType>,
    pub epoch: Option<&'a str>,
    pub version: Option<&'a str>,
    pub release: Option<&'a str>,
    pub preinstall: bool,
}

/// Borrowed changelog data, valid only during the visitor callback.
#[derive(Debug)]
pub struct ChangelogData<'a> {
    pub author: &'a str,
    pub description: &'a str,
    pub timestamp: u64,
}

// ── Conversions from borrowed to owned types ──────────────────────────

impl From<RequirementData<'_>> for Requirement {
    fn from(data: RequirementData<'_>) -> Self {
        Requirement::new(data.name)
            .set_flags(data.flags)
            .set_epoch(data.epoch)
            .set_version(data.version)
            .set_release(data.release)
            .set_preinstall(data.preinstall)
    }
}

impl From<ChangelogData<'_>> for Changelog {
    fn from(data: ChangelogData<'_>) -> Self {
        Changelog {
            author: data.author.to_owned(),
            timestamp: data.timestamp,
            description: data.description.to_owned(),
        }
    }
}

// ── Visitor traits ──────────────────────────────────────────────────────

/// Visitor trait for streaming primary.xml parsing.
///
/// All `&str` arguments are borrowed from the parser's internal buffers
/// and are valid only for the duration of each method call.
///
/// Methods have default no-op implementations so visitors only need to
/// override the callbacks they care about.
#[allow(unused_variables)]
pub trait PrimaryVisitor {
    fn begin_package(&mut self, name: &str, arch: &str, checksum_type: &str, pkgid: &str) {}
    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {}
    fn set_summary(&mut self, summary: &str) {}
    fn set_description(&mut self, description: &str) {}
    fn set_packager(&mut self, packager: &str) {}
    fn set_url(&mut self, url: &str) {}
    fn set_time(&mut self, file: u64, build: u64) {}
    fn set_size(&mut self, package: u64, installed: u64, archive: u64) {}
    fn set_location(&mut self, href: &str, base: Option<&str>) {}
    fn set_rpm_license(&mut self, license: &str) {}
    fn set_rpm_vendor(&mut self, vendor: &str) {}
    fn set_rpm_group(&mut self, group: &str) {}
    fn set_rpm_buildhost(&mut self, buildhost: &str) {}
    fn set_rpm_sourcerpm(&mut self, sourcerpm: &str) {}
    fn set_rpm_header_range(&mut self, start: u64, end: u64) {}
    fn add_provide(&mut self, req: RequirementData<'_>) {}
    fn add_require(&mut self, req: RequirementData<'_>) {}
    fn add_conflict(&mut self, req: RequirementData<'_>) {}
    fn add_obsolete(&mut self, req: RequirementData<'_>) {}
    fn add_suggest(&mut self, req: RequirementData<'_>) {}
    fn add_enhance(&mut self, req: RequirementData<'_>) {}
    fn add_recommend(&mut self, req: RequirementData<'_>) {}
    fn add_supplement(&mut self, req: RequirementData<'_>) {}
    fn add_file(&mut self, filetype: FileType, path: &str) {}
    fn end_package(&mut self) {}
}

/// Visitor trait for streaming filelists.xml parsing.
#[allow(unused_variables)]
pub trait FilelistsVisitor {
    fn begin_package(&mut self, pkgid: &str, name: &str, arch: &str) {}
    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {}
    fn add_file(&mut self, filetype: FileType, path: &str) {}
    fn end_package(&mut self) {}
}

/// Visitor trait for streaming other.xml parsing.
#[allow(unused_variables)]
pub trait OtherVisitor {
    fn begin_package(&mut self, pkgid: &str, name: &str, arch: &str) {}
    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {}
    fn add_changelog(&mut self, changelog: ChangelogData<'_>) {}
    fn end_package(&mut self) {}
}

/// Visitor trait for streaming updateinfo.xml parsing.
#[allow(unused_variables)]
pub trait UpdateinfoVisitor {
    fn begin_update(&mut self, from: &str, update_type: &str, status: &str, version: &str) {}
    fn set_id(&mut self, id: &str) {}
    fn set_title(&mut self, title: &str) {}
    fn set_issued_date(&mut self, date: &str) {}
    fn set_updated_date(&mut self, date: &str) {}
    fn set_rights(&mut self, rights: &str) {}
    fn set_release(&mut self, release: &str) {}
    fn set_severity(&mut self, severity: &str) {}
    fn set_pushcount(&mut self, pushcount: &str) {}
    fn set_summary(&mut self, summary: &str) {}
    fn set_description(&mut self, description: &str) {}
    fn set_solution(&mut self, solution: &str) {}
    fn add_reference(&mut self, href: &str, id: Option<&str>, reftype: &str, title: &str) {}
    fn begin_collection(&mut self, shortname: &str) {}
    fn set_collection_name(&mut self, name: &str) {}
    fn set_collection_module(
        &mut self,
        name: &str,
        stream: &str,
        version: u64,
        context: &str,
        arch: &str,
    ) {
    }
    fn begin_collection_package(
        &mut self,
        name: &str,
        epoch: &str,
        version: &str,
        release: &str,
        arch: &str,
        src: Option<&str>,
    ) {
    }
    fn set_package_filename(&mut self, filename: &str) {}
    fn set_package_checksum(&mut self, checksum_type: &str, value: &str) {}
    fn set_package_reboot_suggested(&mut self) {}
    fn set_package_restart_suggested(&mut self) {}
    fn set_package_relogin_suggested(&mut self) {}
    fn end_collection_package(&mut self) {}
    fn end_collection(&mut self) {}
    fn end_update(&mut self) {}
}

/// Visitor trait for streaming comps.xml parsing.
#[allow(unused_variables)]
pub trait CompsVisitor {
    fn begin_group(&mut self) {}
    fn set_group_id(&mut self, id: &str) {}
    fn set_group_name(&mut self, name: &str, lang: Option<&str>) {}
    fn set_group_description(&mut self, desc: &str, lang: Option<&str>) {}
    fn set_group_default(&mut self, default: bool) {}
    fn set_group_uservisible(&mut self, visible: bool) {}
    fn set_group_biarchonly(&mut self, biarchonly: bool) {}
    fn set_group_langonly(&mut self, langonly: &str) {}
    fn set_group_display_order(&mut self, order: u32) {}
    fn add_group_package(
        &mut self,
        name: &str,
        reqtype: &str,
        requires: Option<&str>,
        basearchonly: bool,
    ) {
    }
    fn end_group(&mut self) {}

    fn begin_category(&mut self) {}
    fn set_category_id(&mut self, id: &str) {}
    fn set_category_name(&mut self, name: &str, lang: Option<&str>) {}
    fn set_category_description(&mut self, desc: &str, lang: Option<&str>) {}
    fn set_category_display_order(&mut self, order: u32) {}
    fn add_category_group_id(&mut self, group_id: &str) {}
    fn end_category(&mut self) {}

    fn begin_environment(&mut self) {}
    fn set_environment_id(&mut self, id: &str) {}
    fn set_environment_name(&mut self, name: &str, lang: Option<&str>) {}
    fn set_environment_description(&mut self, desc: &str, lang: Option<&str>) {}
    fn set_environment_display_order(&mut self, order: u32) {}
    fn add_environment_group_id(&mut self, group_id: &str) {}
    fn add_environment_option_id(&mut self, group_id: &str, default: bool) {}
    fn end_environment(&mut self) {}

    fn add_langpack(&mut self, name: &str, install: &str) {}
}

/// Parse primary.xml, dispatching to `visitor` for each package.
///
/// Returns the declared package count from the header.
pub fn parse_primary<R: BufRead, V: PrimaryVisitor>(
    reader: &mut quick_xml::Reader<R>,
    visitor: &mut V,
) -> Result<usize, MetadataError> {
    let num_packages = primary::parse_primary_header(reader)?;
    while primary::parse_primary_package(reader, visitor)? {}
    Ok(num_packages)
}

/// Parse filelists.xml, dispatching to `visitor` for each package.
///
/// Returns the declared package count from the header.
pub fn parse_filelists<R: BufRead, V: FilelistsVisitor>(
    reader: &mut quick_xml::Reader<R>,
    visitor: &mut V,
) -> Result<usize, MetadataError> {
    let num_packages = filelist::parse_filelists_header(reader)?;
    while filelist::parse_filelists_package(reader, visitor)? {}
    Ok(num_packages)
}

/// Parse other.xml, dispatching to `visitor` for each package.
///
/// Returns the declared package count from the header.
pub fn parse_other<R: BufRead, V: OtherVisitor>(
    reader: &mut quick_xml::Reader<R>,
    visitor: &mut V,
) -> Result<usize, MetadataError> {
    let num_packages = other::parse_other_header(reader)?;
    while other::parse_other_package(reader, visitor)? {}
    Ok(num_packages)
}

/// Parse updateinfo.xml, dispatching to `visitor` for each update record.
pub fn parse_updateinfo<R: BufRead, V: UpdateinfoVisitor>(
    reader: &mut quick_xml::Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    updateinfo::parse_updateinfo_header(reader)?;
    while updateinfo::parse_updateinfo_update(reader, visitor)? {}
    Ok(())
}

/// Parse comps.xml, dispatching to `visitor` for each element.
pub fn parse_comps<R: BufRead, V: CompsVisitor>(
    reader: &mut quick_xml::Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    while comps::parse_comps_item(reader, visitor)? {}
    Ok(())
}
