use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use crate::Checksum;

use super::metadata::{
    FileType, FilelistsXml, Package, PackageFile, RpmMetadata, XML_NS_FILELISTS,
};
use super::{MetadataError, Repository, EVR};

const TAG_FILELISTS: &[u8] = b"filelists";
const TAG_PACKAGE: &[u8] = b"package";
const TAG_VERSION: &[u8] = b"version";
const TAG_FILE: &[u8] = b"file";

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
    pub fn new_writer<W: Write>(writer: Writer<W>) -> FilelistsXmlWriter<W> {
        FilelistsXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: Reader<R>) -> FilelistsXmlReader<R> {
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
            .write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

        // <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="210">
        let mut filelists_tag = BytesStart::borrowed_name(TAG_FILELISTS);
        filelists_tag.push_attribute(("xmlns", XML_NS_FILELISTS));
        filelists_tag.push_attribute(("packages", num_pkgs.to_string().as_str()));
        self.writer
            .write_event(Event::Start(filelists_tag.to_borrowed()))?;

        Ok(())
    }

    pub fn write_package(&mut self, package: &Package) -> Result<(), MetadataError> {
        // <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
        let mut package_tag = BytesStart::borrowed_name(TAG_PACKAGE);
        let pkgid = package.pkgid();
        package_tag.push_attribute(("pkgid", pkgid));
        package_tag.push_attribute(("name", package.name()));
        package_tag.push_attribute(("arch", package.arch()));
        self.writer
            .write_event(Event::Start(package_tag.to_borrowed()))?;

        // <version epoch="0" ver="2.8.0" rel="5.fc33"/>
        let (epoch, version, release) = package.evr().values();
        let mut version_tag = BytesStart::borrowed_name(TAG_VERSION);
        version_tag.push_attribute(("epoch", epoch));
        version_tag.push_attribute(("ver", version));
        version_tag.push_attribute(("rel", release));
        self.writer.write_event(Event::Empty(version_tag))?;

        // <file type="dir">/etc/fonts/conf.avail</file>
        package.files().iter().try_for_each(|f| write_file_element(&mut self.writer, f))?;

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </filelists>
        self.writer
            .write_event(Event::End(BytesEnd::borrowed(TAG_FILELISTS)))?;

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

pub(crate) fn write_file_element<W: Write>(
    writer: &mut Writer<W>,
    file: &PackageFile,
) -> Result<(), MetadataError> {
    let mut file_tag = BytesStart::borrowed_name(TAG_FILE);
    if file.filetype != FileType::File {
        file_tag.push_attribute(("type".as_bytes(), file.filetype.to_values()));
    }
    writer.write_event(Event::Start(file_tag.to_borrowed()))?;
    writer.write_event(Event::Text(BytesText::from_plain_str(&file.path)))?;
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
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        Ok(())
    }
}

// <?xml version="1.0" encoding="UTF-8"?>
// <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="35">
fn parse_header<R: BufRead>(reader: &mut Reader<R>) -> Result<usize, MetadataError> {
    let mut buf = Vec::new();

    // TODO: get rid of this buffer
    loop {
        match reader.read_event(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name() == TAG_FILELISTS => {
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
    let mut buf = Vec::new();

    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_PACKAGE => break,

            Event::Start(e) => match e.name() {
                TAG_PACKAGE => {
                    let pkgid = e
                        .try_get_attribute("pkgid")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("pkgid"))?
                        .unescape_and_decode_value(reader)?;
                    let name = e
                        .try_get_attribute("name")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
                        .unescape_and_decode_value(reader)?;
                    let arch = e
                        .try_get_attribute("arch")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
                        .unescape_and_decode_value(reader)?;

                    if let Some(pkg) = package {
                        assert!(pkg.pkgid() == &pkgid); // TODO err instead of assert
                    } else {
                        let mut pkg = Package::default();
                        pkg.set_name(&name)
                            .set_arch(&arch)
                            .set_checksum(Checksum::Unknown(pkgid));
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
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<EVR, MetadataError> {
    let epoch = open_tag
        .try_get_attribute("epoch")?
        .ok_or_else(|| MetadataError::MissingAttributeError("epoch"))?
        .unescape_and_decode_value(reader)?;
    let version = open_tag
        .try_get_attribute("ver")?
        .ok_or_else(|| MetadataError::MissingAttributeError("ver"))?
        .unescape_and_decode_value(reader)?;
    let release = open_tag
        .try_get_attribute("rel")?
        .ok_or_else(|| MetadataError::MissingAttributeError("rel"))?
        .unescape_and_decode_value(reader)?;

    // TODO: double-allocations
    Ok(EVR::new(&epoch, &version, &release))
}

// <file type="dir">/etc/fonts/conf.avail</file>
pub fn parse_file<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<PackageFile, MetadataError> {
    let mut file = PackageFile::default();
    file.path = reader.read_text(open_tag.name(), &mut Vec::new())?;

    if let Some(filetype) = open_tag.try_get_attribute("type")? {
        file.filetype = FileType::try_create(filetype.value.as_ref())?;
    }

    Ok(file)
}
