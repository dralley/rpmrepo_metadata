// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};

use super::filelist;
use super::metadata::{
    Checksum, MetadataError, Package, PrimaryXml, Requirement, RpmMetadata, XML_NS_COMMON,
    XML_NS_RPM,
};
use super::{EVR, Repository};

const TAG_METADATA: &str = "metadata";
const TAG_PACKAGE: &str = "package";
const TAG_NAME: &str = "name";
const TAG_VERSION: &str = "version";
const TAG_CHECKSUM: &str = "checksum";
const TAG_ARCH: &str = "arch";
const TAG_SUMMARY: &str = "summary";
const TAG_DESCRIPTION: &str = "description";
const TAG_PACKAGER: &str = "packager";
const TAG_URL: &str = "url";
const TAG_TIME: &str = "time";
const TAG_SIZE: &str = "size";
const TAG_LOCATION: &str = "location";
const TAG_FORMAT: &str = "format";

const TAG_RPM_LICENSE: &str = "rpm:license";
const TAG_RPM_VENDOR: &str = "rpm:vendor";
const TAG_RPM_GROUP: &str = "rpm:group";
const TAG_RPM_BUILDHOST: &str = "rpm:buildhost";
const TAG_RPM_SOURCERPM: &str = "rpm:sourcerpm";
const TAG_RPM_HEADER_RANGE: &str = "rpm:header-range";

const TAG_RPM_ENTRY: &str = "rpm:entry";
const TAG_RPM_PROVIDES: &str = "rpm:provides";
const TAG_RPM_REQUIRES: &str = "rpm:requires";
const TAG_RPM_CONFLICTS: &str = "rpm:conflicts";
const TAG_RPM_OBSOLETES: &str = "rpm:obsoletes";
const TAG_RPM_SUGGESTS: &str = "rpm:suggests";
const TAG_RPM_ENHANCES: &str = "rpm:enhances";
const TAG_RPM_RECOMMENDS: &str = "rpm:recommends";
const TAG_RPM_SUPPLEMENTS: &str = "rpm:supplements";
const TAG_FILE: &str = "file";

impl RpmMetadata for PrimaryXml {
    fn filename() -> &'static str {
        "primary.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        // TODO: in theory, other or filelists could be parsed first, and in that case this is wrong
        let mut reader = PrimaryXml::new_reader(reader);
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
                .insert(pkgid, package.take().unwrap());
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = PrimaryXml::new_writer(writer);
        writer.write_header(repository.packages().len())?;
        for package in repository.packages().values() {
            writer.write_package(package)?;
        }
        writer.finish()
    }
}

impl PrimaryXml {
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> PrimaryXmlWriter<W> {
        PrimaryXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> PrimaryXmlReader<R> {
        PrimaryXmlReader { reader }
    }
}

pub struct PrimaryXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> PrimaryXmlReader<R> {
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_header(&mut self.reader)
    }

    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        parse_package(&mut self.reader, package)
    }
}

// <?xml version="1.0" encoding="UTF-8"?>
// <metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="35">
fn parse_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    let mut buf = Vec::new();

    // TODO: get rid of this buffer
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name().as_ref() == TAG_METADATA.as_bytes() => {
                let count = e.try_get_attribute("packages")?.unwrap().value;
                return Ok(std::str::from_utf8(&count)?.parse()?);
            }
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

