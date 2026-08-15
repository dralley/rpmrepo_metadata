// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};

use crate::MetadataError;
use crate::constants::tag::*;
use crate::metadata::{
    SupportInfo, SupportInfoData, SupportInfoLevel, SupportInfoLifecycle, SupportInfoMilestone,
    SupportInfoNote, SupportInfoPackage, SupportInfoPackageClass, SupportInfoPackageOrigin,
    SupportInfoPhase, SupportInfoXml,
};
use crate::parsing_utils::{resolve_attr, resolve_text};
use crate::visitor::SupportInfoVisitor;

impl SupportInfoXml {
    /// Create a new V1.0 support_info.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> SupportInfoXmlWriter<W> {
        SupportInfoXmlWriter {
            writer,
            current_as: String::new(),
        }
    }

    /// Create a new support_info.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> SupportInfoXmlReader<R> {
        SupportInfoXmlReader {
            reader,
            done: false,
        }
    }
}

// ---------- Writer ----------------------------------------

/// Streaming writer for V1.0 support_info.xml metadata.
pub struct SupportInfoXmlWriter<W: Write> {
    writer: Writer<W>,
    current_as: String,
}

impl<W: Write> SupportInfoXmlWriter<W> {
    /// Write the XML declaration and opening `<package_support>` element.
    pub fn write_header(&mut self, current_as: &str) -> Result<(), MetadataError> {
        self.current_as = current_as.to_owned();

        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        let mut tag = BytesStart::new(TAG_PACKAGE_SUPPORT);
        tag.push_attribute(("schema_version", "1.0"));
        tag.push_attribute(("current_as", current_as));
        self.writer.write_event(Event::Start(tag))?;

        Ok(())
    }

    /// Write all inner sections of a V1.0 support info document.
    pub fn write_support_info(&mut self, data: &SupportInfoData) -> Result<(), MetadataError> {
        self.write_lifecycles(&data.lifecycles)?;
        self.write_milestones(&data.milestones)?;
        self.write_support_levels(&data.support_levels)?;
        self.write_packages(&data.packages)?;
        self.write_package_classes(&data.package_classes)?;
        self.write_package_origins(&data.package_origins)?;
        self.write_notes(&data.notes)?;
        Ok(())
    }

