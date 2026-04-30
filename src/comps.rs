// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufRead, Write};

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::QName;
use quick_xml::{Reader, Writer};

use super::metadata::{
    CompsCategory, CompsData, CompsEnvironment, CompsEnvironmentOption, CompsGroup, CompsLangpack,
    CompsPackageReq, CompsXml, RpmMetadata,
};
use super::{MetadataError, Repository};

const TAG_COMPS: &str = "comps";
const TAG_GROUP: &str = "group";
const TAG_CATEGORY: &str = "category";
const TAG_ENVIRONMENT: &str = "environment";
const TAG_ID: &str = "id";
const TAG_NAME: &str = "name";
const TAG_DESCRIPTION: &str = "description";
const TAG_DEFAULT: &str = "default";
const TAG_USERVISIBLE: &str = "uservisible";
const TAG_BIARCHONLY: &str = "biarchonly";
const TAG_LANGONLY: &str = "langonly";
const TAG_DISPLAY_ORDER: &str = "display_order";
const TAG_PACKAGELIST: &str = "packagelist";
const TAG_PACKAGEREQ: &str = "packagereq";
const TAG_GROUPLIST: &str = "grouplist";
const TAG_OPTIONLIST: &str = "optionlist";
const TAG_GROUPID: &str = "groupid";
const TAG_LANGPACKS: &str = "langpacks";
const TAG_MATCH: &str = "match";

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
    pub fn new_writer<W: Write>(writer: quick_xml::Writer<W>) -> CompsXmlWriter<W> {
        CompsXmlWriter { writer }
    }

    pub fn new_reader<R: BufRead>(reader: quick_xml::Reader<R>) -> CompsXmlReader<R> {
        CompsXmlReader { reader }
    }

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

    pub fn into_inner(self) -> W {
        self.writer.into_inner()
    }
}

/// Reader for comps.xml metadata (package groups, categories, and environments).
pub struct CompsXmlReader<R: BufRead> {
    reader: Reader<R>,
}

impl<R: BufRead> CompsXmlReader<R> {
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
        parse_item(&mut self.reader)
    }
}

fn parse_item<R: BufRead>(reader: &mut Reader<R>) -> Result<Option<CompsItem>, MetadataError> {
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_GROUP => return parse_group(reader).map(|g| Some(CompsItem::Group(g))),
                TAG_CATEGORY => {
                    return parse_category(reader).map(|c| Some(CompsItem::Category(c)));
                }
                TAG_ENVIRONMENT => {
                    return parse_environment(reader).map(|e| Some(CompsItem::Environment(e)));
                }
                TAG_LANGPACKS => {
                    return parse_langpacks(reader).map(|l| Some(CompsItem::Langpacks(l)));
                }
                _ => (),
            },
            Event::Eof => return Ok(None),
            _ => (),
        }
        buf.clear();
    }
}

fn parse_bool(s: &str) -> bool {
    matches!(s, "true" | "yes" | "1")
}

fn parse_group<R: BufRead>(reader: &mut Reader<R>) -> Result<CompsGroup, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut group = CompsGroup::default();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_GROUP.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    group.id = reader
                        .read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_NAME => {
                    let text = reader
                        .read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        group
                            .name_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        group.name = text;
                    }
                }
                TAG_DESCRIPTION => {
                    let text = reader
                        .read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        group
                            .desc_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        group.description = text;
                    }
                }
                TAG_DEFAULT => {
                    let text = reader
                        .read_text_into(QName(TAG_DEFAULT.as_bytes()), &mut text_buf)?
                        .decode()?;
                    group.default = parse_bool(&text);
                }
                TAG_USERVISIBLE => {
                    let text = reader
                        .read_text_into(QName(TAG_USERVISIBLE.as_bytes()), &mut text_buf)?
                        .decode()?;
                    group.uservisible = parse_bool(&text);
                }
                TAG_BIARCHONLY => {
                    let text = reader
                        .read_text_into(QName(TAG_BIARCHONLY.as_bytes()), &mut text_buf)?
                        .decode()?;
                    group.biarchonly = parse_bool(&text);
                }
                TAG_LANGONLY => {
                    group.langonly = Some(
                        reader
                            .read_text_into(QName(TAG_LANGONLY.as_bytes()), &mut text_buf)?
                            .decode()?
                            .into_owned(),
                    );
                }
                TAG_DISPLAY_ORDER => {
                    let text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?
                        .decode()?;
                    group.display_order = Some(text.parse()?);
                }
                TAG_PACKAGELIST => {
                    group.packages = parse_packagelist(reader)?;
                }
                _ => (),
            },
            Event::Empty(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_PACKAGELIST => (),
                TAG_DESCRIPTION => {
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        group
                            .desc_by_lang
                            .push((lang.unescape_value()?.into_owned(), String::new()));
                    }
                }
                _ => (),
            },
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(group)
}

fn parse_packagelist<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Vec<CompsPackageReq>, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut packages = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_PACKAGELIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_PACKAGEREQ.as_bytes() => {
                let mut pkg = CompsPackageReq::default();

                pkg.reqtype = e
                    .try_get_attribute("type")?
                    .ok_or_else(|| MetadataError::MissingAttributeError("type"))?
                    .unescape_value()?
                    .into_owned();

                if let Some(requires) = e.try_get_attribute("requires")? {
                    pkg.requires = Some(requires.unescape_value()?.into_owned());
                }

                if let Some(basearchonly) = e.try_get_attribute("basearchonly")? {
                    pkg.basearchonly = parse_bool(&basearchonly.unescape_value()?);
                }

                pkg.name = reader
                    .read_text_into(QName(TAG_PACKAGEREQ.as_bytes()), &mut text_buf)?
                    .decode()?
                    .into_owned();

                packages.push(pkg);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(packages)
}

