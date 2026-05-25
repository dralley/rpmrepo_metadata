// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::borrow::Cow;
use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, name::QName};

use crate::constants::tag::*;
use crate::metadata::{
    Checksum, UpdateCollection, UpdateCollectionModule, UpdateCollectionPackage, UpdateReference,
};
use crate::parsing_utils::{resolve_attr, resolve_text};
use crate::visitor::UpdateinfoVisitor;

use super::metadata::{RpmMetadata, UpdateRecord, UpdateinfoXml};
use super::{MetadataError, Repository};

impl RpmMetadata for UpdateinfoXml {
    fn filename() -> &'static str {
        "updateinfo.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = UpdateinfoXml::new_reader(reader);
        // reader.read_header()?;
        while let Some(updaterecord) = reader.read_update()? {
            repository
                .advisories_mut()
                .insert(updaterecord.id.clone(), updaterecord);
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = UpdateinfoXml::new_writer(writer);
        writer.write_header()?;
        for record in repository.advisories().values() {
            writer.write_updaterecord(record)?;
        }
        writer.finish()?;
        Ok(())
    }
}

/// Streaming writer for updateinfo.xml advisory metadata.
pub struct UpdateinfoXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> UpdateinfoXmlWriter<W> {
    /// Write the XML declaration and opening `<updates>` element.
    pub fn write_header(&mut self) -> Result<(), MetadataError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        // <updates>
        let updates_tag = BytesStart::new(TAG_UPDATES);
        self.writer
            .write_event(Event::Start(updates_tag.borrow()))?;

        Ok(())
    }

    /// Write a single `<update>` element.
    pub fn write_updaterecord(&mut self, record: &UpdateRecord) -> Result<(), MetadataError> {
        write_updaterecord(record, &mut self.writer)
    }

    /// Write the closing `</updates>` element and flush.
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        // </updates>
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_UPDATES)))?;

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

/// Streaming reader for updateinfo.xml advisory metadata.
pub struct UpdateinfoXmlReader<R: BufRead> {
    reader: Reader<R>,
    header_read: bool,
}

impl<R: BufRead> UpdateinfoXmlReader<R> {
    /// Read the next advisory record, or `None` if no more updates.
    pub fn read_update(&mut self) -> Result<Option<UpdateRecord>, MetadataError> {
        if !self.header_read {
            parse_updateinfo_header(&mut self.reader)?;
            self.header_read = true;
        }
        let mut materializer = UpdateRecordMaterializer::new();
        if parse_updateinfo_update(&mut self.reader, &mut materializer)? {
            if let Some(err) = materializer.error {
                return Err(err);
            }
            Ok(materializer.record)
        } else {
            Ok(None)
        }
    }
}

impl<R: BufRead> Iterator for UpdateinfoXmlReader<R> {
    type Item = Result<UpdateRecord, MetadataError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_update().transpose()
    }
}

impl UpdateinfoXml {
    /// Create a new updateinfo.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> UpdateinfoXmlWriter<W> {
        UpdateinfoXmlWriter { writer }
    }

    /// Create a new updateinfo.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> UpdateinfoXmlReader<R> {
        UpdateinfoXmlReader {
            reader,
            header_read: false,
        }
    }
}

struct UpdateRecordMaterializer {
    record: Option<UpdateRecord>,
    current_collection: Option<UpdateCollection>,
    current_package: Option<UpdateCollectionPackage>,
    error: Option<MetadataError>,
}

impl UpdateRecordMaterializer {
    fn new() -> Self {
        UpdateRecordMaterializer {
            record: None,
            current_collection: None,
            current_package: None,
            error: None,
        }
    }
}

impl UpdateinfoVisitor for UpdateRecordMaterializer {
    fn begin_update(&mut self, from: &str, update_type: &str, status: &str, version: &str) {
        let record = UpdateRecord {
            from: from.to_owned(),
            update_type: update_type.to_owned(),
            status: status.to_owned(),
            version: version.to_owned(),
            ..UpdateRecord::default()
        };
        self.record = Some(record);
    }

    fn set_id(&mut self, id: &str) {
        if let Some(record) = self.record.as_mut() {
            record.id = id.to_owned();
        }
    }