pub fn parse_package<R: BufRead>(
    reader: &mut Reader<R>,
    package: &mut Option<Package>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(512);
    let mut text_buf = Vec::with_capacity(512);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGE => {
                    let ptype = e.try_get_attribute(b"type")?.unwrap().unescape_value()?;

                    assert_eq!(ptype.as_ref(), "rpm"); // TODO: better error handling

                    if let Some(_pkg) = package {
                        // TODO: need a temporary place to store this since we don't know the pkgid yet
                        unimplemented!("package must be parsed from primary.xml first");
                        // assert!(pkg.pkgid() == &pkgid); // TODO err instead of assert
                    } else {
                        let pkg = Package::default();
                        *package = Some(pkg);
                    };
                }
                TAG_NAME => {
                    let text = reader
                        .read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_name(text);
                }
                TAG_VERSION => {
                    let epoch = e
                        .try_get_attribute("epoch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
                        .unescape_value()?;

                    let version = e
                        .try_get_attribute("ver")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("ver"))?
                        .unescape_value()?;

                    let release = e
                        .try_get_attribute("rel")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("rel"))?
                        .unescape_value()?;

                    // TODO: temporary conversions
                    let evr = EVR::new(epoch, version, release);
                    package.as_mut().unwrap().set_evr(evr);
                }
                TAG_CHECKSUM => {
                    let checksum_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                        .unescape_value()?;
                    let checksum_value = reader
                        .read_text_into(QName(TAG_CHECKSUM.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    package.as_mut().unwrap().set_checksum(Checksum::try_create(
                        checksum_type.as_bytes(),
                        checksum_value.as_bytes(),
                    )?);
                }
                TAG_ARCH => {
                    let text = reader
                        .read_text_into(QName(TAG_ARCH.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_arch(text);
                }
                TAG_SUMMARY => {
                    let text = reader
                        .read_text_into(QName(TAG_SUMMARY.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_summary(text);
                }
                TAG_DESCRIPTION => {
                    let text = reader
                        .read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_description(text);
                }
                TAG_PACKAGER => {
                    let text = reader
                        .read_text_into(QName(TAG_PACKAGER.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_packager(text);
                }
                TAG_URL => {
                    let text = reader
                        .read_text_into(QName(TAG_URL.as_bytes()), &mut text_buf)?
                        .decode()?;
                    package.as_mut().unwrap().set_url(text);
                }
                TAG_TIME => {
                    let time_file = e
                        .try_get_attribute("file")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("file"))?
                        .unescape_value()?
                        .parse()?;

                    let time_build = e
                        .try_get_attribute("build")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("build"))?
                        .unescape_value()?
                        .parse()?;

                    package
                        .as_mut()
                        .unwrap()
                        .set_time_file(time_file)
                        .set_time_build(time_build);
                }
                TAG_SIZE => {
                    let package_size = e
                        .try_get_attribute("package")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("package"))?
                        .unescape_value()?
                        .parse()?;

                    let installed_size = e
                        .try_get_attribute("installed")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("installed"))?
                        .unescape_value()?
                        .parse()?;

                    let archive_size = e
                        .try_get_attribute("archive")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("archive"))?
                        .unescape_value()?
                        .parse()?;

                    package
                        .as_mut()
                        .unwrap()
                        .set_size_package(package_size)
                        .set_size_installed(installed_size)
                        .set_size_archive(archive_size);
                }
                TAG_LOCATION => {
                    let location_href = e
                        .try_get_attribute("href")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("href"))?
                        .unescape_value()?;

                    if let Some(base_attr) = e.try_get_attribute("base")? {
                        let location_base = base_attr.unescape_value()?;
                        package
                            .as_mut()
                            .unwrap()
                            .set_location_base(Some(location_base));
                    }
                    package.as_mut().unwrap().set_location_href(location_href);
                }
                TAG_FORMAT => {
                    // TODO: allocations
                    buf.clear();
                    text_buf.clear();
                    loop {
                        match reader.read_event_into(&mut buf)? {
                            Event::End(e) if e.name().as_ref() == TAG_FORMAT.as_bytes() => break,
                            Event::Start(e) => match std::str::from_utf8(e.name().as_ref())
                                .unwrap_or("")
                            {
                                TAG_RPM_LICENSE => {
                                    let text = reader
                                        .read_text_into(
                                            QName(TAG_RPM_LICENSE.as_bytes()),
                                            &mut text_buf,
                                        )?
                                        .decode()?;
                                    package.as_mut().unwrap().set_rpm_license(text);
                                }
                                TAG_RPM_VENDOR => {
                                    let text = reader
                                        .read_text_into(
                                            QName(TAG_RPM_VENDOR.as_bytes()),
                                            &mut text_buf,
                                        )?
                                        .decode()?;
                                    package.as_mut().unwrap().set_rpm_vendor(text);
                                }
                                TAG_RPM_GROUP => {
                                    let text = reader
                                        .read_text_into(
                                            QName(TAG_RPM_GROUP.as_bytes()),
                                            &mut text_buf,
                                        )?
                                        .decode()?;
                                    package.as_mut().unwrap().set_rpm_group(text);
                                }
                                TAG_RPM_BUILDHOST => {
                                    let text = reader
                                        .read_text_into(
                                            QName(TAG_RPM_BUILDHOST.as_bytes()),
                                            &mut text_buf,
                                        )?
                                        .decode()?;
                                    package.as_mut().unwrap().set_rpm_buildhost(text);
                                }
                                TAG_RPM_SOURCERPM => {
                                    let text = reader
                                        .read_text_into(
                                            QName(TAG_RPM_SOURCERPM.as_bytes()),
                                            &mut text_buf,
                                        )?
                                        .decode()?;
                                    package.as_mut().unwrap().set_rpm_sourcerpm(text);
                                }
                                TAG_RPM_HEADER_RANGE => {
                                    let start = e
                                        .try_get_attribute("start")?
                                        .ok_or_else(|| {
                                            MetadataError::MissingAttributeError("start")
                                        })?
                                        .unescape_value()?
                                        .parse()?;

                                    let end = e
                                        .try_get_attribute("end")?
                                        .ok_or_else(|| MetadataError::MissingAttributeError("end"))?
                                        .unescape_value()?
                                        .parse()?;

                                    package.as_mut().unwrap().set_rpm_header_range(start, end);
                                }
                                TAG_RPM_PROVIDES => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_provides(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_REQUIRES => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_requires(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_CONFLICTS => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_conflicts(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_OBSOLETES => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_obsoletes(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_SUGGESTS => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_suggests(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_ENHANCES => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_enhances(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_RECOMMENDS => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_recommends(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_SUPPLEMENTS => {
                                    package
                                        .as_mut()
                                        .unwrap()
                                        .set_supplements(parse_requirement_list(reader, &e)?);
                                }
                                TAG_FILE => (),
                                // TODO: share implementation w/ filelists, but don't parse twice.
                                // use IndexSet to enforce uniqueness while keeping order
                                _ => (),
                            },
                            _ => (),
                        }
                    }
                }
                _ => (),
            },
            Event::Eof => break,
            _ => (),
            // TODO: match arms, make sure nothing falls through
        }
        buf.clear();
        text_buf.clear();
    }

    // package.parse_state |= ParseState::PRIMARY;
    Ok(())
}

pub struct PrimaryXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> PrimaryXmlWriter<W> {
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="210">
        let mut metadata_tag = BytesStart::new(TAG_METADATA);
        metadata_tag.push_attribute(("xmlns", XML_NS_COMMON));
        metadata_tag.push_attribute(("xmlns:rpm", XML_NS_RPM));
        metadata_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(metadata_tag.borrow()))?;

        Ok(())
    }

    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        write_package(&mut self.writer, package)?;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </metadata>
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_METADATA)))?;

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

pub fn write_package<W: Write>(
    writer: &mut Writer<W>,
    package: &Package,
) -> Result<(), MetadataError> {
    // <package type="rpm">
    let mut package_tag = BytesStart::new(TAG_PACKAGE);
    package_tag.push_attribute(("type", "rpm"));
    writer.write_event(Event::Start(package_tag.borrow()))?;

    // <name>horse</name>
    writer
        .create_element(TAG_NAME)
        .write_text_content(BytesText::new(package.name()))?;

    // <arch>noarch</arch>
    writer
        .create_element(TAG_ARCH)
        .write_text_content(BytesText::new(package.arch()))?;

    // <version epoch="0" ver="4.1" rel="1"/>
    let (epoch, version, release) = package.evr().values();
    writer
        .create_element(TAG_VERSION)
        .with_attribute(("epoch", epoch))
        .with_attribute(("ver", version))
        .with_attribute(("rel", release))
        .write_empty()?;

    // <checksum type="sha256" pkgid="YES">6d0fd7f08cef63677726973d327e0b99f819b1983f90c2b656bb27cd2112cb7f</checksum>
    let (checksum_type, checksum_value) = package.checksum().to_values()?;
    writer
        .create_element(TAG_CHECKSUM)
        .with_attribute(("type", checksum_type))
        .with_attribute(("pkgid", "YES"))
        .write_text_content(BytesText::new(checksum_value))?;

    // <summary>A dummy package of horse</summary>
    writer
        .create_element(TAG_SUMMARY)
        .write_text_content(BytesText::new(package.summary()))?;

    // <description>A dummy package of horse</description>
    writer
        .create_element(TAG_DESCRIPTION)
        .write_text_content(BytesText::new(package.description()))?;

    // <packager>Bojack Horseman</packager>
    writer
        .create_element(TAG_PACKAGER)
        .write_text_content(BytesText::new(package.packager()))?;

    // <url>http://arandomaddress.com</url>
    writer
        .create_element(TAG_URL)
        .write_text_content(BytesText::new(package.url()))?;

    // <time file="1615451135" build="1331831374"/>
    writer
        .create_element(TAG_TIME)
        .with_attribute(("file", package.time_file().to_string().as_str()))
        .with_attribute(("build", package.time_build().to_string().as_str()))
        .write_empty()?;

    // <size package="1846" installed="42" archive="296"/>
    writer
        .create_element(TAG_SIZE)
        .with_attribute(("package", package.size_package().to_string().as_str()))
        .with_attribute(("installed", package.size_installed().to_string().as_str()))
        .with_attribute(("archive", package.size_archive().to_string().as_str()))
        .write_empty()?;

    // <location href="horse-4.1-1.noarch.rpm"/>
    writer
        .create_element(TAG_LOCATION)
        .with_attribute(("href", package.location_href()))
        .write_empty()?;

    // <format>
    let format_tag = BytesStart::new(TAG_FORMAT);
    writer.write_event(Event::Start(format_tag.borrow()))?;

    // <rpm:license>GPLv2</rpm:license>
    writer
        .create_element(TAG_RPM_LICENSE)
        .write_text_content(BytesText::new(package.rpm_license()))?;

    // <rpm:vendor></rpm:vendor>
    writer
        .create_element(TAG_RPM_VENDOR)
        .write_text_content(BytesText::new(package.rpm_vendor()))?;

    // <rpm:group>Internet/Applications</rpm:group>
    writer
        .create_element(TAG_RPM_GROUP)
        .write_text_content(BytesText::new(&package.rpm_group()))?;

    // <rpm:buildhost>smqe-ws15</rpm:buildhost>
    writer
        .create_element(TAG_RPM_BUILDHOST)
        .write_text_content(BytesText::new(&package.rpm_buildhost()))?;

    // <rpm:sourcerpm>horse-4.1-1.src.rpm</rpm:sourcerpm>
    writer
        .create_element(TAG_RPM_SOURCERPM)
        .write_text_content(BytesText::new(&package.rpm_sourcerpm()))?;

    // <rpm:header-range start="280" end="1697"/>
    let header_start = package.rpm_header_range().start.to_string();
    let header_end = package.rpm_header_range().end.to_string();
    writer
        .create_element(TAG_RPM_HEADER_RANGE)
        .with_attribute(("start", header_start.as_str()))
        .with_attribute(("end", header_end.as_str()))
        .write_empty()?;

    // <rpm:supplements>
    //   <rpm:entry name="horse" flags="EQ" epoch="0" ver="4.1" rel="1"/>
    // </rpm:supplements>
    write_requirement_section(writer, TAG_RPM_PROVIDES, package.provides())?;
    write_requirement_section(writer, TAG_RPM_REQUIRES, package.requires())?;
    write_requirement_section(writer, TAG_RPM_CONFLICTS, package.conflicts())?;
    write_requirement_section(writer, TAG_RPM_OBSOLETES, package.obsoletes())?;
    write_requirement_section(writer, TAG_RPM_SUGGESTS, package.suggests())?;
    write_requirement_section(writer, TAG_RPM_ENHANCES, package.enhances())?;
    write_requirement_section(writer, TAG_RPM_RECOMMENDS, package.recommends())?;
    write_requirement_section(writer, TAG_RPM_SUPPLEMENTS, package.supplements())?;

    // <file>/usr/bin/bash</file>
    package
        .files()
        .iter()
        .filter(|f| crate::utils::is_primary_file(&f.path))
        .try_for_each(|f| filelist::write_file_element(writer, f))?;

    // </format>
    writer.write_event(Event::End(format_tag.to_end()))?;

    // </package>
    writer.write_event(Event::End(package_tag.to_end()))?;

    Ok(())
}

// <rpm:supplements>
//   <rpm:entry name="horse" flags="EQ" epoch="0" ver="4.1" rel="1"/>
// </rpm:supplements>
fn write_requirement_section<W: Write>(
    writer: &mut Writer<W>,
    section_name: &str,
    entry_list: &[Requirement],
) -> Result<(), MetadataError> {
    // skip writing empty sections
    if entry_list.is_empty() {
        return Ok(());
    }

    let section_tag = BytesStart::new(section_name);
    writer.write_event(Event::Start(section_tag.borrow()))?;

    for entry in entry_list {
        let mut entry_tag = BytesStart::new("rpm:entry");
        entry_tag.push_attribute(("name", entry.name.as_str()));

        if let Some(flags) = &entry.flags {
            entry_tag.push_attribute(("flags", flags.as_str()));
        }

        if let Some(epoch) = &entry.epoch {
            entry_tag.push_attribute(("epoch", epoch.as_str()));
        }

        if let Some(version) = &entry.version {
            entry_tag.push_attribute(("ver", version.as_str()));
        }

        if let Some(release) = &entry.release {
            entry_tag.push_attribute(("rel", release.as_str()));
        }
        if entry.preinstall {
            entry_tag.push_attribute(("pre", "1"));
        }
        writer.write_event(Event::Empty(entry_tag))?;
    }

    writer.write_event(Event::End(section_tag.to_end()))?;

    Ok(())
}

pub fn parse_requirement_list<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<Vec<Requirement>, MetadataError> {
    let mut list = Vec::with_capacity(10);

    // TODO: another hot allocation
    let mut buf = Vec::with_capacity(128);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == TAG_RPM_ENTRY.as_bytes() => {
                let mut requirement = Requirement::default();
                for attr in e.attributes() {
                    let attr = attr?;
                    match attr.key.as_ref() {
                        b"name" => {
                            requirement.name = attr.unescape_value()?.into_owned();
                        }
                        b"flags" => requirement.flags = Some(attr.unescape_value()?.into_owned()),
                        b"epoch" => requirement.epoch = Some(attr.unescape_value()?.into_owned()),
                        b"ver" => requirement.version = Some(attr.unescape_value()?.into_owned()),
                        b"rel" => requirement.release = Some(attr.unescape_value()?.into_owned()),
                        b"pre" => {
                            let val = attr.unescape_value()?;
                            requirement.preinstall =
                                val != "0" && !val.eq_ignore_ascii_case("false");
                        }
                        a @ _ => {
                            return Err(MetadataError::UnknownAttributeError(format!(
                                "unrecognized attribute {}",
                                std::str::from_utf8(a)?
                            )));
                        }
                    }
                }

                if requirement.name.is_empty() {
                    return Err(MetadataError::MissingAttributeError("name"));
                }

                list.push(requirement);
            }
            Event::End(e) if e.name() == open_tag.name() => break,
            _ => (), // TODO
        }
    }

    Ok(list)
}