    /// Write the closing `</package_support>` element and flush.
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_PACKAGE_SUPPORT)))?;
        self.writer.write_event(Event::Text(BytesText::new("\n")))?;
        self.writer.get_mut().flush()?;
        Ok(())
    }

    /// Consume the writer and return the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }

    fn write_lifecycles(
        &mut self,
        lifecycles: &[SupportInfoLifecycle],
    ) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_LIFECYCLES);
        if lifecycles.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for lc in lifecycles {
            let mut lc_tag = BytesStart::new(TAG_LIFECYCLE);
            lc_tag.push_attribute(("name", lc.name.as_str()));
            if let Some(note) = &lc.note {
                lc_tag.push_attribute(("note", note.as_str()));
            }
            if let Some(dn) = &lc.display_name {
                lc_tag.push_attribute(("display_name", dn.as_str()));
            }
            if let Some(desc) = &lc.description {
                lc_tag.push_attribute(("description", desc.as_str()));
            }
            self.writer.write_event(Event::Start(lc_tag.borrow()))?;

            for phase in &lc.phases {
                let mut p_tag = BytesStart::new(TAG_PHASE);
                p_tag.push_attribute(("name", phase.name.as_str()));
                p_tag.push_attribute(("support_level", phase.support_level.as_str()));
                if let Some(date) = &phase.start_date {
                    p_tag.push_attribute(("start_date", date.as_str()));
                }
                if let Some(ms) = &phase.start_milestone {
                    p_tag.push_attribute(("start_milestone", ms.as_str()));
                }
                if let Some(dn) = &phase.display_name {
                    p_tag.push_attribute(("display_name", dn.as_str()));
                }
                self.writer.write_event(Event::Empty(p_tag))?;
            }

            self.writer.write_event(Event::End(lc_tag.to_end()))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_LIFECYCLES)))?;
        Ok(())
    }

    fn write_milestones(
        &mut self,
        milestones: &[SupportInfoMilestone],
    ) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_SUPPORT_MILESTONES);
        if milestones.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for ms in milestones {
            let mut ms_tag = BytesStart::new(TAG_MILESTONE);
            ms_tag.push_attribute(("name", ms.name.as_str()));
            ms_tag.push_attribute(("date", ms.date.as_str()));
            if let Some(dn) = &ms.display_name {
                ms_tag.push_attribute(("display_name", dn.as_str()));
            }
            if let Some(desc) = &ms.description {
                ms_tag.push_attribute(("description", desc.as_str()));
            }
            self.writer.write_event(Event::Empty(ms_tag))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_SUPPORT_MILESTONES)))?;
        Ok(())
    }

    fn write_support_levels(&mut self, levels: &[SupportInfoLevel]) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_SUPPORT_LEVELS);
        if levels.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for level in levels {
            let mut l_tag = BytesStart::new(TAG_SUPPORT_LEVEL);
            l_tag.push_attribute(("name", level.name.as_str()));
            l_tag.push_attribute(("severities", level.severities.as_str()));
            if let Some(desc) = &level.description {
                l_tag.push_attribute(("description", desc.as_str()));
            }
            if let Some(dn) = &level.display_name {
                l_tag.push_attribute(("display_name", dn.as_str()));
            }
            self.writer.write_event(Event::Empty(l_tag))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_SUPPORT_LEVELS)))?;
        Ok(())
    }

    fn write_packages(&mut self, packages: &[SupportInfoPackage]) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_PACKAGES);
        if packages.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for pkg in packages {
            let mut p_tag = BytesStart::new(TAG_PACKAGE);
            p_tag.push_attribute(("name", pkg.name.as_str()));
            p_tag.push_attribute(("lifecycle", pkg.lifecycle.as_str()));
            if let Some(cls) = &pkg.package_class {
                p_tag.push_attribute(("package_class", cls.as_str()));
            }
            p_tag.push_attribute(("origin", pkg.origin.as_str()));
            self.writer.write_event(Event::Empty(p_tag))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_PACKAGES)))?;
        Ok(())
    }

    fn write_package_classes(
        &mut self,
        classes: &[SupportInfoPackageClass],
    ) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_PACKAGE_CLASSES);
        if classes.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for cls in classes {
            let mut c_tag = BytesStart::new(TAG_PACKAGE_CLASS);
            c_tag.push_attribute(("name", cls.name.as_str()));
            self.writer.write_event(Event::Start(c_tag.borrow()))?;

            self.writer
                .create_element(TAG_SUMMARY)
                .write_text_content(BytesText::new(&cls.summary))?;
            self.writer
                .create_element(TAG_TEXT)
                .write_text_content(BytesText::new(&cls.text))?;

            self.writer.write_event(Event::End(c_tag.to_end()))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_PACKAGE_CLASSES)))?;
        Ok(())
    }

    fn write_package_origins(
        &mut self,
        origins: &[SupportInfoPackageOrigin],
    ) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_PACKAGE_ORIGINS);
        if origins.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for origin in origins {
            let mut o_tag = BytesStart::new(TAG_PACKAGE_ORIGIN);
            o_tag.push_attribute(("name", origin.name.as_str()));
            o_tag.push_attribute(("repo_id", origin.repo_id.as_str()));
            o_tag.push_attribute(("dist", origin.dist.as_str()));
            o_tag.push_attribute(("vendor", origin.vendor.as_str()));
            if let Some(key) = &origin.signing_key {
                o_tag.push_attribute(("signing_key", key.as_str()));
            }
            if let Some(dn) = &origin.display_name {
                o_tag.push_attribute(("display_name", dn.as_str()));
            }
            if let Some(desc) = &origin.description {
                o_tag.push_attribute(("description", desc.as_str()));
            }
            self.writer.write_event(Event::Empty(o_tag))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_PACKAGE_ORIGINS)))?;
        Ok(())
    }

    fn write_notes(&mut self, notes: &[SupportInfoNote]) -> Result<(), MetadataError> {
        let tag = BytesStart::new(TAG_NOTES);
        if notes.is_empty() {
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        self.writer.write_event(Event::Start(tag.borrow()))?;

        for note in notes {
            self.writer
                .create_element(TAG_NOTE)
                .with_attribute(("name", note.name.as_str()))
                .write_text_content(BytesText::new(&note.content))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_NOTES)))?;
        Ok(())
    }
}

// ── Reader ─────────────────────────────────────────────────────────────────

/// Streaming reader for support_info.xml, supporting the V1.0 format.
pub struct SupportInfoXmlReader<R: BufRead> {
    reader: Reader<R>,
    done: bool,
}

