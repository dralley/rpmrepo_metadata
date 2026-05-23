// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::constants::{tag::*, xmlns};
use crate::parsing_utils::{self, resolve_attr, resolve_text};
use crate::visitor::FilelistsVisitor;
use crate::{Checksum, Evr};

use super::Repository;
use super::metadata::{FileType, FilelistsXml, MetadataError, Package, RpmMetadata};

impl RpmMetadata for FilelistsXml {
    fn filename() -> &'static str {
        "filelists.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = FilelistsXml::new_reader(reader);
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
                    std::mem::swap(&mut p.rpm_files, &mut package.as_mut().unwrap().rpm_files)
                })
                .or_insert(package.take().unwrap());
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = Self::new_writer(writer);
        writer.write_header(repository.packages().len())?;
        for package in repository.packages().values() {
            writer.write_package(package)?;
        }
        writer.finish()
    }
}

impl FilelistsXml {
    /// Create a new filelists.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> FilelistsXmlWriter<W> {
        FilelistsXmlWriter { writer }
    }

    /// Create a new filelists.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> FilelistsXmlReader<R> {
        FilelistsXmlReader { reader }
    }
}

/// Streaming writer for filelists.xml metadata.
pub struct FilelistsXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> FilelistsXmlWriter<W> {
    /// Write the XML declaration and opening `<filelists>` element.
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="210">
        let mut filelists_tag = BytesStart::new(TAG_FILELISTS);
        filelists_tag.push_attribute(("xmlns", xmlns::NS_FILELISTS));
        filelists_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(filelists_tag.borrow()))?;

        Ok(())
    }

    /// Write a single `<package>` element with its file entries.
    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        // <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
        let mut package_tag = BytesStart::new(TAG_PACKAGE);
        let pkgid = package.pkgid();
        package_tag.push_attribute(("pkgid", pkgid));
        package_tag.push_attribute(("name", package.name()));
        package_tag.push_attribute(("arch", package.arch()));
        self.writer
            .write_event(Event::Start(package_tag.borrow()))?;

        // <version epoch="0" ver="2.8.0" rel="5.el6"/>
        let (epoch, version, release) = package.as_evr().values();
        self.writer
            .create_element(TAG_VERSION)
            .with_attribute(("epoch", epoch))
            .with_attribute(("ver", version))
            .with_attribute(("rel", release))
            .write_empty()?;

        // <file type="dir">/etc/fonts/conf.avail</file>
        let writer = &mut self.writer;
        let mut err: Result<(), MetadataError> = Ok(());
        package.files().for_each_file(|filetype, path| {
            if err.is_ok()
                && let Err(e) = write_file_entry(writer, filetype, path)
            {
                err = Err(e);
            }
        });
        err?;

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        Ok(())
    }

    /// Write the closing `</filelists>` element and flush.
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </filelists>
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_FILELISTS)))?;

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

// <file type="dir">/etc/fonts/conf.avail</file>
pub(crate) fn write_file_entry<W: Write>(
    writer: &mut Writer<W>,
    filetype: FileType,
    path: &str,
) -> Result<(), MetadataError> {
    let mut file_tag = BytesStart::new(TAG_FILE);
    if filetype != FileType::File {
        file_tag.push_attribute(("type".as_bytes(), filetype.to_values()));
    }
    writer.write_event(Event::Start(file_tag.borrow()))?;
    writer.write_event(Event::Text(BytesText::new(path)))?;
    writer.write_event(Event::End(file_tag.to_end()))?;
    Ok(())
}

/// Streaming reader for filelists.xml metadata.
pub struct FilelistsXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> FilelistsXmlReader<R> {
    /// Read and parse the XML header, returning the declared package count.
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_filelists_header(&mut self.reader)
    }

    /// Read file entries for the next package into `package`.
    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        let mut materializer = FilelistsMaterializer {
            package,
            error: None,
        };
        let found = parse_filelists_package(&mut self.reader, &mut materializer)?;
        if let Some(err) = materializer.error {
            return Err(err);
        }
        if !found {
            *materializer.package = None;
        }
        Ok(())
    }
}

/// Parse the filelists.xml header, returning the declared package count.
pub fn parse_filelists_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    parsing_utils::parse_header_tag(reader, TAG_FILELISTS)
}

/// Parse one `<package>` element from filelists.xml, dispatching to `visitor`.
///
/// Returns `true` if a package was parsed, `false` at EOF.
pub fn parse_filelists_package<R: BufRead, V: FilelistsVisitor>(
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
                TAG_FILE => {
                    let filetype = if let Some(attr) = e.try_get_attribute("type")? {
                        FileType::try_create(attr.value.as_ref())?
                    } else {
                        FileType::File
                    };
                    let bytes_text = reader.read_text_into(e.name(), &mut text_buf)?;
                    let path = resolve_text(&bytes_text)?;
                    visitor.add_file(filetype, &path);
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

struct FilelistsMaterializer<'a> {
    package: &'a mut Option<Package>,
    error: Option<MetadataError>,
}

impl FilelistsVisitor for FilelistsMaterializer<'_> {
    fn begin_package(&mut self, pkgid: &str, name: &str, arch: &str) {
        if let Some(pkg) = self.package.as_mut() {
            if pkg.pkgid() != pkgid {
                self.error = Some(MetadataError::InconsistentMetadataError(format!(
                    "filelists.xml pkgid {} does not match primary.xml pkgid {}",
                    pkgid,
                    pkg.pkgid()
                )));
                return;
            }
            pkg.rpm_files.clear();
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

    fn add_file(&mut self, filetype: FileType, path: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.add_file(filetype, path);
        }
    }
}
