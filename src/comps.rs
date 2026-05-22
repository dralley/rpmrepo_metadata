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
    CompsCategory, CompsData, CompsEnvironment, CompsEnvironmentOption, CompsGroup, CompsLangpack,
    CompsPackageReq, CompsXml, RpmMetadata,
};
use crate::parsing_utils::{resolve_attr, resolve_text};
use crate::visitor::CompsVisitor;
use crate::{MetadataError, Repository};

impl RpmMetadata for CompsXml {
    fn filename() -> &'static str {
        "comps.xml"
    }

    fn load_metadata<R: BufRead>(
        repository: &mut Repository,
        reader: Reader<R>,
    ) -> Result<(), MetadataError> {
        let mut reader = CompsXml::new_reader(reader);
        while let Some(item) = reader.read_item()? {
            match item {
                CompsItem::Group(g) => repository.groups_mut().push(g),
                CompsItem::Category(c) => repository.categories_mut().push(c),
                CompsItem::Environment(e) => repository.environments_mut().push(e),
                CompsItem::Langpacks(l) => repository.langpacks_mut().extend(l),
            }
        }
        Ok(())
    }

    fn write_metadata<W: Write>(
        repository: &Repository,
        writer: Writer<W>,
    ) -> Result<(), MetadataError> {
        let mut writer = CompsXml::new_writer(writer);
        writer.write_header()?;
        for group in repository.groups() {
            writer.write_group(group)?;
        }
        for category in repository.categories() {
            writer.write_category(category)?;
        }
        for environment in repository.environments() {
            writer.write_environment(environment)?;
        }
        if !repository.langpacks().is_empty() {
            writer.write_langpacks(repository.langpacks())?;
        }
        writer.finish()
    }
}

impl CompsXml {
    /// Create a new comps.xml writer.
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> CompsXmlWriter<W> {
        CompsXmlWriter { writer }
    }

    /// Create a new comps.xml reader.
    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> CompsXmlReader<R> {
        CompsXmlReader { reader }
    }

    /// Parse an entire comps.xml file into a [`CompsData`] structure.
    pub fn read_data<R: BufRead>(reader: Reader<R>) -> Result<CompsData, MetadataError> {
        let mut comps_reader = CompsXml::new_reader(reader);
        let mut groups = Vec::new();
        let mut categories = Vec::new();
        let mut environments = Vec::new();
        let langpacks = comps_reader.read_all(&mut groups, &mut categories, &mut environments)?;
        Ok(CompsData {
            groups,
            categories,
            environments,
            langpacks,
        })
    }

    /// Serialize a [`CompsData`] structure to a comps.xml file.
    pub fn write_data<W: Write>(data: &CompsData, writer: Writer<W>) -> Result<(), MetadataError> {
        let mut writer = CompsXml::new_writer(writer);
        writer.write_header()?;
        for group in &data.groups {
            writer.write_group(group)?;
        }
        for category in &data.categories {
            writer.write_category(category)?;
        }
        for environment in &data.environments {
            writer.write_environment(environment)?;
        }
        if !data.langpacks.is_empty() {
            writer.write_langpacks(&data.langpacks)?;
        }
        writer.finish()
    }
}

/// Parsed item from a comps.xml file.
enum CompsItem {
    Group(CompsGroup),
    Category(CompsCategory),
    Environment(CompsEnvironment),
    Langpacks(Vec<CompsLangpack>),
}

/// Writer for comps.xml metadata (package groups, categories, and environments).
pub struct CompsXmlWriter<W: Write> {
    writer: Writer<W>,
}

impl<W: Write> CompsXmlWriter<W> {
    /// Write the XML declaration and opening `<comps>` tag.
    pub fn write_header(&mut self) -> Result<(), MetadataError> {
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        let comps_tag = BytesStart::new(TAG_COMPS);
        self.writer.write_event(Event::Start(comps_tag.borrow()))?;

        Ok(())
    }

