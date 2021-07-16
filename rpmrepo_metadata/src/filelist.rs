use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use super::metadata::{
    FileType, FilelistsXml, Package, PackageFile, ParseState, RpmMetadata, XML_NS_FILELISTS,
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
        reader: &mut Reader<R>,
    ) -> Result<(), MetadataError> {
        read_filelists_xml(repository, reader)
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
        FilelistsXmlWriter {
            writer,
            num_packages: 0,
            packages_written: 0,
        }
    }

    // pub fn new_reader<'a, R: BufRead>(reader: &'a mut Reader<R>) -> FilelistsXmlReader<'a, R> {
    //     FilelistsXmlReader {
    //         reader,
    //         num_packages: 0,
    //         packages_read: 0,
    //         buffer: Vec::new(),
    //     }
    // }
}

pub struct FilelistsXmlWriter<W: Write> {
    pub writer: Writer<W>,
    num_packages: usize,
    packages_written: usize,
}

impl<W: Write> FilelistsXmlWriter<W> {
    pub fn write_header(&mut self, num_pkgs: usize) -> Result<(), MetadataError> {
        self.num_packages = num_pkgs;

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
        let (_, pkgid) = package.checksum().to_values()?;
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
        for file in package.files() {
            let mut file_tag = BytesStart::borrowed_name(TAG_FILE);
            if file.filetype != FileType::File {
                file_tag.push_attribute(("type".as_bytes(), file.filetype.to_values()));
            }
            self.writer
                .write_event(Event::Start(file_tag.to_borrowed()))?;
            self.writer
                .write_event(Event::Text(BytesText::from_plain_str(&file.path)))?;
            self.writer.write_event(Event::End(file_tag.to_end()))?;
        }

        // </package>
        self.writer.write_event(Event::End(package_tag.to_end()))?;

        self.packages_written += 1;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<(), MetadataError> {
        assert_eq!(
            self.packages_written, self.num_packages,
            "Number of packages written {} does not match number of packages declared {}.",
            self.packages_written, self.num_packages
        );

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

// pub struct FilelistsXmlReader<'a, R: BufRead> {
//     reader: &'a mut Reader<R>,
//     num_packages: usize,
//     packages_read: usize,
//     buffer: Vec<u8>,
// }

// impl<'a, R: BufRead> FilelistsXmlReader<'a, R> {
//     pub fn read_header(&mut self) -> Result<(), MetadataError> {
//         let mut found_metadata_tag = false;

//         loop {
//             match self.reader.read_event(&mut self.buffer)? {
//                 Event::Start(e) => match e.name() {
//                     TAG_FILELISTS => {
//                         found_metadata_tag = true;
//                         self.num_packages = e
//                             .try_get_attribute("packages")?
//                             .ok_or_else(|| MetadataError::MissingAttributeError("packages"))?
//                             .unescape_and_decode_value(self.reader)?
//                             .parse()?;
//                     }
//                     _ => (),
//                 },
//                 Event::Eof => break,
//                 Event::Decl(_) => (),
//                 _ => break,
//             }
//         }
//         if !found_metadata_tag {
//             return Err(MetadataError::MissingHeaderError)
//         }

//         self.buffer.clear();
//         Ok(())
//     }

//     pub fn read_into_package(&mut self, package: &mut Package) -> Result<(), MetadataError> {
//         loop {
//             match self.reader.read_event(&mut self.buffer)? {
//                 Event::Start(e) => match e.name() {
//                     TAG_PACKAGE => {
//                         found_metadata_tag = true;
//                         self.num_packages = e
//                             .try_get_attribute("packages")?
//                             .ok_or_else(|| MetadataError::MissingAttributeError("packages"))?
//                             .unescape_and_decode_value(self.reader)?
//                             .parse()?;
//                     }
//                     _ => (),
//                 },
//                 Event::Eof => break,
//                 Event::Decl(_) => (),
//                 _ => break,
//             }
//         // TAG_PACKAGE => {
//         //     self.current_element = Some(e)
//         // }
//         parse_package(repository, self.reader, &self.current_package_element);
//         Ok(())
//     }
// }

// <?xml version="1.0" encoding="UTF-8"?>
// <filelists xmlns="http://linux.duke.edu/metadata/filelists" packages="1">
//   <package pkgid="6a915b6e1ad740994aa9688d70a67ff2b6b72e0ced668794aeb27b2d0f2e237b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.el6"/>
//     <file type="dir">/etc/fonts/conf.avail</file>
//     ...
//     <file>/etc/fonts/conf.avail/10-autohint.conf</file>
//   </package>
// </filelists>
fn read_filelists_xml<R: BufRead>(
    repository: &mut Repository,
    reader: &mut Reader<R>,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let mut found_metadata_tag = false;

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => match e.name() {
                TAG_FILELISTS => {
                    found_metadata_tag = true;
                }
                TAG_PACKAGE => {
                    parse_package(repository, reader, &e)?;
                }
                _ => (),
            },
            Event::Eof => break,
            Event::Decl(_) => (), // TOOD
            _ => (),
        }
    }
    if !found_metadata_tag {
        // TODO
    }
    Ok(())
}

//   <package pkgid="a2d3bce512f79b0bc840ca7912a86bbc0016cf06d5c363ffbb6fd5e1ef03de1b" name="fontconfig" arch="x86_64">
//     <version epoch="0" ver="2.8.0" rel="5.fc33"/>
//     <file type="dir">/etc/fonts/conf.avail</file>
//     ...
//     <file>/etc/fonts/conf.avail/10-autohint.conf</file>
//   </package>
pub fn parse_package<R: BufRead>(
    repository: &mut Repository,
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    let pkgid = open_tag
        .try_get_attribute("pkgid")?
        .ok_or_else(|| MetadataError::MissingAttributeError("pkgid"))?
        .unescape_and_decode_value(reader)?;
    let name = open_tag
        .try_get_attribute("name")?
        .ok_or_else(|| MetadataError::MissingAttributeError("name"))?
        .unescape_and_decode_value(reader)?;
    let arch = open_tag
        .try_get_attribute("arch")?
        .ok_or_else(|| MetadataError::MissingAttributeError("arch"))?
        .unescape_and_decode_value(reader)?;

    let package = repository
        .packages_mut()
        .entry(pkgid)
        .or_insert(Package::default()); // TODO

    // TODO: using empty strings as null value is slightly questionable
    if package.name().is_empty() {
        package.set_name(&name);
    }

    if package.arch().is_empty() {
        package.set_arch(&arch);
    }

    loop {
        match reader.read_event(&mut buf)? {
            Event::End(e) if e.name() == TAG_PACKAGE => break,

            Event::Start(e) => match e.name() {
                TAG_VERSION => {
                    package.set_evr(parse_evr(reader, &e)?);
                }
                TAG_FILE => {
                    let file = parse_file(reader, &e)?;
                    // TODO: temporary PackageFile?
                    package.add_file(file.filetype, &file.path);
                }
                _ => (),
            },
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