    fn set_title(&mut self, title: &str) {
        if let Some(record) = self.record.as_mut() {
            record.title = title.to_owned();
        }
    }

    fn set_issued_date(&mut self, date: &str) {
        if let Some(record) = self.record.as_mut() {
            record.issued_date = Some(date.to_owned());
        }
    }

    fn set_updated_date(&mut self, date: &str) {
        if let Some(record) = self.record.as_mut() {
            record.updated_date = Some(date.to_owned());
        }
    }

    fn set_rights(&mut self, rights: &str) {
        if let Some(record) = self.record.as_mut() {
            record.rights = Some(rights.to_owned());
        }
    }

    fn set_release(&mut self, release: &str) {
        if let Some(record) = self.record.as_mut() {
            record.release = Some(release.to_owned());
        }
    }

    fn set_severity(&mut self, severity: &str) {
        if let Some(record) = self.record.as_mut() {
            record.severity = Some(severity.to_owned());
        }
    }

    fn set_pushcount(&mut self, pushcount: &str) {
        if let Some(record) = self.record.as_mut() {
            record.pushcount = Some(pushcount.to_owned());
        }
    }

    fn set_summary(&mut self, summary: &str) {
        if let Some(record) = self.record.as_mut() {
            record.summary = Some(summary.to_owned());
        }
    }

    fn set_description(&mut self, description: &str) {
        if let Some(record) = self.record.as_mut() {
            record.description = Some(description.to_owned());
        }
    }

    fn set_solution(&mut self, solution: &str) {
        if let Some(record) = self.record.as_mut() {
            record.solution = Some(solution.to_owned());
        }
    }

    fn add_reference(&mut self, href: &str, id: Option<&str>, reftype: &str, title: &str) {
        if let Some(record) = self.record.as_mut() {
            record.references.push(UpdateReference {
                href: href.to_owned(),
                id: id.map(|s| s.to_owned()),
                reftype: reftype.to_owned(),
                title: title.to_owned(),
            });
        }
    }

    fn begin_collection(&mut self, shortname: &str) {
        let collection = UpdateCollection { shortname: shortname.to_owned(), ..Default::default() };
        self.current_collection = Some(collection);
    }

    fn set_collection_name(&mut self, name: &str) {
        if let Some(collection) = self.current_collection.as_mut() {
            collection.name = name.to_owned();
        }
    }

    fn set_collection_module(
        &mut self,
        name: &str,
        stream: &str,
        version: u64,
        context: &str,
        arch: &str,
    ) {
        if let Some(collection) = self.current_collection.as_mut() {
            collection.module = Some(UpdateCollectionModule {
                name: name.to_owned(),
                stream: stream.to_owned(),
                version,
                context: context.to_owned(),
                arch: arch.to_owned(),
            });
        }
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
        let package = UpdateCollectionPackage {
            name: name.to_owned(),
            epoch: epoch.to_owned(),
            version: version.to_owned(),
            release: release.to_owned(),
            arch: arch.to_owned(),
            src: src.map(|s| s.to_owned()),
            ..Default::default()
        };
        self.current_package = Some(package);
    }

    fn set_package_filename(&mut self, filename: &str) {
        if let Some(pkg) = self.current_package.as_mut() {
            pkg.filename = filename.to_owned();
        }
    }

    fn set_package_checksum(&mut self, checksum_type: &str, value: &str) {
        if let Some(pkg) = self.current_package.as_mut() {
            match Checksum::try_create(checksum_type, value) {
                Ok(checksum) => pkg.checksum = Some(checksum),
                Err(e) => self.error = Some(e),
            }
        }
    }

    fn set_package_reboot_suggested(&mut self) {
        if let Some(pkg) = self.current_package.as_mut() {
            pkg.reboot_suggested = true;
        }
    }

    fn set_package_restart_suggested(&mut self) {
        if let Some(pkg) = self.current_package.as_mut() {
            pkg.restart_suggested = true;
        }
    }

    fn set_package_relogin_suggested(&mut self) {
        if let Some(pkg) = self.current_package.as_mut() {
            pkg.relogin_suggested = true;
        }
    }

    fn end_collection_package(&mut self) {
        if let Some(pkg) = self.current_package.take()
            && let Some(collection) = self.current_collection.as_mut()
        {
            collection.packages.push(pkg);
        }
    }

