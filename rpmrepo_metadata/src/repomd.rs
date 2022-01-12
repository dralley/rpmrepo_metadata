use std::convert::{TryFrom, TryInto};
use std::io::{BufRead, Write};
use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use std::time::SystemTime;

// use super::metadata::RpmMetadata;
use quick_xml::events::{BytesDecl, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

use super::metadata::RepomdData;
use super::metadata::{
    Checksum, MetadataError, RepomdRecord, RepomdXml, RpmMetadata, XML_NS_REPO, XML_NS_RPM,
};
use super::Repository;

// RepoMd
const TAG_REPOMD: &[u8] = b"repomd";
const TAG_REVISION: &[u8] = b"revision";
const TAG_TAGS: &[u8] = b"tags";
const TAG_DATA: &[u8] = b"data";
// Tags
const TAG_REPO: &[u8] = b"repo";
const TAG_CONTENT: &[u8] = b"content";
const TAG_DISTRO: &[u8] = b"distro";
// RepoMdRecord
const TAG_LOCATION: &[u8] = b"location";
const TAG_CHECKSUM: &[u8] = b"checksum";
const TAG_OPEN_CHECKSUM: &[u8] = b"open-checksum";
const TAG_HEADER_CHECKSUM: &[u8] = b"header-checksum";
const TAG_TIMESTAMP: &[u8] = b"timestamp";
const TAG_SIZE: &[u8] = b"size";
const TAG_OPEN_SIZE: &[u8] = b"open-size";
const TAG_HEADER_SIZE: &[u8] = b"header-size";
const TAG_DATABASE_VERSION: &[u8] = b"database_version";

impl RpmMetadata for RepomdXml {
    fn filename() -> &'static str {
        "repomd.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        read_repomd_xml(repository.repomd_mut(), reader)?;
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = writer;
        write_repomd_xml(repository.repomd(), &mut writer)?;
        Ok(())
    }
}

impl RepomdXml {
    pub fn write_data<W: Write>(
        repomd_data: &RepomdData,
        writer: &mut Writer<W>,
    ) -> Result<(), MetadataError> {
        write_repomd_xml(repomd_data, writer)
    }

    pub fn read_data<R: BufRead>(reader: Reader<R>) -> Result<RepomdData, MetadataError> {
        let mut repomd = RepomdData::default();
        read_repomd_xml(&mut repomd, reader)?;
        Ok(repomd)
    }
}

#[derive(Debug, PartialEq, Default)]
struct RepomdRecordBuilder {
    metadata_name: String,
    location_href: Option<PathBuf>,
    location_base: Option<String>,
    timestamp: Option<i64>,
    size: Option<u64>,
    checksum: Option<Checksum>,
    open_size: Option<u64>,
    open_checksum: Option<Checksum>,
    header_size: Option<u64>,
    header_checksum: Option<Checksum>,
    database_version: Option<u32>,
}

impl TryFrom<RepomdRecordBuilder> for RepomdRecord {
    type Error = MetadataError;

    fn try_from(builder: RepomdRecordBuilder) -> Result<Self, Self::Error> {
        let mut record = RepomdRecord::default();
        record.metadata_name = builder.metadata_name;
        record.location_href = builder
            .location_href
            .ok_or_else(|| MetadataError::MissingFieldError("location_href"))?;
        record.location_base = builder.location_base;
        record.timestamp = builder
            .timestamp
            .ok_or_else(|| MetadataError::MissingFieldError("timestamp"))?;
        record.size = builder.size;
        record.checksum = builder
            .checksum
            .ok_or_else(|| MetadataError::MissingFieldError("checksum"))?;
        record.open_size = builder.open_size;
        record.open_checksum = builder.open_checksum; // TODO: do these need to be conditionally required?
        record.header_size = builder.header_size;
        record.header_checksum = builder.header_checksum;
        record.database_version = builder.database_version;

        Ok(record)
    }
}

// struct RepomdXmlWriter<'a, W: Write> {
//     repository: &'a mut Repository,
//     writer: Writer<W>
// }

