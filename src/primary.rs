// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, name::QName};

use crate::constants::{tag::*, xmlns};
use crate::filelist;
use crate::metadata::{
    Checksum, FileType, MetadataError, Package, PrimaryXml, Requirement, RequirementType,
    RpmMetadata,
};
use crate::parsing_utils::{self, resolve_attr, resolve_text};
use crate::visitor::{PrimaryVisitor, RequirementData};
use crate::{EVR, Repository};

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
            if package.is_none() {
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
    /// Create a new primary.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> PrimaryXmlWriter<W> {
        PrimaryXmlWriter { writer }
    }

    /// Create a new primary.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> PrimaryXmlReader<R> {
        PrimaryXmlReader { reader }
    }
}

/// Streaming reader for primary.xml metadata.
pub struct PrimaryXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> PrimaryXmlReader<R> {
    /// Read and parse the XML header, returning the declared package count.
    pub fn read_header(&mut self) -> Result<usize, MetadataError> {
        parse_primary_header(&mut self.reader)
    }

    /// Read the next package element into `package`, or set it to `None` if no more packages.
    pub fn read_package(&mut self, package: &mut Option<Package>) -> Result<(), MetadataError> {
        let mut materializer = PackageMaterializer {
            package,
            error: None,
        };
        let found = parse_primary_package(&mut self.reader, &mut materializer)?;
        if let Some(err) = materializer.error {
            return Err(err);
        }
        if !found {
            *materializer.package = None;
        }
        Ok(())
    }
}

/// Parse `<rpm:entry>` children from a requirement section, calling `emit`
/// for each one with borrowed data.
fn parse_requirement_list_visitor(
    reader: &mut Reader<impl BufRead>,
    open_tag: &BytesStart<'_>,
    mut emit: impl FnMut(RequirementData<'_>),
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(128);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) if e.name().as_ref() == TAG_RPM_ENTRY.as_bytes() => {
                let mut name_cow = None;
                let mut flags = None;
                let mut epoch_cow = None;
                let mut ver_cow = None;
                let mut rel_cow = None;
                let mut preinstall = false;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        b"name" => name_cow = Some(resolve_attr(&attr)?),
                        b"flags" => {
                            let val = resolve_attr(&attr)?;
                            flags = Some(RequirementType::try_from(val.as_ref())?);
                        }
                        b"epoch" => epoch_cow = Some(resolve_attr(&attr)?),
                        b"ver" => ver_cow = Some(resolve_attr(&attr)?),
                        b"rel" => rel_cow = Some(resolve_attr(&attr)?),
                        b"pre" => {
                            let val = resolve_attr(&attr)?;
                            preinstall =
                                val.as_ref() != "0" && !val.as_ref().eq_ignore_ascii_case("false");
                        }
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;

                emit(RequirementData {
                    name: &name,
                    flags,
                    epoch: epoch_cow.as_deref(),
                    version: ver_cow.as_deref(),
                    release: rel_cow.as_deref(),
                    preinstall,
                });
            }
            Event::End(e) if e.name() == open_tag.name() => break,
            _ => (),
        }
    }

    Ok(())
}

// ── Primary parser ──────────────────────────────────────────────────────

/// Parse the primary.xml header, returning the declared package count.
pub fn parse_primary_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    parsing_utils::parse_header_tag(reader, TAG_METADATA)
}