    fn end_collection(&mut self) {
        if let Some(collection) = self.current_collection.take()
            && let Some(record) = self.record.as_mut()
        {
            record.pkglist.push(collection);
        }
    }
}

/// Read past the `<updates>` opening tag in updateinfo.xml.
pub fn parse_updateinfo_header<R: BufRead>(reader: &mut Reader<R>) -> Result<(), MetadataError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name().as_ref() == TAG_UPDATES.as_bytes() => return Ok(()),
            Event::Eof => return Ok(()),
            _ => return Err(MetadataError::MissingHeaderError),
        }
    }
}

/// Parse one `<update>` element from updateinfo.xml, dispatching to `visitor`.
///
/// Returns `true` if an update was parsed, `false` at EOF.
pub fn parse_updateinfo_update<R: BufRead, V: UpdateinfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<bool, MetadataError> {
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_UPDATE.as_bytes() => {
                visitor.end_update();
                return Ok(true);
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_UPDATE => {
                    let mut from_cow = None;
                    let mut type_cow = None;
                    let mut status_cow = None;
                    let mut version_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"from" => from_cow = Some(resolve_attr(&attr)?),
                            b"type" => type_cow = Some(resolve_attr(&attr)?),
                            b"status" => status_cow = Some(resolve_attr(&attr)?),
                            b"version" => version_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let from = from_cow.ok_or(MetadataError::MissingAttributeError("from"))?;
                    let update_type =
                        type_cow.ok_or(MetadataError::MissingAttributeError("type"))?;
                    let status =
                        status_cow.ok_or(MetadataError::MissingAttributeError("status"))?;
                    let version =
                        version_cow.ok_or(MetadataError::MissingAttributeError("version"))?;
                    visitor.begin_update(&from, &update_type, &status, &version);
                }
                TAG_ID => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_id(&text);
                }
                TAG_TITLE => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_TITLE.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_title(&text);
                }
                TAG_ISSUED => {
                    if let Some(attr) = e.try_get_attribute("date")? {
                        let date = resolve_attr(&attr)?;
                        visitor.set_issued_date(&date);
                    }
                    reader.read_to_end_into(QName(TAG_ISSUED.as_bytes()), &mut buf)?;
                }
                TAG_UPDATED => {
                    if let Some(attr) = e.try_get_attribute("date")? {
                        let date = resolve_attr(&attr)?;
                        visitor.set_updated_date(&date);
                    }
                    reader.read_to_end_into(QName(TAG_UPDATED.as_bytes()), &mut buf)?;
                }
                TAG_RIGHTS => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RIGHTS.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_rights(&text);
                    }
                }
                TAG_RELEASE => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_RELEASE.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_release(&text);
                    }
                }
                TAG_SEVERITY => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_SEVERITY.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_severity(&text);
                    }
                }
                TAG_PUSHCOUNT => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_PUSHCOUNT.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_pushcount(&text);
                    }
                }
                TAG_SUMMARY => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_SUMMARY.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_summary(&text);
                    }
                }
                TAG_DESCRIPTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_description(&text);
                    }
                }
                TAG_SOLUTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_SOLUTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    if !text.is_empty() {
                        visitor.set_solution(&text);
                    }
                }
                TAG_REFERENCES => {
                    parse_updateinfo_references(reader, visitor, &mut buf)?;
                }
                TAG_PKGLIST => {
                    parse_updateinfo_pkglist(reader, visitor, &mut buf, &mut text_buf)?;
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

fn parse_updateinfo_references<R: BufRead, V: UpdateinfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::Start(e) if e.name().as_ref() == TAG_REFERENCE.as_bytes() => {
                let mut href_cow = None;
                let mut id_cow = None;
                let mut type_cow = None;
                let mut title_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        b"href" => href_cow = Some(resolve_attr(&attr)?),
                        b"id" => id_cow = Some(resolve_attr(&attr)?),
                        b"type" => type_cow = Some(resolve_attr(&attr)?),
                        b"title" => title_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let href = href_cow.ok_or(MetadataError::MissingAttributeError("href"))?;
                let reftype = type_cow.ok_or(MetadataError::MissingAttributeError("type"))?;
                let title = title_cow.ok_or(MetadataError::MissingAttributeError("title"))?;
                visitor.add_reference(&href, id_cow.as_deref(), &reftype, &title);
            }
            Event::End(e) if e.name().as_ref() == TAG_REFERENCES.as_bytes() => break,
            _ => (),
        }
    }
    Ok(())
}

