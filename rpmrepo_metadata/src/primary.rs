use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use super::metadata::{
    Checksum, MetadataError, Package, ParseState, PrimaryXml, Requirement, RpmMetadata,
    XML_NS_COMMON, XML_NS_RPM,
};
use super::{FileType, PackageFile, Repository, EVR};

const TAG_METADATA: &[u8] = b"metadata";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_NAME: &[u8] = b"name";
const TAG_VERSION: &[u8] = b"version";
const TAG_CHECKSUM: &[u8] = b"checksum";
const TAG_ARCH: &[u8] = b"arch";
const TAG_SUMMARY: &[u8] = b"summary";
const TAG_DESCRIPTION: &[u8] = b"description";
const TAG_PACKAGER: &[u8] = b"packager";
const TAG_URL: &[u8] = b"url";
const TAG_TIME: &[u8] = b"time";
const TAG_SIZE: &[u8] = b"size";
const TAG_LOCATION: &[u8] = b"location";
const TAG_FORMAT: &[u8] = b"format";

const TAG_RPM_LICENSE: &[u8] = b"rpm:license";
const TAG_RPM_VENDOR: &[u8] = b"rpm:vendor";
const TAG_RPM_GROUP: &[u8] = b"rpm:group";
const TAG_RPM_BUILDHOST: &[u8] = b"rpm:buildhost";
const TAG_RPM_SOURCERPM: &[u8] = b"rpm:sourcerpm";
const TAG_RPM_HEADER_RANGE: &[u8] = b"rpm:header-range";

const TAG_RPM_ENTRY: &[u8] = b"rpm:entry";
const TAG_RPM_PROVIDES: &[u8] = b"rpm:provides";
const TAG_RPM_REQUIRES: &[u8] = b"rpm:requires";
const TAG_RPM_CONFLICTS: &[u8] = b"rpm:conflicts";
const TAG_RPM_OBSOLETES: &[u8] = b"rpm:obsoletes";
const TAG_RPM_SUGGESTS: &[u8] = b"rpm:suggests";
const TAG_RPM_ENHANCES: &[u8] = b"rpm:enhances";
const TAG_RPM_RECOMMENDS: &[u8] = b"rpm:recommends";
const TAG_RPM_SUPPLEMENTS: &[u8] = b"rpm:supplements";
const TAG_FILE: &[u8] = b"file";

impl RpmMetadata for PrimaryXml {
    fn filename() -> &'static str {
        "primary.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError> {
        read_primary_xml(repository, reader)
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
    pub fn new_writer<W: Write>(writer: Writer<W>) -> PrimaryXmlWriter<W> {
        PrimaryXmlWriter {
            writer,
            num_packages: 0,
            packages_written: 0,
        }
    }

    pub fn new_reader<'a, R: BufRead>(reader: &'a mut Reader<R>) -> PrimaryXmlReader<'a, R> {
        PrimaryXmlReader { reader }
    }
}

pub struct PrimaryXmlWriter<W: Write> {
    writer: Writer<W>,
    num_packages: usize,
    packages_written: usize,
}

impl<W: Write> PrimaryXmlWriter<W> {
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        self.num_packages = num_pkgs;

        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

        // <metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="210">
        let mut metadata_tag = BytesStart::borrowed_name(TAG_METADATA);
        metadata_tag.push_attribute(("xmlns", XML_NS_COMMON));
        metadata_tag.push_attribute(("xmlns:rpm", XML_NS_RPM));
        metadata_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(metadata_tag.to_borrowed()))?;

        Ok(())
    }

    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        write_package(&mut self.writer, package)?;
        self.packages_written += 1;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        assert_eq!(
            self.packages_written, self.num_packages,
            "Number of packages written {} does not match number of packages declared {}.",
            self.packages_written, self.num_packages
        );

        // </metadata>
        self.writer
            .write_event(Event::End(BytesEnd::borrowed(TAG_METADATA)))?;

        // trailing newline
        self.writer
            .write_event(Event::Text(BytesText::from_plain_str("\n")))?;

        // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
        self.writer.inner().flush()?;

        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

pub struct PrimaryXmlReader<'a, R: BufRead> {
    reader: &'a mut Reader<R>,
}

impl<'a, R: BufRead> PrimaryXmlReader<'a, R> {
    pub fn read_header(&mut self) {}

    pub fn read_package(&mut self, package: &mut Package) {}

    pub fn finish(&mut self) {}
}

