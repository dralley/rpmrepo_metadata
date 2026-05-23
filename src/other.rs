// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::escape::partial_escape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::Checksum;
use crate::constants::{tag::*, xmlns};
use crate::metadata::{OtherXml, Package, RpmMetadata};
use crate::parsing_utils::{self, resolve_attr, resolve_text};
use crate::visitor::{ChangelogData, OtherVisitor};
use crate::{Evr, MetadataError, Repository};

impl RpmMetadata for OtherXml {
    fn filename() -> &'static str {
        "other.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = OtherXml::new_reader(reader);
        reader.read_header()?;
        let mut package = None;
        loop {
            reader.read_package(&mut package)?;
            if package.is_none() {
                break;
            }
            let pkgid = package.as_ref().unwrap().pkgid().to_owned();
            repository
                .packages_mut()
                .entry(pkgid)
                .and_modify(|p| {
                    std::mem::swap(
                        &mut p.rpm_changelogs,
                        &mut package.as_mut().unwrap().rpm_changelogs,
                    )
                })
                .or_insert(package.take().unwrap());
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = OtherXml::new_writer(writer);
        writer.write_header(repository.packages().len())?;
        for package in repository.packages().values() {
            writer.write_package(package)?;
        }
        writer.finish()
    }
}

impl OtherXml {
    /// Create a new other.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> OtherXmlWriter<W> {
        OtherXmlWriter { writer }
    }

    /// Create a new other.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> OtherXmlReader<R> {
        OtherXmlReader { reader }
    }
}

/// Streaming writer for other.xml metadata.
pub struct OtherXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> OtherXmlWriter<W> {
    /// Write the XML declaration and opening `<otherdata>` element.
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <otherdata xmlns="http://linux.duke.edu/metadata/other" packages="200">
        let mut other_tag = BytesStart::new(TAG_OTHER);
        other_tag.push_attribute(("xmlns", xmlns::NS_OTHER));
        other_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer.write_event(Event::Start(other_tag))?;

        Ok(())
    }

    /// Write a single `<package>` element with its changelog entries.
    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        // <package pkgid="6a915b6e1ad740994aa9688d70a67ff2b6b72e0ced668794aeb27b2d0f2e237b" name="fontconfig" arch="x86_64">
        let mut package_tag = BytesStart::new(TAG_PACKAGE);
        let (_, pkgid) = package.checksum().to_values()?;
        package_tag.push_attribute(("pkgid", pkgid));
        package_tag.push_attribute(("name", package.name()));
        package_tag.push_attribute(("arch", package.arch()));
        self.writer
            .write_event(Event::Start(package_tag.borrow()))?;

        let (epoch, version, release) = package.as_evr().values();
        // <version epoch="0" ver="2.8.0" rel="5.el6"/>
        self.writer
            .create_element(TAG_VERSION)
            .with_attribute(("epoch", epoch))
            .with_attribute(("ver", version))
            .with_attribute(("rel", release))
            .write_empty()?;

        for changelog in package.changelogs() {
            //  <changelog author="dalley &lt;dalley@redhat.com&gt; - 2.7.2-1" date="1251720000">- Update to 2.7.2</changelog>
            self.writer
                .create_element(TAG_CHANGELOG)
                .with_attribute(("author", changelog.author.as_str()))
                .with_attribute(("date", changelog.timestamp.to_string().as_str()))
                .write_text_content(BytesText::from_escaped(partial_escape(
                    &changelog.description,
                )))?;
        }

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        Ok(())
    }

    /// Write the closing `</otherdata>` element and flush.
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </otherdata>
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_OTHER)))?;

        // trailing newline
        self.writer.write_event(Event::Text(BytesText::new("\n")))?;

        // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
        self.writer.get_mut().flush()?;

        Ok(())
    }

    /// Consume the writer and return the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

/// Streaming reader for other.xml metadata.
pub struct OtherXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> OtherXmlReader<R> {
    /// Read and parse the XML header, returning the declared package count.
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_other_header(&mut self.reader)
    }

    /// Read changelog entries for the next package into `package`.
    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        let mut materializer = OtherMaterializer {
            package,
            error: None,
        };
        let found = parse_other_package(&mut self.reader, &mut materializer)?;
        if let Some(err) = materializer.error {
            return Err(err);
        }
        if !found {
            *materializer.package = None;
        }
        Ok(())
    }
}

/// Parse the other.xml header, returning the declared package count.
pub fn parse_other_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    parsing_utils::parse_header_tag(reader, TAG_OTHER)
}

/// Parse one `<package>` element from other.xml, dispatching to `visitor`.
///
/// Returns `true` if a package was parsed, `false` at EOF.
pub fn parse_other_package<R: BufRead, V: OtherVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<bool, MetadataError> {
    let mut buf = Vec::with_capacity(128);
    let mut text_buf = Vec::with_capacity(128);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE.as_bytes() => {
                visitor.end_package();
                return Ok(true);
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGE => {
                    let mut pkgid_cow = None;
                    let mut name_cow = None;
                    let mut arch_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"pkgid" => pkgid_cow = Some(resolve_attr(&attr)?),
                            b"name" => name_cow = Some(resolve_attr(&attr)?),
                            b"arch" => arch_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let pkgid = pkgid_cow.ok_or(MetadataError::MissingAttributeError("pkgid"))?;
                    let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                    let arch = arch_cow.ok_or(MetadataError::MissingAttributeError("arch"))?;
                    visitor.begin_package(&pkgid, &name, &arch);
                }
                TAG_VERSION => {
                    let (epoch, version, release) = parsing_utils::parse_evr_from_tag(&e)?;
                    visitor.set_evr(&epoch, &version, &release);
                }
                TAG_CHANGELOG => {
                    let mut author_cow = None;
                    let mut date_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"author" => author_cow = Some(resolve_attr(&attr)?),
                            b"date" => date_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let author =
                        author_cow.ok_or(MetadataError::MissingAttributeError("author"))?;
                    let date_val = date_cow.ok_or(MetadataError::MissingAttributeError("date"))?;
                    let timestamp: u64 = date_val.parse()?;
                    let bytes_text = reader.read_text_into(e.name(), &mut text_buf)?;
                    let description = resolve_text(&bytes_text)?;
                    visitor.add_changelog(ChangelogData {
                        author: &author,
                        description: &description,
                        timestamp,
                    });
                }
                _ => (),
            },
            Event::Eof => return Ok(false),
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

struct OtherMaterializer<'a> {
    package: &'a mut Option<Package>,
    error: Option<MetadataError>,
}

impl OtherVisitor for OtherMaterializer<'_> {
    fn begin_package(&mut self, pkgid: &str, name: &str, arch: &str) {
        if let Some(pkg) = self.package.as_mut() {
            if pkg.pkgid() != pkgid {
                self.error = Some(MetadataError::InconsistentMetadataError(format!(
                    "other.xml pkgid {} does not match primary.xml pkgid {}",
                    pkgid,
                    pkg.pkgid()
                )));
            }
        } else {
            let mut pkg = Package::default();
            pkg.set_name(name)
                .set_arch(arch)
                .set_checksum(Checksum::Unknown(pkgid.to_owned()));
            *self.package = Some(pkg);
        }
    }

    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_evr(Evr::new(
                epoch.to_owned(),
                version.to_owned(),
                release.to_owned(),
            ));
        }
    }

    fn add_changelog(&mut self, changelog: ChangelogData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.add_changelog(changelog.author, changelog.description, changelog.timestamp);
        }
    }
}