fn parse_updateinfo_pkglist<R: BufRead, V: UpdateinfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PKGLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_COLLECTION.as_bytes() => {
                let shortname = match e.try_get_attribute("short")? {
                    Some(attr) => resolve_attr(&attr)?,
                    None => Cow::Borrowed(""),
                };
                visitor.begin_collection(&shortname);
            }
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE.as_bytes() => {
                visitor.end_collection_package();
            }
            Event::End(e) if e.name().as_ref() == TAG_COLLECTION.as_bytes() => {
                visitor.end_collection();
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_NAME => {
                    let bytes_text = reader.read_text_into(QName(TAG_NAME.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_collection_name(&text);
                }
                TAG_MODULE => {
                    let mut name_cow = None;
                    let mut stream_cow = None;
                    let mut version_cow = None;
                    let mut context_cow = None;
                    let mut arch_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"name" => name_cow = Some(resolve_attr(&attr)?),
                            b"stream" => stream_cow = Some(resolve_attr(&attr)?),
                            b"version" => version_cow = Some(resolve_attr(&attr)?),
                            b"context" => context_cow = Some(resolve_attr(&attr)?),
                            b"arch" => arch_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                    let stream =
                        stream_cow.ok_or(MetadataError::MissingAttributeError("stream"))?;
                    let version_str =
                        version_cow.ok_or(MetadataError::MissingAttributeError("version"))?;
                    let version: u64 = version_str.parse()?;
                    let context =
                        context_cow.ok_or(MetadataError::MissingAttributeError("context"))?;
                    let arch = arch_cow.ok_or(MetadataError::MissingAttributeError("arch"))?;
                    visitor.set_collection_module(&name, &stream, version, &context, &arch);
                }
                TAG_PACKAGE => {
                    let mut name_cow = None;
                    let mut epoch_cow = None;
                    let mut version_cow = None;
                    let mut release_cow = None;
                    let mut arch_cow = None;
                    let mut src_cow = None;

                    for attr_result in e.attributes() {
                        let attr = attr_result?;
                        match attr.key.as_ref() {
                            b"name" => name_cow = Some(resolve_attr(&attr)?),
                            b"epoch" => epoch_cow = Some(resolve_attr(&attr)?),
                            b"version" => version_cow = Some(resolve_attr(&attr)?),
                            b"release" => release_cow = Some(resolve_attr(&attr)?),
                            b"arch" => arch_cow = Some(resolve_attr(&attr)?),
                            b"src" => src_cow = Some(resolve_attr(&attr)?),
                            _ => (),
                        }
                    }

                    let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                    let epoch = epoch_cow.ok_or(MetadataError::MissingAttributeError("epoch"))?;
                    let version =
                        version_cow.ok_or(MetadataError::MissingAttributeError("version"))?;
                    let release =
                        release_cow.ok_or(MetadataError::MissingAttributeError("release"))?;
                    let arch = arch_cow.ok_or(MetadataError::MissingAttributeError("arch"))?;
                    visitor.begin_collection_package(
                        &name,
                        &epoch,
                        &version,
                        &release,
                        &arch,
                        src_cow.as_deref(),
                    );
                }
                TAG_FILENAME => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_FILENAME.as_bytes()), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_package_filename(&text);
                }
                TAG_SUM => {
                    let type_attr = e
                        .try_get_attribute("type")?
                        .ok_or(MetadataError::MissingAttributeError("type"))?;
                    let checksum_type = resolve_attr(&type_attr)?;
                    let bytes_text = reader.read_text_into(QName(TAG_SUM.as_bytes()), text_buf)?;
                    let value = resolve_text(&bytes_text)?;
                    visitor.set_package_checksum(&checksum_type, &value);
                }
                TAG_REBOOT_SUGGESTED => {
                    let val =
                        reader.read_text_into(QName(TAG_REBOOT_SUGGESTED.as_bytes()), text_buf)?;
                    if val.as_ref() == b"1" || val.as_ref() == b"True" {
                        visitor.set_package_reboot_suggested();
                    }
                }
                TAG_RESTART_SUGGESTED => {
                    let val =
                        reader.read_text_into(QName(TAG_RESTART_SUGGESTED.as_bytes()), text_buf)?;
                    if val.as_ref() == b"1" || val.as_ref() == b"True" {
                        visitor.set_package_restart_suggested();
                    }
                }
                TAG_RELOGIN_SUGGESTED => {
                    let val =
                        reader.read_text_into(QName(TAG_RELOGIN_SUGGESTED.as_bytes()), text_buf)?;
                    if val.as_ref() == b"1" || val.as_ref() == b"True" {
                        visitor.set_package_relogin_suggested();
                    }
                }
                _ => (),
            },
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
    Ok(())
}