fn read_primary_xml<R: BufRead>(
    repository: &mut Repository,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();
    let mut found_metadata_tag = false;

    // TODO: less buffers, less allocation
    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => match e.name() {
                TAG_METADATA => {
                    found_metadata_tag = true;
                }
                TAG_PACKAGE => {
                    let ptype = e
                        .try_get_attribute(b"type")?
                        .unwrap()
                        .unescape_and_decode_value(reader)?;

                    assert_eq!(&ptype, "rpm"); // TODO: better error handling

                    // TODO: in theory, other or filelists could be parsed first, and in that case this is wrong
                    // need to at least enforce order w/ a state machine, or just handle it.
                    let mut package = Package::default();
                    // TODO: need to do something with the data if it already existed
                    parse_package(&mut package, reader)?;
                    let (_, pkgid) = package.checksum().to_values()?;
                    repository
                        .packages_mut()
                        .entry(pkgid.to_owned())
                        .or_insert(package);
                }
                _ => (),
            },
            Event::Eof => break,
            Event::Decl(_) => (), // TODO
            _ => (),
        }
    }
    if !found_metadata_tag {
        // TODO
    }

    Ok(())
}

pub fn write_package<W: Write>(
    writer: &mut Writer<W>,
    package: &Package,
) -> Result<(), MetadataError> {
    // <package type="rpm">
    let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
    package_tag.push_attribute(("type", "rpm"));
    writer.write_event(Event::Start(package_tag.to_borrowed()))?;

    // <name>horse</name>
    writer
        .create_element(TAG_NAME)
        .write_text_content(BytesText::from_plain_str(package.name()))?;

    // <arch>noarch</arch>
    writer
        .create_element(TAG_ARCH)
        .write_text_content(BytesText::from_plain_str(package.arch()))?;

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
        .write_text_content(BytesText::from_plain_str(checksum_value))?;

    // <summary>A dummy package of horse</summary>
    writer
        .create_element(TAG_SUMMARY)
        .write_text_content(BytesText::from_plain_str(package.summary()))?;

    // <description>A dummy package of horse</description>
    writer
        .create_element(TAG_DESCRIPTION)
        .write_text_content(BytesText::from_plain_str(package.description()))?;

    // <packager>Bojack Horseman</packager>
    writer
        .create_element(TAG_PACKAGER)
        .write_text_content(BytesText::from_plain_str(package.packager()))?;

    // <url>http://arandomaddress.com</url>
    writer
        .create_element(TAG_URL)
        .write_text_content(BytesText::from_plain_str(package.url()))?;

    // <time file="1615451135" build="1331831374"/>
    writer
        .create_element(TAG_TIME)
        .with_attribute(("file", package.time().file.to_string().as_str()))
        .with_attribute(("build", package.time().build.to_string().as_str()))
        .write_empty()?;

    // <size package="1846" installed="42" archive="296"/>
    writer
        .create_element(TAG_SIZE)
        .with_attribute(("package", package.size().package.to_string().as_str()))
        .with_attribute(("installed", package.size().installed.to_string().as_str()))
        .with_attribute(("archive", package.size().archive.to_string().as_str()))
        .write_empty()?;

    // <location href="horse-4.1-1.noarch.rpm"/>
    writer
        .create_element(TAG_LOCATION)
        .with_attribute(("href", package.location_href()))
        .write_empty()?;

    // <format>
    let format_tag = BytesStart::borrowed_name(TAG_FORMAT);
    writer.write_event(Event::Start(format_tag.to_borrowed()))?;

    // <rpm:license>GPLv2</rpm:license>
    writer
        .create_element(TAG_RPM_LICENSE)
        .write_text_content(BytesText::from_plain_str(package.rpm_license()))?;

    // <rpm:vendor></rpm:vendor>
    writer
        .create_element(TAG_RPM_VENDOR)
        .write_text_content(BytesText::from_plain_str(package.rpm_vendor()))?;

    // <rpm:group>Internet/Applications</rpm:group>
    writer
        .create_element(TAG_RPM_GROUP)
        .write_text_content(BytesText::from_plain_str(&package.rpm_group()))?;

    // <rpm:buildhost>smqe-ws15</rpm:buildhost>
    writer
        .create_element(TAG_RPM_BUILDHOST)
        .write_text_content(BytesText::from_plain_str(&package.rpm_buildhost()))?;

    // <rpm:sourcerpm>horse-4.1-1.src.rpm</rpm:sourcerpm>
    writer
        .create_element(TAG_RPM_SOURCERPM)
        .write_text_content(BytesText::from_plain_str(&package.rpm_sourcerpm()))?;

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

    // TODO: check this logic
    fn include_file(f: &PackageFile) -> bool {
        // strange algorithm, but it's what the original uses
        f.path.starts_with("/etc/")
            || f.path.contains("bin/")
            || f.path.starts_with("/usr/lib/sendmail")
    }

    // <file>/usr/bin/bash</file>
    for file in package.files() {
        if file.filetype == FileType::File && include_file(file) {
            writer
                .create_element(TAG_FILE)
                .write_text_content(BytesText::from_plain_str(&file.path))?;
        }
    }

    // </format>
    writer.write_event(Event::End(format_tag.to_end()))?;

    // </package>
    writer.write_event(Event::End(package_tag.to_end()))?;

    Ok(())
}

