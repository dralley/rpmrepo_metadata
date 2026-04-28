// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;
use std::io::Cursor;

const COMPS_FIXTURE_PATH: &str = "./tests/assets/comps_fixture.xml";

static EMPTY_COMPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<comps>
</comps>
"#;

#[test]
fn test_comps_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = CompsXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));
    writer.write_header()?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();
    let actual = std::str::from_utf8(&buffer)?;
    assert_eq!(&actual, &EMPTY_COMPS);

    Ok(())
}

#[test]
fn test_comps_xml_read_fixture() -> Result<(), MetadataError> {
    let fixture = std::fs::read_to_string(COMPS_FIXTURE_PATH).unwrap();
    let mut reader = CompsXmlReader::new(utils::create_xml_reader(fixture.as_bytes()));

    let mut groups = Vec::new();
    let mut categories = Vec::new();
    let mut environments = Vec::new();

    let langpacks = reader.read_all(&mut groups, &mut categories, &mut environments)?;

    assert_eq!(groups.len(), 8);
    assert_eq!(categories.len(), 2);
    assert_eq!(environments.len(), 1);

    // Verify first group
    let g = &groups[0];
    assert_eq!(g.id, "additional-devel");
    assert_eq!(g.name, "Additional Development");
    assert_eq!(g.default, false);
    assert_eq!(g.uservisible, false);
    assert_eq!(g.biarchonly, true);
    assert_eq!(g.langonly, Some("fr".to_string()));
    assert_eq!(g.packages.len(), 6);
    assert_eq!(g.packages[0].name, "alsa-lib-devel");
    assert_eq!(g.packages[0].reqtype, "default");

    // Verify group with conditional requires
    let g = &groups[3]; // ansible-node
    assert_eq!(g.id, "ansible-node");
    assert_eq!(g.packages[1].name, "libselinux-python");
    assert_eq!(g.packages[1].reqtype, "conditional");
    assert_eq!(g.packages[1].requires, Some("selinux-policy".to_string()));

    // Verify basearchonly
    let g = &groups[4]; // d-development
    assert_eq!(g.packages[0].basearchonly, true);
    assert_eq!(g.packages[4].basearchonly, false);

    // Verify empty packagelist
    let g = &groups[5];
    assert_eq!(g.id, "empty-group-1");
    assert_eq!(g.packages.len(), 0);

    // Verify missing packagelist
    let g = &groups[6];
    assert_eq!(g.id, "empty-group-2");
    assert_eq!(g.packages.len(), 0);

    // Verify category
    let c = &categories[0];
    assert_eq!(c.id, "development");
    assert_eq!(c.name, "Development");
    assert_eq!(c.display_order, Some(90));
    assert_eq!(c.group_ids, vec!["additional-devel", "d-development"]);

    // Verify category without display_order
    let c = &categories[1];
    assert_eq!(c.id, "servers");
    assert_eq!(c.display_order, None);

    // Verify environment
    let e = &environments[0];
    assert_eq!(e.id, "minimal-environment");
    assert_eq!(e.name, "Minimal Install");
    assert_eq!(e.display_order, Some(3));
    assert_eq!(e.group_ids, vec!["backup-client"]);
    assert_eq!(e.option_ids.len(), 2);
    assert_eq!(e.option_ids[0].group_id, "ansible-node");
    assert_eq!(e.option_ids[0].default, true);
    assert_eq!(e.option_ids[1].group_id, "backup-server");
    assert_eq!(e.option_ids[1].default, false);

    // Verify langpacks
    assert_eq!(langpacks.len(), 2);
    assert_eq!(langpacks[0].name, "firefox");
    assert_eq!(langpacks[0].install, "firefox-langpack-%s");
    assert_eq!(langpacks[1].name, "libreoffice-core");
    assert_eq!(langpacks[1].install, "libreoffice-langpack-%s");

    Ok(())
}

#[test]
fn test_comps_xml_roundtrip() -> Result<(), MetadataError> {
    let fixture = std::fs::read_to_string(COMPS_FIXTURE_PATH).unwrap();

    // Read
    let mut reader = CompsXmlReader::new(utils::create_xml_reader(fixture.as_bytes()));
    let mut groups = Vec::new();
    let mut categories = Vec::new();
    let mut environments = Vec::new();
    let langpacks = reader.read_all(&mut groups, &mut categories, &mut environments)?;

    // Write
    let mut writer = CompsXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));
    writer.write_header()?;
    for g in &groups {
        writer.write_group(g)?;
    }
    for c in &categories {
        writer.write_category(c)?;
    }
    for e in &environments {
        writer.write_environment(e)?;
    }
    writer.write_langpacks(&langpacks)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();
    let written = std::str::from_utf8(&buffer)?;

    // Re-read
    let mut reader2 = CompsXmlReader::new(utils::create_xml_reader(written.as_bytes()));
    let mut groups2 = Vec::new();
    let mut categories2 = Vec::new();
    let mut environments2 = Vec::new();
    let langpacks2 = reader2.read_all(&mut groups2, &mut categories2, &mut environments2)?;

    assert_eq!(groups, groups2);
    assert_eq!(categories, categories2);
    assert_eq!(environments, environments2);
    assert_eq!(langpacks, langpacks2);

    Ok(())
}

#[test]
fn test_comps_xml_read_real_repo() -> Result<(), MetadataError> {
    let path = "./tests/assets/external_repos/epel9/repodata/4ec0ae1bc2c5baa2d1e390e68ed92033cb05deea86333bd01351e8f73615e404-comps-Everything.x86_64.xml";

    if !std::path::Path::new(path).exists() {
        return Ok(());
    }

    let fixture = std::fs::read_to_string(path).unwrap();
    let mut reader = CompsXmlReader::new(utils::create_xml_reader(fixture.as_bytes()));
    let mut groups = Vec::new();
    let mut categories = Vec::new();
    let mut environments = Vec::new();
    let _langpacks = reader.read_all(&mut groups, &mut categories, &mut environments)?;

    assert!(groups.len() > 10);
    assert!(categories.len() >= 1);
    assert!(environments.len() >= 1);

    // Verify translations are parsed
    let g = &groups[0];
    assert!(!g.name_by_lang.is_empty());

    Ok(())
}