fn read_repomd_xml<R: BufRead>(
    repomd_data: &mut RepomdData,
    reader: Reader<R>,
) -> Result<(), MetadataError> {
    let mut reader = reader;
    let mut event_buf = Vec::new();
    let mut text_buf = Vec::new();

    let mut found_metadata_tag = false;

    loop {
        match reader.read_event(&mut event_buf)? {
            Event::Start(e) => match e.name() {
                TAG_REPOMD => {
                    found_metadata_tag = true;
                }
                TAG_REVISION => {
                    let revision = reader.read_text(e.name(), &mut text_buf)?;
                    repomd_data.set_revision(&revision);
                }
                TAG_DATA => {
                    let data = parse_repomdrecord(&mut reader, &e)?;
                    repomd_data.add_record(data);
                }
                TAG_TAGS => {
                    //   <tags>
                    //     <repo>Fedora</repo>
                    //     <content>binary-x86_64</content>
                    //     <distro cpeid="cpe:/o:fedoraproject:fedora:33">Fedora 33</distro>
                    //   </tags>
                    loop {
                        match reader.read_event(&mut event_buf)? {
                            Event::Start(e) => match e.name() {
                                TAG_DISTRO => {
                                    let cpeid = (&e).try_get_attribute("cpeid")?.and_then(|a| {
                                        a.unescape_and_decode_value(&mut reader).ok()
                                    });
                                    let name = reader.read_text(TAG_DISTRO, &mut text_buf)?;
                                    repomd_data.add_distro_tag(name, cpeid);
                                }
                                TAG_REPO => {
                                    let repo = reader.read_text(e.name(), &mut text_buf)?;
                                    repomd_data.add_repo_tag(repo);
                                }
                                TAG_CONTENT => {
                                    let content = reader.read_text(e.name(), &mut text_buf)?;
                                    repomd_data.add_content_tag(content);
                                }
                                _ => (),
                            },

                            Event::End(e) if e.name() == TAG_TAGS => break,
                            _ => (),
                        }
                        text_buf.clear();
                    }
                }
                _ => (),
            },
            Event::Eof => break,
            Event::Decl(_) => (),
            _ => (),
        }
        text_buf.clear()
    }
    if !found_metadata_tag {
        // TODO
    }
    Ok(())
}

// <data type="other_db">
//     <checksum type="sha256">fd2ff685b13d5b18b7c16d1316f7ccf299283cdf5db27ab780cb6b855b022000</checksum>
//     <open-checksum type="sha256">fd0619cc82de1a6475c98bd11cdd09e38b359c57a3ef1ab8411e5cc6076cbab8</open-checksum>
//     <location href="repodata/fd2ff685b13d5b18b7c16d1316f7ccf299283cdf5db27ab780cb6b855b022000-other.sqlite.xz"/>
//     <timestamp>1602869947</timestamp>
//     <database_version>10</database_version>
//     <size>78112</size>
//     <open-size>651264</open-size>
// </data>
pub fn parse_repomdrecord<R: BufRead>(
    reader: &mut Reader<R>,
    open_tag: &BytesStart,
) -> Result<RepomdRecord, MetadataError> {
    let mut record_builder = RepomdRecordBuilder::default();

    let record_type = open_tag
        .try_get_attribute("type")?
        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
        .value
        .iter()
        .cloned()
        .collect();
    record_builder.metadata_name = String::from_utf8(record_type).map_err(|e| e.utf8_error())?; // TODO weird conversion

    let mut buf = Vec::new();
    let mut record_buf = Vec::new();

    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(e) => match e.name() {
                TAG_CHECKSUM => {
                    let checksum_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?;
                    let checksum_value = reader.read_text(e.name(), &mut record_buf)?;
                    let checksum = Checksum::try_create(
                        checksum_type.value.as_ref(),
                        checksum_value.as_bytes(),
                    )?;
                    record_builder.checksum = Some(checksum);
                }
                TAG_OPEN_CHECKSUM => {
                    let checksum_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?;
                    let checksum_value = reader.read_text(e.name(), &mut record_buf)?;
                    let checksum = Checksum::try_create(
                        checksum_type.value.as_ref(),
                        checksum_value.as_bytes(),
                    )?;
                    record_builder.open_checksum = Some(checksum);
                }
                TAG_HEADER_CHECKSUM => {
                    let checksum_type = e
                        .try_get_attribute("type")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("type"))?;
                    let checksum_value = reader.read_text(e.name(), &mut record_buf)?;
                    let checksum = Checksum::try_create(
                        checksum_type.value.as_ref(),
                        checksum_value.as_bytes(),
                    )?;
                    record_builder.header_checksum = Some(checksum);
                }
                TAG_LOCATION => {
                    let location = e
                        .try_get_attribute("href")?
                        .ok_or_else(|| MetadataError::MissingAttributeError("href"))?
                        .unescape_and_decode_value(reader)?
                        .into();
                    record_builder.location_href = Some(location);
                }
                TAG_TIMESTAMP => {
                    let timestamp = reader.read_text(e.name(), &mut record_buf)?.parse()?;
                    record_builder.timestamp = Some(timestamp);
                }
                TAG_SIZE => {
                    let size = reader.read_text(e.name(), &mut record_buf)?.parse()?;
                    record_builder.size = Some(size);
                }
                TAG_HEADER_SIZE => {
                    let header_size = reader.read_text(e.name(), &mut record_buf)?.parse()?;
                    record_builder.header_size = Some(header_size);
                }
                TAG_OPEN_SIZE => {
                    let open_size = reader.read_text(e.name(), &mut record_buf)?.parse()?;
                    record_builder.open_size = Some(open_size);
                }
                TAG_DATABASE_VERSION => {
                    let database_version = reader.read_text(e.name(), &mut record_buf)?.parse()?;
                    record_builder.database_version = Some(database_version);
                }
                _ => (),
            },
            Event::End(e) if e.name() == TAG_DATA => break,
            _ => (),
        }
        record_buf.clear();
    }
    Ok(record_builder.try_into()?)
}