fn parse_category<R: BufRead>(reader: &mut Reader<R>) -> Result<CompsCategory, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut category = CompsCategory::default();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_CATEGORY.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    category.id = reader
                        .read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_NAME => {
                    let text = reader
                        .read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        category
                            .name_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        category.name = text;
                    }
                }
                TAG_DESCRIPTION => {
                    let text = reader
                        .read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        category
                            .desc_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        category.description = text;
                    }
                }
                TAG_DISPLAY_ORDER => {
                    let text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?
                        .decode()?;
                    category.display_order = Some(text.parse()?);
                }
                TAG_GROUPLIST => {
                    category.group_ids = parse_grouplist(reader)?;
                }
                _ => (),
            },
            Event::Empty(e) if e.name().as_ref() == TAG_DESCRIPTION.as_bytes() => {
                if let Some(lang) = e.try_get_attribute("xml:lang")? {
                    category
                        .desc_by_lang
                        .push((lang.unescape_value()?.into_owned(), String::new()));
                }
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(category)
}

fn parse_environment<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<CompsEnvironment, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut environment = CompsEnvironment::default();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_ENVIRONMENT.as_bytes() => break,
            Event::Start(e) => match std::str::from_utf8(e.name().as_ref()).unwrap_or("") {
                TAG_ID => {
                    environment.id = reader
                        .read_text_into(QName(TAG_ID.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                }
                TAG_NAME => {
                    let text = reader
                        .read_text_into(QName(TAG_NAME.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        environment
                            .name_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        environment.name = text;
                    }
                }
                TAG_DESCRIPTION => {
                    let text = reader
                        .read_text_into(QName(TAG_DESCRIPTION.as_bytes()), &mut text_buf)?
                        .decode()?
                        .into_owned();
                    if let Some(lang) = e.try_get_attribute("xml:lang")? {
                        environment
                            .desc_by_lang
                            .push((lang.unescape_value()?.into_owned(), text));
                    } else {
                        environment.description = text;
                    }
                }
                TAG_DISPLAY_ORDER => {
                    let text = reader
                        .read_text_into(QName(TAG_DISPLAY_ORDER.as_bytes()), &mut text_buf)?
                        .decode()?;
                    environment.display_order = Some(text.parse()?);
                }
                TAG_GROUPLIST => {
                    environment.group_ids = parse_grouplist(reader)?;
                }
                TAG_OPTIONLIST => {
                    environment.option_ids = parse_optionlist(reader)?;
                }
                _ => (),
            },
            Event::Empty(e) if e.name().as_ref() == TAG_DESCRIPTION.as_bytes() => {
                if let Some(lang) = e.try_get_attribute("xml:lang")? {
                    environment
                        .desc_by_lang
                        .push((lang.unescape_value()?.into_owned(), String::new()));
                }
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(environment)
}

fn parse_grouplist<R: BufRead>(reader: &mut Reader<R>) -> Result<Vec<String>, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut group_ids = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e)
                if e.name().as_ref() == TAG_GROUPLIST.as_bytes()
                    || e.name().as_ref() == TAG_OPTIONLIST.as_bytes() =>
            {
                break;
            }
            Event::Start(e) if e.name().as_ref() == TAG_GROUPID.as_bytes() => {
                let gid = reader
                    .read_text_into(QName(TAG_GROUPID.as_bytes()), &mut text_buf)?
                    .decode()?
                    .into_owned();
                group_ids.push(gid);
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(group_ids)
}

fn parse_optionlist<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Vec<CompsEnvironmentOption>, MetadataError> {
    let mut buf = Vec::new();
    let mut text_buf = Vec::new();
    let mut options = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_OPTIONLIST.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_GROUPID.as_bytes() => {
                let default = e
                    .try_get_attribute("default")?
                    .map(|a| parse_bool(&a.unescape_value().unwrap_or_default()))
                    .unwrap_or(false);

                let group_id = reader
                    .read_text_into(QName(TAG_GROUPID.as_bytes()), &mut text_buf)?
                    .decode()?
                    .into_owned();

                options.push(CompsEnvironmentOption { group_id, default });
            }
            _ => (),
        }
        buf.clear();
        text_buf.clear();
    }

    Ok(options)
}

/// Parse a `<langpacks>` element containing `<match>` entries.
fn parse_langpacks<R: BufRead>(
    reader: &mut Reader<R>,
) -> Result<Vec<CompsLangpack>, MetadataError> {
    let mut buf = Vec::new();
    let mut langpacks = Vec::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::End(e) if e.name().as_ref() == TAG_LANGPACKS.as_bytes() => break,
            Event::Start(e) if e.name().as_ref() == TAG_MATCH.as_bytes() => {
                let name = e
                    .try_get_attribute("name")?
                    .map(|a| a.unescape_value().unwrap_or_default().into_owned())
                    .unwrap_or_default();
                let install = e
                    .try_get_attribute("install")?
                    .map(|a| a.unescape_value().unwrap_or_default().into_owned())
                    .unwrap_or_default();
                langpacks.push(CompsLangpack { name, install });
            }
            _ => (),
        }
        buf.clear();
    }

    Ok(langpacks)
}