impl<R: BufRead> SupportInfoXmlReader<R> {
    /// Read the support info document. Returns `None` at EOF.
    pub fn read(&mut self) -> Result<Option<SupportInfo>, MetadataError> {
        if self.done {
            return Ok(None);
        }

        let header = parse_supportinfo_header(&mut self.reader)?;
        let (current_as, is_v1) = match header {
            Some(h) => h,
            None => return Ok(None),
        };

        self.done = true;

        if is_v1 {
            let mut materializer = SupportInfoV1Materializer::new(current_as);
            parse_supportinfo_v1_body(&mut self.reader, &mut materializer)?;
            if let Some(err) = materializer.error {
                return Err(err);
            }
            Ok(Some(SupportInfo::V1(materializer.data)))
        } else {
            unimplemented!("Legacy format unsupported");
        }
    }
}

// ----------- Header parsing ------------------------------------------

/// Parse the `<package_support>` opening tag.
///
/// Returns `Some((current_as, is_v1))` on success, `None` at EOF.
pub fn parse_supportinfo_header<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Option<(String, bool)>, MetadataError> {
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Decl(_) => (),
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGE_SUPPORT => {
                let mut current_as_cow = None;
                let mut schema_version_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "current_as" => current_as_cow = Some(resolve_attr(&attr)?),
                        "schema_version" => schema_version_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let current_as = current_as_cow
                    .ok_or(MetadataError::MissingAttributeError("current_as"))?
                    .into_owned();
                let is_v1 = schema_version_cow.is_some();

                return Ok(Some((current_as, is_v1)));
            }
            Event::Eof => return Ok(None),
            _ => (),
        }
        buf.clear();
    }
}

// -------------- V1.0 body parsing ------------------------------------

/// Parse the body of a V1.0 support info document, dispatching to `visitor`.
pub fn parse_supportinfo_v1_body<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE_SUPPORT => {
                return Ok(());
            }
            Event::Start(e) => match e.name().as_ref() {
                TAG_LIFECYCLES => {
                    parse_lifecycles(reader, visitor, &mut buf)?;
                }
                TAG_SUPPORT_MILESTONES => {
                    parse_milestones(reader, visitor, &mut buf)?;
                }
                TAG_SUPPORT_LEVELS => {
                    parse_support_levels(reader, visitor, &mut buf)?;
                }
                TAG_PACKAGES => {
                    parse_v1_packages(reader, visitor, &mut buf)?;
                }
                TAG_PACKAGE_CLASSES => {
                    parse_package_classes(reader, visitor, &mut buf, &mut text_buf)?;
                }
                TAG_PACKAGE_ORIGINS => {
                    parse_package_origins(reader, visitor, &mut buf)?;
                }
                TAG_NOTES => {
                    parse_v1_notes(reader, visitor, &mut buf, &mut text_buf)?;
                }
                _ => (),
            },
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