fn write_repomd_xml<W: Write>(
    repomd_data: &RepomdData,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <?xml version="1.0" encoding="UTF-8"?>
    writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"UTF-8"), None)))?;

    // <repomd xmlns="http://linux.duke.edu/metadata/repo" xmlns:rpm="http://linux.duke.edu/metadata/rpm">
    let mut repomd_tag = BytesStart::borrowed_name(TAG_REPOMD);
    repomd_tag.push_attribute(("xmlns", XML_NS_REPO));
    repomd_tag.push_attribute(("xmlns:rpm", XML_NS_RPM));
    writer.write_event(Event::Start(repomd_tag.to_borrowed()))?;

    // <revision>123897</revision>
    let get_current_time = || {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock failure")
            .as_secs()
            .to_string()
    };
    let revision = if let Some(revision) = repomd_data.revision() {
        revision.to_owned()
    } else {
        get_current_time()
    };
    writer
        .create_element(TAG_REVISION)
        .write_text_content(BytesText::from_plain_str(revision.as_str()))?;

    write_tags(repomd_data, writer)?;
    for record in repomd_data.records() {
        write_data(record, writer)?;
    }

    // </repomd>
    writer.write_event(Event::End(repomd_tag.to_end()))?;

    // trailing newline
    writer.write_event(Event::Text(BytesText::from_plain_str("\n")))?;
    Ok(())
}

/// <tags>
///   <repo>Fedora</repo>
///   <distro cpeid="cpe:/o:fedoraproject:fedora:33">Fedora 33</distro>
///   <content>binary-x86_64</content>
//// </tags>
fn write_tags<W: Write>(
    repomd_data: &RepomdData,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    let has_distro_tags = !repomd_data.distro_tags().is_empty();
    let has_repo_tags = !repomd_data.repo_tags().is_empty();
    let has_content_tags = !repomd_data.content_tags().is_empty();

    if has_distro_tags || has_repo_tags || has_content_tags {
        // <tags>
        let tags_tag = BytesStart::borrowed_name(TAG_TAGS);
        writer.write_event(Event::Start(tags_tag.to_borrowed()))?;

        for item in repomd_data.content_tags() {
            // <content>binary-x86_64</content>
            writer
                .create_element(TAG_CONTENT)
                .write_text_content(BytesText::from_plain_str(item))?;
        }

        for item in repomd_data.repo_tags() {
            // <repo>Fedora</repo>
            writer
                .create_element(TAG_REPO)
                .write_text_content(BytesText::from_plain_str(item))?;
        }

        for item in repomd_data.distro_tags() {
            // <distro cpeid="cpe:/o:fedoraproject:fedora:33">Fedora 33</distro>
            let mut distro_tag = BytesStart::borrowed_name(TAG_DISTRO);
            if let Some(cpeid) = &item.cpeid {
                distro_tag.push_attribute(("cpeid", cpeid.as_str()))
            }
            writer.write_event(Event::Start(distro_tag.to_borrowed()))?;
            writer.write_event(Event::Text(BytesText::from_plain_str(item.name.as_str())))?;
            writer.write_event(Event::End(distro_tag.to_end()))?;
        }

        // </tags>
        writer.write_event(Event::End(tags_tag.to_end()))?;
    }

    Ok(())
}