    /// Write a `<group>` element with its package list.
    pub fn write_group(&mut self, group: &CompsGroup) -> Result<(), MetadataError> {
        let group_tag = BytesStart::new(TAG_GROUP);
        self.writer.write_event(Event::Start(group_tag.borrow()))?;

        self.writer
            .create_element(TAG_ID)
            .write_text_content(BytesText::new(&group.id))?;

        self.writer
            .create_element(TAG_NAME)
            .write_text_content(BytesText::new(&group.name))?;
        for (lang, value) in &group.name_by_lang {
            self.writer
                .create_element(TAG_NAME)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        self.writer
            .create_element(TAG_DESCRIPTION)
            .write_text_content(BytesText::new(&group.description))?;
        for (lang, value) in &group.desc_by_lang {
            self.writer
                .create_element(TAG_DESCRIPTION)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        self.writer
            .create_element(TAG_DEFAULT)
            .write_text_content(BytesText::new(if group.default { "true" } else { "false" }))?;

        self.writer
            .create_element(TAG_USERVISIBLE)
            .write_text_content(BytesText::new(if group.uservisible {
                "true"
            } else {
                "false"
            }))?;

        if group.biarchonly {
            self.writer
                .create_element(TAG_BIARCHONLY)
                .write_text_content(BytesText::new("true"))?;
        }

        if let Some(langonly) = &group.langonly {
            self.writer
                .create_element(TAG_LANGONLY)
                .write_text_content(BytesText::new(langonly))?;
        }

        if let Some(order) = group.display_order {
            self.writer
                .create_element(TAG_DISPLAY_ORDER)
                .write_text_content(BytesText::new(&order.to_string()))?;
        }

        if group.packages.is_empty() {
            self.writer
                .write_event(Event::Empty(BytesStart::new(TAG_PACKAGELIST)))?;
        } else {
            let packagelist_tag = BytesStart::new(TAG_PACKAGELIST);
            self.writer
                .write_event(Event::Start(packagelist_tag.borrow()))?;

            for pkg in &group.packages {
                let mut req_tag = BytesStart::new(TAG_PACKAGEREQ);
                req_tag.push_attribute(("type", pkg.reqtype.as_str()));
                if let Some(requires) = &pkg.requires {
                    req_tag.push_attribute(("requires", requires.as_str()));
                }
                if pkg.basearchonly {
                    req_tag.push_attribute(("basearchonly", "true"));
                }
                self.writer.write_event(Event::Start(req_tag.borrow()))?;
                self.writer
                    .write_event(Event::Text(BytesText::new(&pkg.name)))?;
                self.writer.write_event(Event::End(req_tag.to_end()))?;
            }

            self.writer
                .write_event(Event::End(packagelist_tag.to_end()))?;
        }

        self.writer.write_event(Event::End(group_tag.to_end()))?;

        Ok(())
    }

    /// Write a `<category>` element with its group list.
    pub fn write_category(&mut self, category: &CompsCategory) -> Result<(), MetadataError> {
        let category_tag = BytesStart::new(TAG_CATEGORY);
        self.writer
            .write_event(Event::Start(category_tag.borrow()))?;

        self.writer
            .create_element(TAG_ID)
            .write_text_content(BytesText::new(&category.id))?;

        self.writer
            .create_element(TAG_NAME)
            .write_text_content(BytesText::new(&category.name))?;
        for (lang, value) in &category.name_by_lang {
            self.writer
                .create_element(TAG_NAME)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        self.writer
            .create_element(TAG_DESCRIPTION)
            .write_text_content(BytesText::new(&category.description))?;
        for (lang, value) in &category.desc_by_lang {
            self.writer
                .create_element(TAG_DESCRIPTION)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        if let Some(order) = category.display_order {
            self.writer
                .create_element(TAG_DISPLAY_ORDER)
                .write_text_content(BytesText::new(&order.to_string()))?;
        }

        let grouplist_tag = BytesStart::new(TAG_GROUPLIST);
        self.writer
            .write_event(Event::Start(grouplist_tag.borrow()))?;
        for gid in &category.group_ids {
            self.writer
                .create_element(TAG_GROUPID)
                .write_text_content(BytesText::new(gid))?;
        }
        self.writer
            .write_event(Event::End(grouplist_tag.to_end()))?;

        self.writer.write_event(Event::End(category_tag.to_end()))?;

        Ok(())
    }

    /// Write an `<environment>` element with its group and option lists.
    pub fn write_environment(
        &mut self,
        environment: &CompsEnvironment,
    ) -> Result<(), MetadataError> {
        let env_tag = BytesStart::new(TAG_ENVIRONMENT);
        self.writer.write_event(Event::Start(env_tag.borrow()))?;

        self.writer
            .create_element(TAG_ID)
            .write_text_content(BytesText::new(&environment.id))?;

        self.writer
            .create_element(TAG_NAME)
            .write_text_content(BytesText::new(&environment.name))?;
        for (lang, value) in &environment.name_by_lang {
            self.writer
                .create_element(TAG_NAME)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        self.writer
            .create_element(TAG_DESCRIPTION)
            .write_text_content(BytesText::new(&environment.description))?;
        for (lang, value) in &environment.desc_by_lang {
            self.writer
                .create_element(TAG_DESCRIPTION)
                .with_attribute(("xml:lang", lang.as_str()))
                .write_text_content(BytesText::new(value))?;
        }

        if let Some(order) = environment.display_order {
            self.writer
                .create_element(TAG_DISPLAY_ORDER)
                .write_text_content(BytesText::new(&order.to_string()))?;
        }

        let grouplist_tag = BytesStart::new(TAG_GROUPLIST);
        self.writer
            .write_event(Event::Start(grouplist_tag.borrow()))?;
        for gid in &environment.group_ids {
            self.writer
                .create_element(TAG_GROUPID)
                .write_text_content(BytesText::new(gid))?;
        }
        self.writer
            .write_event(Event::End(grouplist_tag.to_end()))?;

        let optionlist_tag = BytesStart::new(TAG_OPTIONLIST);
        self.writer
            .write_event(Event::Start(optionlist_tag.borrow()))?;
        for opt in &environment.option_ids {
            if opt.default {
                self.writer
                    .create_element(TAG_GROUPID)
                    .with_attribute(("default", "true"))
                    .write_text_content(BytesText::new(&opt.group_id))?;
            } else {
                self.writer
                    .create_element(TAG_GROUPID)
                    .write_text_content(BytesText::new(&opt.group_id))?;
            }
        }
        self.writer
            .write_event(Event::End(optionlist_tag.to_end()))?;

        self.writer.write_event(Event::End(env_tag.to_end()))?;

        Ok(())
    }

    /// Write a `<langpacks>` element with its match entries.
    pub fn write_langpacks(&mut self, langpacks: &[CompsLangpack]) -> Result<(), MetadataError> {
        let langpacks_tag = BytesStart::new(TAG_LANGPACKS);
        self.writer
            .write_event(Event::Start(langpacks_tag.borrow()))?;

        for lp in langpacks {
            self.writer
                .create_element(TAG_MATCH)
                .with_attribute(("install", lp.install.as_str()))
                .with_attribute(("name", lp.name.as_str()))
                .write_empty()?;
        }

        self.writer
            .write_event(Event::End(langpacks_tag.to_end()))?;

        Ok(())
    }

    /// Write the closing `</comps>` tag and flush.
    pub fn finish(&mut self) -> Result<(), MetadataError> {
        self.writer
            .write_event(Event::End(BytesEnd::new(TAG_COMPS)))?;

        self.writer.write_event(Event::Text(BytesText::new("\n")))?;

        self.writer.get_mut().flush()?;

        Ok(())
    }

    /// Consume the writer and return the underlying writer.
    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

/// Reader for comps.xml metadata (package groups, categories, and environments).
pub struct CompsXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> CompsXmlReader<R> {
    /// Create a new comps.xml reader from an XML reader.
    pub fn new(reader: Reader<R>) -> Self {
        Self { reader }
    }

    /// Read all groups, categories, environments, and langpacks from the comps.xml stream.
    pub fn read_all(
        &mut self,
        groups: &mut Vec<CompsGroup>,
        categories: &mut Vec<CompsCategory>,
        environments: &mut Vec<CompsEnvironment>,
    ) -> Result<Vec<CompsLangpack>, MetadataError> {
        let mut langpacks = Vec::new();
        while let Some(item) = self.read_item()? {
            match item {
                CompsItem::Group(g) => groups.push(g),
                CompsItem::Category(c) => categories.push(c),
                CompsItem::Environment(e) => environments.push(e),
                CompsItem::Langpacks(l) => langpacks.extend(l),
            }
        }
        Ok(langpacks)
    }

    fn read_item(&mut self) -> Result<Option<CompsItem>, MetadataError> {
        let mut materializer = CompsMaterializer::new();
        if parse_comps_item(&mut self.reader, &mut materializer)? {
            Ok(materializer.take_item())
        } else {
            Ok(None)
        }
    }
}

struct CompsMaterializer {
    current_group: Option<CompsGroup>,
    current_category: Option<CompsCategory>,
    current_environment: Option<CompsEnvironment>,
    current_langpacks: Option<Vec<CompsLangpack>>,
}

impl CompsMaterializer {
    fn new() -> Self {
        CompsMaterializer {
            current_group: None,
            current_category: None,
            current_environment: None,
            current_langpacks: None,
        }
    }

    fn take_item(self) -> Option<CompsItem> {
        if let Some(g) = self.current_group {
            Some(CompsItem::Group(g))
        } else if let Some(c) = self.current_category {
            Some(CompsItem::Category(c))
        } else if let Some(e) = self.current_environment {
            Some(CompsItem::Environment(e))
        } else {
            self.current_langpacks.map(CompsItem::Langpacks)
        }
    }
}

impl CompsVisitor for CompsMaterializer {
    fn begin_group(&mut self) {
        self.current_group = Some(CompsGroup::default());
    }

    fn set_group_id(&mut self, id: &str) {
        if let Some(g) = self.current_group.as_mut() {
            g.id = id.to_owned();
        }
    }

    fn set_group_name(&mut self, name: &str, lang: Option<&str>) {
        if let Some(g) = self.current_group.as_mut() {
            if let Some(lang) = lang {
                g.name_by_lang.push((lang.to_owned(), name.to_owned()));
            } else {
                g.name = name.to_owned();
            }
        }
    }

    fn set_group_description(&mut self, desc: &str, lang: Option<&str>) {
        if let Some(g) = self.current_group.as_mut() {
            if let Some(lang) = lang {
                g.desc_by_lang.push((lang.to_owned(), desc.to_owned()));
            } else {
                g.description = desc.to_owned();
            }
        }
    }

    fn set_group_default(&mut self, default: bool) {
        if let Some(g) = self.current_group.as_mut() {
            g.default = default;
        }
    }

    fn set_group_uservisible(&mut self, visible: bool) {
        if let Some(g) = self.current_group.as_mut() {
            g.uservisible = visible;
        }
    }

    fn set_group_biarchonly(&mut self, biarchonly: bool) {
        if let Some(g) = self.current_group.as_mut() {
            g.biarchonly = biarchonly;
        }
    }

    fn set_group_langonly(&mut self, langonly: &str) {
        if let Some(g) = self.current_group.as_mut() {
            g.langonly = Some(langonly.to_owned());
        }
    }

    fn set_group_display_order(&mut self, order: u32) {
        if let Some(g) = self.current_group.as_mut() {
            g.display_order = Some(order);
        }
    }

    fn add_group_package(
        &mut self,
        name: &str,
        reqtype: &str,
        requires: Option<&str>,
        basearchonly: bool,
    ) {
        if let Some(g) = self.current_group.as_mut() {
            g.packages.push(CompsPackageReq {
                name: name.to_owned(),
                reqtype: reqtype.to_owned(),
                requires: requires.map(|s| s.to_owned()),
                basearchonly,
            });
        }
    }

    fn begin_category(&mut self) {
        self.current_category = Some(CompsCategory::default());
    }

    fn set_category_id(&mut self, id: &str) {
        if let Some(c) = self.current_category.as_mut() {
            c.id = id.to_owned();
        }
    }

    fn set_category_name(&mut self, name: &str, lang: Option<&str>) {
        if let Some(c) = self.current_category.as_mut() {
            if let Some(lang) = lang {
                c.name_by_lang.push((lang.to_owned(), name.to_owned()));
            } else {
                c.name = name.to_owned();
            }
        }
    }

    fn set_category_description(&mut self, desc: &str, lang: Option<&str>) {
        if let Some(c) = self.current_category.as_mut() {
            if let Some(lang) = lang {
                c.desc_by_lang.push((lang.to_owned(), desc.to_owned()));
            } else {
                c.description = desc.to_owned();
            }
        }
    }

    fn set_category_display_order(&mut self, order: u32) {
        if let Some(c) = self.current_category.as_mut() {
            c.display_order = Some(order);
        }
    }

    fn add_category_group_id(&mut self, group_id: &str) {
        if let Some(c) = self.current_category.as_mut() {
            c.group_ids.push(group_id.to_owned());
        }
    }

    fn begin_environment(&mut self) {
        self.current_environment = Some(CompsEnvironment::default());
    }

    fn set_environment_id(&mut self, id: &str) {
        if let Some(e) = self.current_environment.as_mut() {
            e.id = id.to_owned();
        }
    }

    fn set_environment_name(&mut self, name: &str, lang: Option<&str>) {
        if let Some(e) = self.current_environment.as_mut() {
            if let Some(lang) = lang {
                e.name_by_lang.push((lang.to_owned(), name.to_owned()));
            } else {
                e.name = name.to_owned();
            }
        }
    }

    fn set_environment_description(&mut self, desc: &str, lang: Option<&str>) {
        if let Some(e) = self.current_environment.as_mut() {
            if let Some(lang) = lang {
                e.desc_by_lang.push((lang.to_owned(), desc.to_owned()));
            } else {
                e.description = desc.to_owned();
            }
        }
    }

    fn set_environment_display_order(&mut self, order: u32) {
        if let Some(e) = self.current_environment.as_mut() {
            e.display_order = Some(order);
        }
    }

    fn add_environment_group_id(&mut self, group_id: &str) {
        if let Some(e) = self.current_environment.as_mut() {
            e.group_ids.push(group_id.to_owned());
        }
    }

    fn add_environment_option_id(&mut self, group_id: &str, default: bool) {
        if let Some(e) = self.current_environment.as_mut() {
            e.option_ids.push(CompsEnvironmentOption {
                group_id: group_id.to_owned(),
                default,
            });
        }
    }

    fn add_langpack(&mut self, name: &str, install: &str) {
        self.current_langpacks
            .get_or_insert_with(Vec::new)
            .push(CompsLangpack {
                name: name.to_owned(),
                install: install.to_owned(),
            });
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s, "true" | "yes" | "1")
}

/// Parse one top-level element from comps.xml, dispatching to `visitor`.
///
/// Returns `true` if an element was parsed, `false` at EOF.
pub fn parse_comps_item<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<bool, MetadataError> {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_GROUP => {
                    parse_comps_group(reader, visitor)?;
                    return Ok(true);
                }
                TAG_CATEGORY => {
                    parse_comps_category(reader, visitor)?;
                    return Ok(true);
                }
                TAG_ENVIRONMENT => {
                    parse_comps_environment(reader, visitor)?;
                    return Ok(true);
                }
                TAG_LANGPACKS => {
                    parse_comps_langpacks(reader, visitor)?;
                    return Ok(true);
                }
                _ => (),
            },
            Event::Eof => return Ok(false),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_comps_group<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);

    visitor.begin_group();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_GROUP.as_bytes() => {
                visitor.end_group();
                return Ok(());
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_id(&text);
                }
                TAG_NAME => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_group_name(&text, lang_cow.as_deref());
                }
                TAG_DESCRIPTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_group_description(&text, lang_cow.as_deref());
                }
                TAG_DEFAULT => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DEFAULT.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_default(parse_bool(&text));
                }
                TAG_USERVISIBLE => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_USERVISIBLE.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_uservisible(parse_bool(&text));
                }
                TAG_BIARCHONLY => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_BIARCHONLY.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_biarchonly(parse_bool(&text));
                }
                TAG_LANGONLY => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_LANGONLY.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_langonly(&text);
                }
                TAG_DISPLAY_ORDER => {
                    let bytes_text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_group_display_order(text.parse()?);
                }
                TAG_PACKAGELIST => {
                    parse_comps_packagelist(reader, visitor, &mut buf, &mut text_buf)?;
                }
                _ => (),
            },
            Event::Empty(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGELIST => (),
                TAG_DESCRIPTION => {
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_group_description("", lang_cow.as_deref());
                }
                _ => (),
            },
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