/// Parse one `<package>` element from primary.xml, dispatching to `visitor`.
///
/// Returns `true` if a package was parsed, `false` at EOF.
pub fn parse_primary_package<R: BufRead, V: PrimaryVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<bool, MetadataError> {
    let mut buf = Vec::with_capacity(512);
    let mut text_buf = Vec::with_capacity(512);

    let mut pkg_name = String::new();
    let mut pkg_arch = String::new();
    let mut checksum_type = String::new();
    let mut pkgid = String::new();
    let mut pkg_epoch = String::new();
    let mut pkg_version = String::new();
    let mut pkg_release = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE.as_bytes() => {
                visitor.end_package();
                return Ok(true);
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGE => {}
                TAG_NAME => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    pkg_name.clear();
                    pkg_name.push_str(&text);
                }
                TAG_ARCH => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_ARCH.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    pkg_arch.clear();
                    pkg_arch.push_str(&text);
                }
                TAG_CHECKSUM => {
                    let ctype_attr = e
                        .try_get_attribute("type")?
                        .ok_or(MetadataError::MissingAttributeError("type"))?;
                    let ctype = resolve_attr(&ctype_attr)?;
                    let bytes_text =
                        reader.read_text_into(QName(TAG_CHECKSUM.as_bytes()), &mut text_buf)?;
                    let value = resolve_text(&bytes_text)?;
                    checksum_type.clear();
                    checksum_type.push_str(&ctype);
                    pkgid.clear();
                    pkgid.push_str(&value);

                    visitor.begin_package(&pkg_name, &pkg_arch, &checksum_type, &pkgid);
                    visitor.set_evr(&pkg_epoch, &pkg_version, &pkg_release);
                }
                TAG_VERSION => {
                    let (epoch, version, release) = parsing_utils::parse_evr_from_tag(&e)?;
                    pkg_epoch.clear();
                    pkg_epoch.push_str(&epoch);
                    pkg_version.clear();
                    pkg_version.push_str(&version);
                    pkg_release.clear();
                    pkg_release.push_str(&release);
                }
                TAG_SUMMARY => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_SUMMARY.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_summary(&text);
                }
                TAG_DESCRIPTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_description(&text);
                }
                TAG_PACKAGER => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_PACKAGER.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_packager(&text);
                }
                TAG_URL => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_URL.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_url(&text);
                }
                TAG_TIME => {
                    let mut file = None;
                    let mut build = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"file" => file = Some(resolve_attr(&attr)?.parse()?),
                            b"build" => build = Some(resolve_attr(&attr)?.parse()?),
                            _ => (),
                        }
                    }

                    visitor.set_time(
                        file.ok_or(MetadataError::MissingAttributeError("file"))?,
                        build.ok_or(MetadataError::MissingAttributeError("build"))?,
                    );
                }
                TAG_SIZE => {
                    let mut package = None;
                    let mut installed = None;
                    let mut archive = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"package" => package = Some(resolve_attr(&attr)?.parse()?),
                            b"installed" => installed = Some(resolve_attr(&attr)?.parse()?),
                            b"archive" => archive = Some(resolve_attr(&attr)?.parse()?),
                            _ => (),
                        }
                    }

                    visitor.set_size(
                        package.ok_or(MetadataError::MissingAttributeError("package"))?,
                        installed.ok_or(MetadataError::MissingAttributeError("installed"))?,
                        archive.ok_or(MetadataError::MissingAttributeError("archive"))?,
                    );
                }
                TAG_LOCATION => {
                    let mut href_cow = None;
                    let mut base_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"href" => href_cow = Some(resolve_attr(&attr)?),
                            b"base" => base_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let href = href_cow.ok_or(MetadataError::MissingAttributeError("href"))?;
                    visitor.set_location(&href, base_cow.as_deref());
                }
                TAG_FORMAT => {
                    parse_format_block(reader, visitor, &mut buf, &mut text_buf)?;
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

fn parse_format_block<R: BufRead, V: PrimaryVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    buf.clear();
    text_buf.clear();

    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_FORMAT.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_RPM_LICENSE => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RPM_LICENSE.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_rpm_license(&text);
                }
                TAG_RPM_VENDOR => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RPM_VENDOR.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_rpm_vendor(&text);
                }
                TAG_RPM_GROUP => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RPM_GROUP.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_rpm_group(&text);
                }
                TAG_RPM_BUILDHOST => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RPM_BUILDHOST.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_rpm_buildhost(&text);
                }
                TAG_RPM_SOURCERPM => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RPM_SOURCERPM.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_rpm_sourcerpm(&text);
                }
                TAG_RPM_HEADER_RANGE => {
                    let mut start = None;
                    let mut end = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"start" => start = Some(resolve_attr(&attr)?.parse()?),
                            b"end" => end = Some(resolve_attr(&attr)?.parse()?),
                            _ => (),
                        }
                    }

                    visitor.set_rpm_header_range(
                        start.ok_or(MetadataError::MissingAttributeError("start"))?,
                        end.ok_or(MetadataError::MissingAttributeError("end"))?,
                    );
                }
                TAG_RPM_PROVIDES => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_provide(req))?;
                }
                TAG_RPM_REQUIRES => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_require(req))?;
                }
                TAG_RPM_CONFLICTS => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_conflict(req))?;
                }
                TAG_RPM_OBSOLETES => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_obsolete(req))?;
                }
                TAG_RPM_SUGGESTS => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_suggest(req))?;
                }
                TAG_RPM_ENHANCES => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_enhance(req))?;
                }
                TAG_RPM_RECOMMENDS => {
                    parse_requirement_list_visitor(reader, &e, |req| visitor.add_recommend(req))?;
                }
                TAG_RPM_SUPPLEMENTS => {
                    parse_requirement_list_visitor(reader, &e, |req| {
                        visitor.add_supplement(req);
                    })?;
                }
                TAG_FILE => {
                    let filetype = if let Some(attr) = e.try_get_attribute("type")? {
                        FileType::try_create(attr.value.as_ref())?
                    } else {
                        FileType::File
                    };
                    let bytes_text = reader.read_text_into(e.name(), text_buf)?;
                    let path = resolve_text(&bytes_text)?;
                    visitor.add_file(filetype, &path);
                }
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}