///   <data type="primary">
///    .....
///    <timestamp>1614969700</timestamp>
///    <size>5830735</size>
///    <open-size>53965949</open-size>
///  </data>
fn write_data<W: Write>(data: &RepomdRecord, writer: &mut Writer<W>) -> Result<(), MetadataError> {
    // <data>
    let mut data_tag = BytesStart::borrowed_name(TAG_DATA);
    data_tag.push_attribute(("type".as_bytes(), data.metadata_name.as_bytes()));
    writer.write_event(Event::Start(data_tag.to_borrowed()))?;

    // <checksum type="sha256">afdc6dc379e58d097ed0b350536812bc6a604bbce50c5c109d8d98e28301dc4b</checksum>
    let (checksum_type, checksum_value) = data.checksum.to_values()?;
    writer
        .create_element(TAG_CHECKSUM)
        .with_attribute(("type", checksum_type))
        .write_text_content(BytesText::from_plain_str(checksum_value))?;

    // <open-checksum type="sha256">afdc6dc379e58d097ed0b350536812bc6a604bbce50c5c109d8d98e28301dc4b</open-checksum> (maybe)
    if let Some(open_checksum) = &data.open_checksum {
        let (checksum_type, checksum_value) = open_checksum.to_values()?;
        writer
            .create_element(TAG_OPEN_CHECKSUM)
            .with_attribute(("type", checksum_type))
            .write_text_content(BytesText::from_plain_str(checksum_value))?;
    }

    // <header-checksum type="sha256">afdc6dc379e58d097ed0b350536812bc6a604bbce50c5c109d8d98e28301dc4b</header-checksum> (maybe)
    if let Some(header_checksum) = &data.header_checksum {
        let (checksum_type, checksum_value) = header_checksum.to_values()?;
        writer
            .create_element(TAG_HEADER_CHECKSUM)
            .with_attribute(("type", checksum_type))
            .write_text_content(BytesText::from_plain_str(checksum_value))?;
    }

    // <location href="repodata/primary.xml.gz">
    writer
        .create_element(TAG_LOCATION)
        .with_attribute(("href".as_bytes(), data.location_href.as_os_str().as_bytes()))
        .write_empty()?;

    // <timestamp>1602869947</timestamp>
    writer
        .create_element(TAG_TIMESTAMP)
        .write_text_content(BytesText::from_plain_str(
            data.timestamp.to_string().as_str(),
        ))?;

    // <size>123987</size> (maybe)
    if let Some(size) = data.size {
        writer
            .create_element(TAG_SIZE)
            .write_text_content(BytesText::from_plain_str(&size.to_string()))?;
    }

    // <open-size>68652</open-size> (maybe)
    if let Some(open_size) = data.open_size {
        writer
            .create_element(TAG_OPEN_SIZE)
            .write_text_content(BytesText::from_plain_str(&open_size.to_string()))?;
    }

    // <header-size>761487</header-size> (maybe)
    if let Some(size_header) = data.header_size {
        writer
            .create_element(TAG_HEADER_SIZE)
            .write_text_content(BytesText::from_plain_str(&size_header.to_string()))?;
    }

    // <database_version>10</database_version>
    if let Some(database_version) = data.database_version {
        writer
            .create_element(TAG_DATABASE_VERSION)
            .write_text_content(BytesText::from_plain_str(&database_version.to_string()))?;
    }

    // </data>
    writer.write_event(Event::End(data_tag.to_end()))?;

    Ok(())
}