fn parse_comps_packagelist<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGELIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGEREQ.as_bytes() => {
                let mut type_cow = None;
                let mut requires_cow = None;
                let mut basearchonly = false;

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        b"type" => type_cow = Some(resolve_attr(&attr)?),
                        b"requires" => requires_cow = Some(resolve_attr(&attr)?),
                        b"basearchonly" => {
                            if let Ok(v) = resolve_attr(&attr) {
                                basearchonly = parse_bool(&v);
                            }
                        }
                        _ => (),
                    }
                }

                let reqtype = type_cow.ok_or(MetadataError::MissingAttributeError("type"))?;
                let bytes_text =
                    reader.read_text_into(QName(TAG_PACKAGEREQ.as_bytes()), text_buf)?;
                let name = resolve_text(&bytes_text)?;
                visitor.add_group_package(&name, &reqtype, requires_cow.as_deref(), basearchonly);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
    Ok(())
}

fn parse_comps_category<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);

    visitor.begin_category();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_CATEGORY.as_bytes() => {
                visitor.end_category();
                return Ok(());
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_category_id(&text);
                }
                TAG_NAME => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_category_name(&text, lang_cow.as_deref());
                }
                TAG_DESCRIPTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_category_description(&text, lang_cow.as_deref());
                }
                TAG_DISPLAY_ORDER => {
                    let bytes_text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_category_display_order(text.parse()?);
                }
                TAG_GROUPLIST => {
                    parse_comps_grouplist_category(reader, visitor, &mut buf, &mut text_buf)?;
                }
                _ => (),
            },
            Event::Empty(e) if e.name().as_ref() == TAG_DESCRIPTION.as_bytes() => {
                let lang_attr = e.try_get_attribute("xml:lang")?;
                let lang_cow = match &lang_attr {
                    Some(attr) => Some(resolve_attr(attr)?),
                    None => None,
                };
                visitor.set_category_description("", lang_cow.as_deref());
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

fn parse_comps_grouplist_category<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_GROUPLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_GROUPID.as_bytes() => {
                let bytes_text = reader.read_text_into(QName(TAG_GROUPID.as_bytes()), text_buf)?;
                let text = resolve_text(&bytes_text)?;
                visitor.add_category_group_id(&text);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
    Ok(())
}

fn parse_comps_environment<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    let mut buf = Vec::with_capacity(256);
    let mut text_buf = Vec::with_capacity(256);

    visitor.begin_environment();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_ENVIRONMENT.as_bytes() => {
                visitor.end_environment();
                return Ok(());
            }
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_environment_id(&text);
                }
                TAG_NAME => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_environment_name(&text, lang_cow.as_deref());
                }
                TAG_DESCRIPTION => {
                    let bytes_text =
                        reader.read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    let lang_attr = e.try_get_attribute("xml:lang")?;
                    let lang_cow = match &lang_attr {
                        Some(attr) => Some(resolve_attr(attr)?),
                        None => None,
                    };
                    visitor.set_environment_description(&text, lang_cow.as_deref());
                }
                TAG_DISPLAY_ORDER => {
                    let bytes_text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?;
                    let text = resolve_text(&bytes_text)?;
                    visitor.set_environment_display_order(text.parse()?);
                }
                TAG_GROUPLIST => {
                    parse_comps_grouplist_environment(reader, visitor, &mut buf, &mut text_buf)?;
                }
                TAG_OPTIONLIST => {
                    parse_comps_optionlist(reader, visitor, &mut buf, &mut text_buf)?;
                }
                _ => (),
            },
            Event::Empty(e) if e.name().as_ref() == TAG_DESCRIPTION.as_bytes() => {
                let lang_attr = e.try_get_attribute("xml:lang")?;
                let lang_cow = match &lang_attr {
                    Some(attr) => Some(resolve_attr(attr)?),
                    None => None,
                };
                visitor.set_environment_description("", lang_cow.as_deref());
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
}