fn write_updaterecord<W: Write>(
    record: &UpdateRecord,
    writer: &mut Writer<W>,
) -> Result<(), MetadataError> {
    // <update from="updates@fedoraproject.org" status="stable" type="bugfix" version="2.0">
    let mut updates_tag = BytesStart::new(TAG_UPDATE);
    updates_tag.push_attribute(("status", record.status.as_str()));
    updates_tag.push_attribute(("from", record.from.as_str()));
    updates_tag.push_attribute(("type", record.update_type.as_str()));
    updates_tag.push_attribute(("version", record.version.as_str()));
    writer.write_event(Event::Start(updates_tag.borrow()))?;

    // <id>FEDORA-2020-15f9382449</id>
    writer
        .create_element(TAG_ID)
        .write_text_content(BytesText::new(record.id.as_str()))?;

    // <title>nano-4.9.3-1.fc32</title>
    writer
        .create_element(TAG_TITLE)
        .write_text_content(BytesText::new(record.title.as_str()))?;

    // <issued date="2020-05-27 04:10:31"/>
    if let Some(issued_date) = &record.issued_date {
        writer
            .create_element(TAG_ISSUED)
            .with_attribute(("date", issued_date.as_str()))
            .write_empty()?;
    }

    // <updated date="2021-04-03 00:15:00"/>
    if let Some(updated_date) = &record.updated_date {
        writer
            .create_element(TAG_UPDATED)
            .with_attribute(("date", updated_date.as_str()))
            .write_empty()?;
    }

    if let Some(rights) = &record.rights {
        writer
            .create_element(TAG_RIGHTS)
            .write_text_content(BytesText::new(rights.as_str()))?;
    }

    if let Some(release) = &record.release {
        writer
            .create_element(TAG_RELEASE)
            .write_text_content(BytesText::new(release.as_str()))?;
    }

    if let Some(severity) = &record.severity {
        writer
            .create_element(TAG_SEVERITY)
            .write_text_content(BytesText::new(severity.as_str()))?;
    }

    if let Some(summary) = &record.summary {
        writer
            .create_element(TAG_SUMMARY)
            .write_text_content(BytesText::new(summary.as_str()))?;
    }

    if let Some(description) = &record.description {
        writer
            .create_element(TAG_DESCRIPTION)
            .write_text_content(BytesText::new(description.as_str()))?;
    }

    if let Some(solution) = &record.solution {
        writer
            .create_element(TAG_SOLUTION)
            .write_text_content(BytesText::new(solution.as_str()))?;
    }

    // <pushcount>2</pushcount>
    if let Some(pushcount) = &record.pushcount {
        writer
            .create_element(TAG_PUSHCOUNT)
            .write_text_content(BytesText::new(pushcount.as_str()))?;
    }

    // It's not clear that any metadata actually uses this
    // // <reboot_suggested>True</reboot_suggestion> (optional)
    // if record.reboot_suggested {
    //     writer
    //         .create_element(TAG_REBOOT_SUGGESTED)
    //         .write_text_content(BytesText::new("True"))?;
    // }

    let tag_references = BytesStart::new(TAG_REFERENCES);
    if !record.references.is_empty() {
        // <references>
        writer.write_event(Event::Start(tag_references.borrow()))?;

        for reference in &record.references {
            // <reference href="https://bugzilla.redhat.com/show_bug.cgi?id=1839351" id="1839351" type="bugzilla" title="nano-4.9.3 is available"/>
            let mut elem = writer.create_element(TAG_REFERENCE);
            elem = elem.with_attribute(("href", reference.href.as_str()));
            if let Some(id) = &reference.id {
                elem = elem.with_attribute(("id", id.as_str()));
            }
            elem = elem.with_attribute(("type", reference.reftype.as_str()));
            elem = elem.with_attribute(("title", reference.title.as_str()));
            elem.write_empty()?;
        }

        // </references>
        writer.write_event(Event::End(tag_references.to_end()))?;
    } else {
        // <references/>
        writer.write_event(Event::Empty(tag_references.borrow()))?;
    }

    let tag_pkglist = BytesStart::new(TAG_PKGLIST);
    if !record.pkglist.is_empty() {
        // <pkglist>
        writer.write_event(Event::Start(tag_pkglist.borrow()))?;

        for collection in &record.pkglist {
            // <collection short="F35">
            let mut tag_collection = BytesStart::new(TAG_COLLECTION);
            tag_collection.push_attribute(("short", collection.shortname.as_str()));
            writer.write_event(Event::Start(tag_collection.borrow()))?;

            // <name>Fedora 35</name>
            writer
                .create_element(TAG_NAME)
                .write_text_content(BytesText::new(&collection.name))?;

            // <module stream="3.0" version="8000020190425181943" arch="x86_64" name="freeradius" context="75ec4169" />
            if let Some(module) = &collection.module {
                writer
                    .create_element(TAG_MODULE)
                    .with_attribute(("name", module.name.as_str()))
                    .with_attribute(("stream", module.stream.as_str()))
                    .with_attribute(("version", module.version.to_string().as_str()))
                    .with_attribute(("context", module.context.as_str()))
                    .with_attribute(("arch", module.arch.as_str()))
                    .write_empty()?;
            }

            for package in &collection.packages {
                // <package src="kexec-tools-2.0.4-32.el7_0.1.src.rpm" name="kexec-tools" epoch="0" version="2.0.4" release="32.el7" arch="x86_64">
                let mut package_tag = BytesStart::new(TAG_PACKAGE);
                package_tag.push_attribute(("name", package.name.as_str()));
                package_tag.push_attribute(("version", package.version.as_str()));
                package_tag.push_attribute(("release", package.release.as_str()));
                package_tag.push_attribute(("epoch", package.epoch.to_string().as_str()));
                package_tag.push_attribute(("arch", package.arch.as_str()));
                if let Some(src) = &package.src {
                    package_tag.push_attribute(("src", src.as_str()));
                }
                writer.write_event(Event::Start(package_tag.borrow()))?;

                // <filename>pypy-7.3.6-1.fc35.src.rpm</filename>
                writer
                    .create_element(TAG_FILENAME)
                    .write_text_content(BytesText::new(&package.filename))?;

                // <sum type="sha256">8e214681104e4ba73726e0ce11d21b963ec0390fd70458d439ddc72372082034</sum> (optional)
                if let Some(checksum) = &package.checksum {
                    let (checksum_type, value) = checksum.to_values()?;
                    writer
                        .create_element("sum")
                        .with_attribute(("type", checksum_type))
                        .write_text_content(BytesText::new(value))?;
                }
                if package.reboot_suggested {
                    writer
                        .create_element("reboot_suggested")
                        .write_text_content(BytesText::new("1"))?;
                }
                if package.restart_suggested {
                    writer
                        .create_element("restart_suggested")
                        .write_text_content(BytesText::new("1"))?;
                }
                if package.relogin_suggested {
                    writer
                        .create_element("relogin_suggested")
                        .write_text_content(BytesText::new("1"))?;
                }

                // </package>
                writer.write_event(Event::End(package_tag.to_end()))?;
            }

            // </collection>
            writer.write_event(Event::End(tag_collection.to_end()))?;
        }

        // </pkglist>
        writer.write_event(Event::End(tag_pkglist.to_end()))?;
    } else {
        // <pkglist/>
        writer.write_event(Event::Empty(tag_pkglist.borrow()))?;
    }

    // </update>
    writer.write_event(Event::End(updates_tag.to_end()))?;

    // trailing newline
    writer.write_event(Event::Text(BytesText::new("\n")))?;

    // write everything out to disk - otherwise it won't happen until drop() which impedes debugging
    writer.get_mut().flush()?;

    Ok(())
}