// <rpm:supplements>
//   <rpm:entry name="horse" flags="EQ" epoch="0" ver="4.1" rel="1"/>
// </rpm:supplements>
fn write_requirement_section<W: Write, N: AsRef<[u8]> + Sized>(
    writer: &mut Writer<W>,
    section_name: N,
    entry_list: &[Requirement],
) -> Result<(), MetadataError> {
    // skip writing empty sections
    if entry_list.is_empty() {
        return Ok(());
    }

    let section_tag = BytesStart::borrowed_name(section_name.as_ref());
    writer.write_event(Event::Start(section_tag.to_borrowed()))?;

    for entry in entry_list {
        let mut entry_tag = BytesStart::borrowed_name(b"rpm:entry");
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
        // TODO: make sure this logic is correct, should this be option or just plain bool?
        if let Some(true) = &entry.preinstall {
            entry_tag.push_attribute(("pre", "1"));
        }
        writer.write_event(Event::Empty(entry_tag))?;
    }

    writer.write_event(Event::End(section_tag.to_end()))?;

    Ok(())
}

pub fn parse_package<R: BufRead>(
    package: &mut Package,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = vec![];
    let mut text_buf = vec![];

    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_PACKAGE => break,
            Event::Start(e) => match e.name() {
                TAG_NAME => {
                    package.set_name(reader.read_text(TAG_NAME, &mut text_buf)?.as_str());
                }
                TAG_VERSION => {
                    // TODO: unescape_and_decode_value allocates, that can probably be avoided
                    let epoch = e
                        .try_get_attribute("epoch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
                        .unescape_and_decode_value(reader)?;

                    let version = e
                        .try_get_attribute("ver")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("ver"))?
                        .unescape_and_decode_value(reader)?;

                    let release = e
                        .try_get_attribute("rel")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("rel"))?
                        .unescape_and_decode_value(reader)?;

                    // TODO: temporary conversions
                    let evr = EVR::new(epoch.as_str(), version.as_str(), release.as_str());
                    package.set_evr(evr);
                }
                TAG_CHECKSUM => {
                    let checksum_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                        .unescape_and_decode_value(reader)?;
                    let checksum_value = reader.read_text(TAG_CHECKSUM, &mut text_buf)?;
                    package.set_checksum(Checksum::try_create(checksum_type, checksum_value)?);
                }
                TAG_ARCH => {
                    package.set_arch(reader.read_text(TAG_ARCH, &mut text_buf)?.as_str());
                }
                TAG_SUMMARY => {
                    package.set_summary(reader.read_text(TAG_SUMMARY, &mut text_buf)?.as_str());
                }
                TAG_DESCRIPTION => {
                    package.set_description(
                        reader.read_text(TAG_DESCRIPTION, &mut text_buf)?.as_str(),
                    );
                }
                TAG_PACKAGER => {
                    package.set_packager(reader.read_text(TAG_PACKAGER, &mut text_buf)?.as_str());
                }
                TAG_URL => {
                    package.set_url(reader.read_text(TAG_URL, &mut text_buf)?.as_str());
                }
                TAG_TIME => {
                    let time_file = e
                        .try_get_attribute("file")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("file"))?
                        .unescape_and_decode_value(reader)?
                        .parse()?;

                    let time_build = e
                        .try_get_attribute("build")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("build"))?
                        .unescape_and_decode_value(reader)?
                        .parse()?;

                    package.set_time(time_file, time_build);
                }
                TAG_SIZE => {
                    let package_size = e
                        .try_get_attribute("package")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("package"))?
                        .unescape_and_decode_value(reader)?
                        .parse()?;

                    let installed_size = e
                        .try_get_attribute("installed")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("installed"))?
                        .unescape_and_decode_value(reader)?
                        .parse()?;

                    let archive_size = e
                        .try_get_attribute("archive")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("archive"))?
                        .unescape_and_decode_value(reader)?
                        .parse()?;

                    package.set_size(package_size, installed_size, archive_size);
                }
                TAG_LOCATION => {
                    let location_href = e
                        .try_get_attribute("href")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("href"))?
                        .unescape_and_decode_value(reader)?;
                    package.set_location_href(&location_href);
                }
                TAG_FORMAT => {
                    // TODO: allocations
                    let mut format_buf = vec![];
                    let mut format_text_buf = vec![];
                    loop {
                        match reader.read_event(&mut format_buf)? {
                            Event::End(e) if e.name() == TAG_FORMAT => break,
                            Event::Start(e) => match e.name() {
                                TAG_RPM_LICENSE => {
                                    package.set_rpm_license(
                                        reader
                                            .read_text(TAG_RPM_LICENSE, &mut format_text_buf)?
                                            .as_str(),
                                    );
                                }
                                TAG_RPM_VENDOR => {
                                    package.set_rpm_vendor(
                                        reader
                                            .read_text(TAG_RPM_VENDOR, &mut format_text_buf)?
                                            .as_str(),
                                    );
                                }
                                TAG_RPM_GROUP => {
                                    package.set_rpm_group(
                                        reader
                                            .read_text(TAG_RPM_GROUP, &mut format_text_buf)?
                                            .as_str(),
                                    );
                                }
                                TAG_RPM_BUILDHOST => {
                                    package.set_rpm_buildhost(
                                        reader
                                            .read_text(TAG_RPM_BUILDHOST, &mut format_text_buf)?
                                            .as_str(),
                                    );
                                }
                                TAG_RPM_SOURCERPM => {
                                    package.set_rpm_sourcerpm(
                                        reader
                                            .read_text(TAG_RPM_SOURCERPM, &mut format_text_buf)?
                                            .as_str(),
                                    );
                                }
                                TAG_RPM_HEADER_RANGE => {
                                    let start = e
                                        .try_get_attribute("start")?
                                        .ok_or_else(|| {
                                            MetadataError::MissingAttributeError("start")
                                        })?
                                        .unescape_and_decode_value(reader)?
                                        .parse()?;

                                    let end = e
                                        .try_get_attribute("end")?
                                        .ok_or_else(|| MetadataError::MissingAttributeError("end"))?
                                        .unescape_and_decode_value(reader)?
                                        .parse()?;

                                    package.set_rpm_header_range(start, end);
                                }
                                TAG_RPM_PROVIDES => {
                                    package.set_provides(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_REQUIRES => {
                                    package.set_requires(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_CONFLICTS => {
                                    package.set_conflicts(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_OBSOLETES => {
                                    package.set_obsoletes(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_SUGGESTS => {
                                    package.set_suggests(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_ENHANCES => {
                                    package.set_enhances(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_RECOMMENDS => {
                                    package.set_recommends(parse_requirement_list(reader, &e)?);
                                }
                                TAG_RPM_SUPPLEMENTS => {
                                    package.set_supplements(parse_requirement_list(reader, &e)?);
                                }
                                TAG_FILE => (), // TODO: share implementation w/ filelists, but don't parse twice.
                                _ => (),
                            },
                            _ => (),
                        }
                        format_buf.clear();
                        format_text_buf.clear();
                    }
                }
                _ => (),
            },
            _ => (),
            // TODO: match arms, make sure nothing falls through
        }
        buf.clear();
        text_buf.clear();
    }

    // package.parse_state |= ParseState::PRIMARY;
    Ok(())
}

pub fn parse_requirement_list<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<Vec<Requirement>, MetadataError> {
    let mut list = vec![];

    // TODO: another hot allocation
    let mut buf = vec![];

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) if e.name() == TAG_RPM_ENTRY => {
                let name = e
                    .try_get_attribute("name")?
                    .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                    .unescape_and_decode_value(reader)?;

                let flags = e
                    .try_get_attribute("flags")?
                    .and_then(|attr| attr.unescape_and_decode_value(reader).ok());

                let epoch = e
                    .try_get_attribute("epoch")?
                    .and_then(|attr| attr.unescape_and_decode_value(reader).ok());

                let version = e
                    .try_get_attribute("ver")?
                    .and_then(|attr| attr.unescape_and_decode_value(reader).ok());

                let release = e
                    .try_get_attribute("rel")?
                    .and_then(|attr| attr.unescape_and_decode_value(reader).ok());

                let preinstall = e.try_get_attribute("pre")?.map_or(None, |a| {
                    let val = a.unescape_and_decode_value(reader).unwrap(); // TODO
                    Some(val != "0" || !val.eq_ignore_ascii_case("false"))
                });

                list.push(Requirement {
                    name,
                    flags,
                    epoch,
                    version,
                    release,
                    preinstall,
                });
            }
            Event::End(e) if e.name() == open_tag.name() => break,
            _ => (), // TODO
        }
    }

    Ok(list)
}