fn parse_comps_grouplist_environment<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_GROUPLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_GROUPID.as_bytes() => {
                let bytes_text = reader.read_text_into(QName(TAG_GROUPID.as_bytes()), text_buf)?;
                let text = resolve_text(&bytes_text)?;
                visitor.add_environment_group_id(&text);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
    Ok(())
}

fn parse_comps_optionlist<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
    buf: &mut Vec<u8>,
    text_buf: &mut Vec<u8>,
) -> Result<(), MetadataError> {
    loop {
        match reader.read_event_into(buf)? {
            Event::End(e) if e.name().as_ref() == TAG_OPTIONLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_GROUPID.as_bytes() => {
                let default = e
                    .try_get_attribute("default")?
                    .map(|a| resolve_attr(&a).map(|v| parse_bool(&v)).unwrap_or(false))
                    .unwrap_or(false);

                let bytes_text = reader.read_text_into(QName(TAG_GROUPID.as_bytes()), text_buf)?;
                let text = resolve_text(&bytes_text)?;
                visitor.add_environment_option_id(&text, default);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }
    Ok(())
}

fn parse_comps_langpacks<R: BufRead, V: CompsVisitor>(
    reader: &mut Reader<R>,
    visitor: &mut V,
) -> Result<(), MetadataError> {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_LANGPACKS.as_bytes() => break,
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == TAG_MATCH.as_bytes() => {
                let mut name_cow: Cow<'_, str> = Cow::Borrowed("");
                let mut install_cow: Cow<'_, str> = Cow::Borrowed("");

                for attr_result in e.attributes() {
                    let attr = attr_result?;
                    match attr.key.as_ref() {
                        b"name" => name_cow = resolve_attr(&attr)?,
                        b"install" => install_cow = resolve_attr(&attr)?,
                        _ => (),
                    }
                }

                visitor.add_langpack(&name_cow, &install_cow);
            }
            _ => (),
        }
        buf.clear();
    }
    Ok(())
}
