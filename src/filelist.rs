// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::Checksum;

use super::metadata::{
    FileType, FilelistsXml, Package, PackageFile, RpmMetadata, XML_NS_FILELISTS,
};
use super::{EVR, MetadataError, Repository};

const TAG_FILELISTS: &str = "filelists";
const TAG_PACKAGE: &str = "package";
const TAG_VERSION: &str = "version";
const TAG_FILE: &str = "file";

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
            if package == None {
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
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> FilelistsXmlWriter<W> {
        FilelistsXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> FilelistsXmlReader<R> {
        FilelistsXmlReader { reader }
    }
}

pub struct FilelistsXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> FilelistsXmlWriter<W> {
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="210">
        let mut filelists_tag = BytesStart::new(TAG_FILELISTS);
        filelists_tag.push_attribute(("xmlns", XML_NS_FILELISTS));
        filelists_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(filelists_tag.borrow()))?;

        Ok(())
    }

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
        let (epoch, version, release) = package.evr().values();
        self.writer
            .create_element(TAG_VERSION)
            .with_attribute(("epoch", epoch))
            .with_attribute(("ver", version))
            .with_attribute(("rel", release))
            .write_empty()?;

        // <file type="dir">/etc/fonts/conf.avail</file>
        package
            .files()
            .iter()
            .try_for_each(|f| write_file_element(&mut self.writer, f))?;

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        Ok(())
    }

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

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

// <file type="dir">/etc/fonts/conf.avail</file>
pub(crate) fn write_file_element<W: Write>(
    writer: &mut Writer<W>,
    file: &PackageFile,
) -> Result<(), MetadataError> {
    let mut file_tag = BytesStart::new(TAG_FILE);
    if file.filetype != FileType::File {
        file_tag.push_attribute(("type".as_bytes(), file.filetype.to_values()));
    }
    writer.write_event(Event::Start(file_tag.borrow()))?;
    writer.write_event(Event::Text(BytesText::new(&file.path)))?;
    writer.write_event(Event::End(file_tag.to_end()))?;
    Ok(())
}

pub struct FilelistsXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> FilelistsXmlReader<R> {
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_header(&mut self.reader)
    }

    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        parse_package(package, &mut self.reader)
    }
}

// <?xml version="1.0" encoding="UTF-8"?>
// <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="35">
fn parse_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    let mut buf = Vec::new();

    // TODO: get rid of this buffer
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name().as_ref() == TAG_FILELISTS.as_bytes() => {
                let count = e.try_get_attribute("packages")?.unwrap().value;
                return Ok(std::str::from_utf8(&count)?.parse()?);
            }
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

//   <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.fc33"/>
//     <file type="dir">/etc/fonts/conf.avail</file>
//     ...
//     <file>/etc/fonts/conf.avail/10-autohint.conf</file>
//   </package>
pub fn parse_package<R: BufRead>(
    package: &mut Option<Package>,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(128);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE.as_bytes() => break,

            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGE => {
                    let pkgid = e
                        .try_get_attribute("pkgid")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("pkgid"))?
                        .unescape_value()?;
                    let name = e
                        .try_get_attribute("name")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                        .unescape_value()?;
                    let arch = e
                        .try_get_attribute("arch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
                        .unescape_value()?;

                    if let Some(pkg) = package {
                        assert!(pkg.pkgid() == pkgid.as_ref()); // TODO err instead of assert
                    } else {
                        let mut pkg = Package::default();
                        pkg.set_name(name)
                            .set_arch(arch)
                            .set_checksum(Checksum::Unknown(pkgid.into_owned()));
                        *package = Some(pkg);
                    };
                }
                TAG_VERSION => {
                    package.as_mut().unwrap().set_evr(parse_evr(reader, &e)?);
                }
                TAG_FILE => {
                    let file = parse_file(reader, &e)?;
                    // TODO: temporary PackageFile?
                    package
                        .as_mut()
                        .unwrap()
                        .add_file(file.filetype, &file.path);
                }
                _ => (),
            },
            Event::Eof => break,
            _ => (),
        }
    }

    // package.parse_state |= ParseState::FILELISTS;
    Ok(())
}

// <version epoch="0" ver="2.8.0" rel="5.fc33"/>
pub fn parse_evr<R: BufRead>(
    _reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<EVR, MetadataError> {
    let epoch = open_tag
        .try_get_attribute("epoch")?
        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
        .unescape_value()?;
    let version = open_tag
        .try_get_attribute("ver")?
        .ok_or_else(|| MetadataError::MissingAttributeError("ver"))?
        .unescape_value()?;
    let release = open_tag
        .try_get_attribute("rel")?
        .ok_or_else(|| MetadataError::MissingAttributeError("rel"))?
        .unescape_value()?;

    Ok(EVR::new(epoch, version, release))
}

// <file type="dir">/etc/fonts/conf.avail</file>
pub fn parse_file<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<PackageFile, MetadataError> {
    let mut file = PackageFile::default();
    let mut buf = Vec::with_capacity(128);
    file.path = reader
        .read_text_into(open_tag.name(), &mut buf)?
        .decode()?
        .into_owned();

    if let Some(filetype) = open_tag.try_get_attribute("type")? {
        file.filetype = FileType::try_create(filetype.value.as_ref())?;
    }

    Ok(file)
}