fn parse_lifecycles<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_LIFECYCLES => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_LIFECYCLE => {
                let mut name_cow = None;
                let mut note_cow = None;
                let mut display_name_cow = None;
                let mut description_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "note" => note_cow = Some(resolve_attr(&attr)?),
                        "display_name" => display_name_cow = Some(resolve_attr(&attr)?),
                        "description" => description_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                visitor.begin_lifecycle(
                    &name,
                    note_cow.as_deref(),
                    display_name_cow.as_deref(),
                    description_cow.as_deref(),
                );

                parse_lifecycle_phases(reader, visitor, buf)?;
                visitor.end_lifecycle();
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_lifecycle_phases<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_LIFECYCLE => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_PHASE => {
                let mut name_cow = None;
                let mut support_level_cow = None;
                let mut start_date_cow = None;
                let mut start_milestone_cow = None;
                let mut display_name_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "support_level" => support_level_cow = Some(resolve_attr(&attr)?),
                        "start_date" => start_date_cow = Some(resolve_attr(&attr)?),
                        "start_milestone" => start_milestone_cow = Some(resolve_attr(&attr)?),
                        "display_name" => display_name_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let support_level = support_level_cow
                    .ok_or(MetadataError::MissingAttributeError("support_level"))?;
                visitor.add_phase(
                    &name,
                    &support_level,
                    start_date_cow.as_deref(),
                    start_milestone_cow.as_deref(),
                    display_name_cow.as_deref(),
                );
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_milestones<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_SUPPORT_MILESTONES => {
                return Ok(());
            }
            Event::Start(e) if e.name().as_ref() == TAG_MILESTONE => {
                let mut name_cow = None;
                let mut date_cow = None;
                let mut display_name_cow = None;
                let mut description_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "date" => date_cow = Some(resolve_attr(&attr)?),
                        "display_name" => display_name_cow = Some(resolve_attr(&attr)?),
                        "description" => description_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let date = date_cow.ok_or(MetadataError::MissingAttributeError("date"))?;
                visitor.add_milestone(
                    &name,
                    &date,
                    display_name_cow.as_deref(),
                    description_cow.as_deref(),
                );
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_support_levels<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_SUPPORT_LEVELS => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_SUPPORT_LEVEL => {
                let mut name_cow = None;
                let mut severities_cow = None;
                let mut description_cow = None;
                let mut display_name_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "severities" => severities_cow = Some(resolve_attr(&attr)?),
                        "description" => description_cow = Some(resolve_attr(&attr)?),
                        "display_name" => display_name_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let severities =
                    severities_cow.ok_or(MetadataError::MissingAttributeError("severities"))?;
                visitor.add_support_level(
                    &name,
                    &severities,
                    description_cow.as_deref(),
                    display_name_cow.as_deref(),
                );
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_v1_packages<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGES => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGE => {
                let mut name_cow = None;
                let mut lifecycle_cow = None;
                let mut origin_cow = None;
                let mut package_class_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "lifecycle" => lifecycle_cow = Some(resolve_attr(&attr)?),
                        "origin" => origin_cow = Some(resolve_attr(&attr)?),
                        "package_class" => package_class_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let lifecycle =
                    lifecycle_cow.ok_or(MetadataError::MissingAttributeError("lifecycle"))?;
                let origin = origin_cow.ok_or(MetadataError::MissingAttributeError("origin"))?;
                visitor.add_package(&name, &lifecycle, &origin, package_class_cow.as_deref());
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_package_classes<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE_CLASSES => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGE_CLASS => {
                let mut name_cow = None;
                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    if attr.key.as_ref() == "name" {
                        name_cow = Some(resolve_attr(&attr)?);
                    }
                }
                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                visitor.begin_package_class(&name);

                parse_package_class_body(reader, visitor, buf, text_buf)?;
                visitor.end_package_class();
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_package_class_body<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE_CLASS => return Ok(()),
            Event::Start(e) => match e.name().as_ref() {
                TAG_SUMMARY => {
                    let bytes_text = reader.read_text_into(QName(TAG_SUMMARY), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_package_class_summary(&text);
                }
                TAG_TEXT => {
                    let bytes_text = reader.read_text_into(QName(TAG_TEXT), text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_package_class_text(&text);
                }
                _ => (),
            },
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

fn parse_package_origins<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGE_ORIGINS => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGE_ORIGIN => {
                let mut name_cow = None;
                let mut repo_id_cow = None;
                let mut dist_cow = None;
                let mut vendor_cow = None;
                let mut signing_key_cow = None;
                let mut display_name_cow = None;
                let mut description_cow = None;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        "name" => name_cow = Some(resolve_attr(&attr)?),
                        "repo_id" => repo_id_cow = Some(resolve_attr(&attr)?),
                        "dist" => dist_cow = Some(resolve_attr(&attr)?),
                        "vendor" => vendor_cow = Some(resolve_attr(&attr)?),
                        "signing_key" => signing_key_cow = Some(resolve_attr(&attr)?),
                        "display_name" => display_name_cow = Some(resolve_attr(&attr)?),
                        "description" => description_cow = Some(resolve_attr(&attr)?),
                        _ => (),
                    }
                }

                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let repo_id = repo_id_cow.ok_or(MetadataError::MissingAttributeError("repo_id"))?;
                let dist = dist_cow.ok_or(MetadataError::MissingAttributeError("dist"))?;
                let vendor = vendor_cow.ok_or(MetadataError::MissingAttributeError("vendor"))?;
                visitor.add_package_origin(
                    &name,
                    &repo_id,
                    &dist,
                    &vendor,
                    signing_key_cow.as_deref(),
                    display_name_cow.as_deref(),
                    description_cow.as_deref(),
                );
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_v1_notes<R: BufRead, V: SupportInfoVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_NOTES => return Ok(()),
            Event::Start(e) if e.name().as_ref() == TAG_NOTE => {
                let mut name_cow = None;
                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    if attr.key.as_ref() == "name" {
                        name_cow = Some(resolve_attr(&attr)?);
                    }
                }
                let name = name_cow.ok_or(MetadataError::MissingAttributeError("name"))?;
                let bytes_text = reader.read_text_into(QName(TAG_NOTE), text_buf)?;
                let content = resolve_text(&bytes_text)?;
                visitor.add_note(&name, &content);
            }
            Event::Eof => return Ok(()),
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

// ----------- V1.0 Materializer ----------------------------------------

struct SupportInfoV1Materializer {
    data: SupportInfoData,
    current_lifecycle: Option<SupportInfoLifecycle>,
    current_package_class: Option<SupportInfoPackageClass>,
    error: Option<MetadataError>,
}

impl SupportInfoV1Materializer {
    fn new(current_as: String) -> Self {
        SupportInfoV1Materializer {
            data: SupportInfoData {
                current_as,
                ..Default::default()
            },
            current_lifecycle: None,
            current_package_class: None,
            error: None,
        }
    }
}

impl SupportInfoVisitor for SupportInfoV1Materializer {
    fn begin_lifecycle(
        &mut self,
        name: &str,
        note: Option<&str>,
        display_name: Option<&str>,
        description: Option<&str>,
    ) {
        self.current_lifecycle = Some(SupportInfoLifecycle {
            name: name.to_owned(),
            note: note.map(|s| s.to_owned()),
            display_name: display_name.map(|s| s.to_owned()),
            description: description.map(|s| s.to_owned()),
            phases: Vec::new(),
        });
    }

    fn add_phase(
        &mut self,
        name: &str,
        support_level: &str,
        start_date: Option<&str>,
        start_milestone: Option<&str>,
        display_name: Option<&str>,
    ) {
        if let Some(lc) = self.current_lifecycle.as_mut() {
            lc.phases.push(SupportInfoPhase {
                name: name.to_owned(),
                support_level: support_level.to_owned(),
                start_date: start_date.map(|s| s.to_owned()),
                start_milestone: start_milestone.map(|s| s.to_owned()),
                display_name: display_name.map(|s| s.to_owned()),
            });
        }
    }

    fn end_lifecycle(&mut self) {
        if let Some(lc) = self.current_lifecycle.take() {
            self.data.lifecycles.push(lc);
        }
    }

    fn add_milestone(
        &mut self,
        name: &str,
        date: &str,
        display_name: Option<&str>,
        description: Option<&str>,
    ) {
        self.data.milestones.push(SupportInfoMilestone {
            name: name.to_owned(),
            date: date.to_owned(),
            display_name: display_name.map(|s| s.to_owned()),
            description: description.map(|s| s.to_owned()),
        });
    }

    fn add_support_level(
        &mut self,
        name: &str,
        severities: &str,
        description: Option<&str>,
        display_name: Option<&str>,
    ) {
        self.data.support_levels.push(SupportInfoLevel {
            name: name.to_owned(),
            severities: severities.to_owned(),
            description: description.map(|s| s.to_owned()),
            display_name: display_name.map(|s| s.to_owned()),
        });
    }

    fn add_package(
        &mut self,
        name: &str,
        lifecycle: &str,
        origin: &str,
        package_class: Option<&str>,
    ) {
        self.data.packages.push(SupportInfoPackage {
            name: name.to_owned(),
            lifecycle: lifecycle.to_owned(),
            origin: origin.to_owned(),
            package_class: package_class.map(|s| s.to_owned()),
        });
    }

    fn begin_package_class(&mut self, name: &str) {
        self.current_package_class = Some(SupportInfoPackageClass {
            name: name.to_owned(),
            ..Default::default()
        });
    }

    fn set_package_class_summary(&mut self, summary: &str) {
        if let Some(cls) = self.current_package_class.as_mut() {
            cls.summary = summary.to_owned();
        }
    }

    fn set_package_class_text(&mut self, text: &str) {
        if let Some(cls) = self.current_package_class.as_mut() {
            cls.text = text.to_owned();
        }
    }

    fn end_package_class(&mut self) {
        if let Some(cls) = self.current_package_class.take() {
            self.data.package_classes.push(cls);
        }
    }

    fn add_package_origin(
        &mut self,
        name: &str,
        repo_id: &str,
        dist: &str,
        vendor: &str,
        signing_key: Option<&str>,
        display_name: Option<&str>,
        description: Option<&str>,
    ) {
        self.data.package_origins.push(SupportInfoPackageOrigin {
            name: name.to_owned(),
            repo_id: repo_id.to_owned(),
            dist: dist.to_owned(),
            vendor: vendor.to_owned(),
            signing_key: signing_key.map(|s| s.to_owned()),
            display_name: display_name.map(|s| s.to_owned()),
            description: description.map(|s| s.to_owned()),
        });
    }

    fn add_note(&mut self, name: &str, content: &str) {
        self.data.notes.push(SupportInfoNote {
            name: name.to_owned(),
            content: content.to_owned(),
        });
    }
}