struct PackageMaterializer<'a> {
    package: &'a mut Option<Package>,
    error: Option<MetadataError>,
}

impl PrimaryVisitor for PackageMaterializer<'_> {
    fn begin_package(&mut self, name: &str, arch: &str, checksum_type: &str, pkgid: &str) {
        match Checksum::try_create(checksum_type.as_bytes(), pkgid.as_bytes()) {
            Ok(checksum) => {
                let mut pkg = Package::default();
                pkg.set_name(name);
                pkg.set_arch(arch);
                pkg.set_checksum(checksum);
                *self.package = Some(pkg);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_evr(EVR::new(epoch, version, release));
        }
    }

    fn set_summary(&mut self, summary: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_summary(summary);
        }
    }

    fn set_description(&mut self, description: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_description(description);
        }
    }

    fn set_packager(&mut self, packager: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_packager(packager);
        }
    }

    fn set_url(&mut self, url: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_url(url);
        }
    }

    fn set_time(&mut self, file: u64, build: u64) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_time_file(file).set_time_build(build);
        }
    }

    fn set_size(&mut self, package: u64, installed: u64, archive: u64) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_size_package(package)
                .set_size_installed(installed)
                .set_size_archive(archive);
        }
    }

    fn set_location(&mut self, href: &str, base: Option<&str>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_location_href(href);
            pkg.set_location_base(base.map(String::from));
        }
    }

    fn set_rpm_license(&mut self, license: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_license(license);
        }
    }

    fn set_rpm_vendor(&mut self, vendor: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_vendor(vendor);
        }
    }

    fn set_rpm_group(&mut self, group: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_group(group);
        }
    }

    fn set_rpm_buildhost(&mut self, buildhost: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_buildhost(buildhost);
        }
    }

    fn set_rpm_sourcerpm(&mut self, sourcerpm: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_sourcerpm(sourcerpm);
        }
    }

    fn set_rpm_header_range(&mut self, start: u64, end: u64) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.set_rpm_header_range(start, end);
        }
    }

    fn add_provide(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_provides.push(Requirement::from(req));
        }
    }

    fn add_require(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_requires.push(Requirement::from(req));
        }
    }

    fn add_conflict(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_conflicts.push(Requirement::from(req));
        }
    }

    fn add_obsolete(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_obsoletes.push(Requirement::from(req));
        }
    }

    fn add_suggest(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_suggests.push(Requirement::from(req));
        }
    }

    fn add_enhance(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_enhances.push(Requirement::from(req));
        }
    }

    fn add_recommend(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_recommends.push(Requirement::from(req));
        }
    }

    fn add_supplement(&mut self, req: RequirementData<'_>) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.rpm_supplements.push(Requirement::from(req));
        }
    }

    fn add_file(&mut self, filetype: FileType, path: &str) {
        if let Some(pkg) = self.package.as_mut() {
            pkg.add_file(filetype, path);
        }
    }
}

/// Streaming writer for primary.xml metadata.
pub struct PrimaryXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> PrimaryXmlWriter<W> {
    /// Write the XML declaration and opening `<metadata>` element.
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <metadata xmlns="http://linux.duke.edu/metadata/common" xmlns:rpm="http://linux.duke.edu/metadata/rpm" packages="210">
        let mut metadata_tag = BytesStart::new(TAG_METADATA);
        metadata_tag.push_attribute(("xmlns", xmlns::NS_COMMON));
        metadata_tag.push_attribute(("xmlns:rpm", xmlns::NS_RPM));
        metadata_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(metadata_tag.borrow()))?;

        Ok(())
    }

    /// Write a single `<package>` element.
    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        write_package(&mut self.writer, package)?;
        Ok(())
    }

    /// Write the closing `</metadata>` element and flush.
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

    /// Consume the writer and return the underlying writer.
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
        .write_text_content(BytesText::new(package.rpm_group()))?;

    // <rpm:buildhost>smqe-ws15</rpm:buildhost>
    writer
        .create_element(TAG_RPM_BUILDHOST)
        .write_text_content(BytesText::new(package.rpm_buildhost()))?;

    // <rpm:sourcerpm>horse-4.1-1.src.rpm</rpm:sourcerpm>
    writer
        .create_element(TAG_RPM_SOURCERPM)
        .write_text_content(BytesText::new(package.rpm_sourcerpm()))?;

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
    let mut err: Result<(), MetadataError> = Ok(());
    package.files().for_each_file(|filetype, path| {
        if err.is_ok()
            && crate::utils::is_primary_file(path)
            && let Err(e) = filelist::write_file_entry(writer, filetype, path)
        {
            err = Err(e);
        }
    });
    err?;

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

        if let Some(flags) = entry.flags {
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
