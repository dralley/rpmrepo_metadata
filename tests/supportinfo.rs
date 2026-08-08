// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate rpmrepo_metadata;

use pretty_assertions::assert_eq;
use rpmrepo_metadata::*;
use std::fs::OpenOptions;
use std::io::{Cursor, Read, Seek, SeekFrom};
use tempdir::TempDir;

mod common;

static EMPTY_V1_SUPPORTINFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package_support schema_version="1.0" current_as="2024-01-01T00:00:00">
  <lifecycles/>
  <support_milestones/>
  <support_levels/>
  <packages/>
  <package_classes/>
  <package_origins/>
  <notes/>
</package_support>
"#;

static EMPTY_V1_SUPPORTINFO_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package_support schema_version="1.0" current_as="2024-01-01T00:00:00">
  <lifecycles/>
  <support_milestones/>
  <support_levels/>
  <packages/>
  <package_classes/>
  <package_origins/>
  <notes/>
"#;

static COMPLEX_V1_SUPPORTINFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package_support schema_version="1.0" current_as="2024-01-17T00:00:00">
  <lifecycles>
    <lifecycle name="default_lc" display_name="Default Lifecycle" description="Main support timeline">
      <phase name="supported" support_level="standard" start_milestone="ga" display_name="Supported"/>
      <phase name="unsupported" support_level="eos" start_milestone="eol" display_name="End of Life"/>
    </lifecycle>
    <lifecycle name="php81_lc" note="note_php81" display_name="PHP 8.1 Lifecycle" description="PHP 8.1 support timeline">
      <phase name="supported" support_level="standard" start_date="2023-03-15" display_name="Supported"/>
      <phase name="unsupported" support_level="eos" start_date="2026-11-25" display_name="End of Life"/>
    </lifecycle>
  </lifecycles>
  <support_milestones>
    <milestone name="ga" date="2023-03-15" display_name="GA" description="General availability"/>
    <milestone name="eol" date="2028-03-15" display_name="EOL" description="End of life"/>
  </support_milestones>
  <support_levels>
    <support_level name="standard" severities="Low,Medium,Important,Critical" description="Full support" display_name="Full Support"/>
    <support_level name="eos" severities="" description="End of support" display_name="End of Support"/>
  </support_levels>
  <packages>
    <package name="test-glibc" lifecycle="default_lc" package_class="core" origin="mylinux"/>
    <package name="test-php81" lifecycle="php81_lc" origin="mylinux"/>
  </packages>
  <package_classes>
    <package_class name="core">
      <summary>Core packages</summary>
      <text>Essential system packages</text>
    </package_class>
  </package_classes>
  <package_origins>
    <package_origin name="mylinux" repo_id="mylinux" dist="ml2023" vendor="MyVendor" display_name="My Linux 2023" description="Core repository"/>
  </package_origins>
  <notes>
    <note name="note_php81">PHP 8.1 upstream EOL is 2026-11-25</note>
  </notes>
</package_support>
"#;

fn complex_v1_data() -> SupportInfoData {
    SupportInfoData {
        current_as: "2024-01-17T00:00:00".to_owned(),
        lifecycles: vec![
            SupportInfoLifecycle {
                name: "default_lc".to_owned(),
                note: None,
                display_name: Some("Default Lifecycle".to_owned()),
                description: Some("Main support timeline".to_owned()),
                phases: vec![
                    SupportInfoPhase {
                        name: "supported".to_owned(),
                        support_level: "standard".to_owned(),
                        start_date: None,
                        start_milestone: Some("ga".to_owned()),
                        display_name: Some("Supported".to_owned()),
                    },
                    SupportInfoPhase {
                        name: "unsupported".to_owned(),
                        support_level: "eos".to_owned(),
                        start_date: None,
                        start_milestone: Some("eol".to_owned()),
                        display_name: Some("End of Life".to_owned()),
                    },
                ],
            },
            SupportInfoLifecycle {
                name: "php81_lc".to_owned(),
                note: Some("note_php81".to_owned()),
                display_name: Some("PHP 8.1 Lifecycle".to_owned()),
                description: Some("PHP 8.1 support timeline".to_owned()),
                phases: vec![
                    SupportInfoPhase {
                        name: "supported".to_owned(),
                        support_level: "standard".to_owned(),
                        start_date: Some("2023-03-15".to_owned()),
                        start_milestone: None,
                        display_name: Some("Supported".to_owned()),
                    },
                    SupportInfoPhase {
                        name: "unsupported".to_owned(),
                        support_level: "eos".to_owned(),
                        start_date: Some("2026-11-25".to_owned()),
                        start_milestone: None,
                        display_name: Some("End of Life".to_owned()),
                    },
                ],
            },
        ],
        milestones: vec![
            SupportInfoMilestone {
                name: "ga".to_owned(),
                date: "2023-03-15".to_owned(),
                display_name: Some("GA".to_owned()),
                description: Some("General availability".to_owned()),
            },
            SupportInfoMilestone {
                name: "eol".to_owned(),
                date: "2028-03-15".to_owned(),
                display_name: Some("EOL".to_owned()),
                description: Some("End of life".to_owned()),
            },
        ],
        support_levels: vec![
            SupportInfoLevel {
                name: "standard".to_owned(),
                severities: "Low,Medium,Important,Critical".to_owned(),
                description: Some("Full support".to_owned()),
                display_name: Some("Full Support".to_owned()),
            },
            SupportInfoLevel {
                name: "eos".to_owned(),
                severities: "".to_owned(),
                description: Some("End of support".to_owned()),
                display_name: Some("End of Support".to_owned()),
            },
        ],
        packages: vec![
            SupportInfoPackage {
                name: "test-glibc".to_owned(),
                lifecycle: "default_lc".to_owned(),
                origin: "mylinux".to_owned(),
                package_class: Some("core".to_owned()),
            },
            SupportInfoPackage {
                name: "test-php81".to_owned(),
                lifecycle: "php81_lc".to_owned(),
                origin: "mylinux".to_owned(),
                package_class: None,
            },
        ],
        package_classes: vec![SupportInfoPackageClass {
            name: "core".to_owned(),
            summary: "Core packages".to_owned(),
            text: "Essential system packages".to_owned(),
        }],
        package_origins: vec![SupportInfoPackageOrigin {
            name: "mylinux".to_owned(),
            repo_id: "mylinux".to_owned(),
            dist: "ml2023".to_owned(),
            vendor: "MyVendor".to_owned(),
            signing_key: None,
            display_name: Some("My Linux 2023".to_owned()),
            description: Some("Core repository".to_owned()),
        }],
        notes: vec![SupportInfoNote {
            name: "note_php81".to_owned(),
            content: "PHP 8.1 upstream EOL is 2026-11-25".to_owned(),
        }],
    }
}

// ------------- Writer tests -------------------------------------------

#[test]
fn test_supportinfo_v1_writer_empty() -> Result<(), MetadataError> {
    let mut writer = SupportInfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header("2024-01-01T00:00:00")?;
    writer.write_support_info(&SupportInfoData {
        current_as: "2024-01-01T00:00:00".to_owned(),
        ..Default::default()
    })?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();
    let actual = std::str::from_utf8(&buffer)?;

    assert_eq!(actual, EMPTY_V1_SUPPORTINFO);

    Ok(())
}

#[test]
fn test_supportinfo_v1_writer_complex() -> Result<(), MetadataError> {
    let data = complex_v1_data();

    let mut writer = SupportInfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));
    writer.write_header(&data.current_as)?;
    writer.write_support_info(&data)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();
    let actual = std::str::from_utf8(&buffer)?;

    assert_eq!(actual, COMPLEX_V1_SUPPORTINFO);

    Ok(())
}

#[test]
fn test_supportinfo_v1_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let file_path = working_dir.path().join("support_info.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(file_path)
        .unwrap();

    let mut writer = SupportInfoXml::new_writer(utils::create_xml_writer(f));
    writer.write_header("2024-01-01T00:00:00")?;
    writer.write_support_info(&SupportInfoData {
        current_as: "2024-01-01T00:00:00".to_owned(),
        ..Default::default()
    })?;
    writer.finish()?;

    let mut f = writer.into_inner();
    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();
    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_V1_SUPPORTINFO);

    Ok(())
}

// ----------- V1.0 Reader tests --------------------------------------

#[test]
fn test_supportinfo_v1_reader_empty() -> Result<(), MetadataError> {
    let mut reader =
        SupportInfoXml::new_reader(utils::create_xml_reader(EMPTY_V1_SUPPORTINFO.as_bytes()));

    let result = reader.read()?;
    assert!(result.is_some());

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data.current_as, "2024-01-01T00:00:00");
            assert!(data.lifecycles.is_empty());
            assert!(data.milestones.is_empty());
            assert!(data.support_levels.is_empty());
            assert!(data.packages.is_empty());
            assert!(data.package_classes.is_empty());
            assert!(data.package_origins.is_empty());
            assert!(data.notes.is_empty());
        }
    }

    assert!(reader.read()?.is_none());

    Ok(())
}

#[test]
fn test_supportinfo_v1_reader_no_footer() -> Result<(), MetadataError> {
    let mut reader = SupportInfoXml::new_reader(utils::create_xml_reader(
        EMPTY_V1_SUPPORTINFO_NO_FOOTER.as_bytes(),
    ));

    let result = reader.read()?;
    assert!(result.is_some());
    assert!(matches!(result.unwrap(), SupportInfo::V1(_)));

    Ok(())
}

#[test]
fn test_supportinfo_v1_reader_complex() -> Result<(), MetadataError> {
    let mut reader =
        SupportInfoXml::new_reader(utils::create_xml_reader(COMPLEX_V1_SUPPORTINFO.as_bytes()));

    let result = reader.read()?;
    let expected = complex_v1_data();

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data, expected);
        }
    }

    assert!(reader.read()?.is_none());

    Ok(())
}

// --------- Roundtrip test -----------------------------------

#[test]
fn test_supportinfo_v1_roundtrip() -> Result<(), MetadataError> {
    let original = complex_v1_data();

    let mut writer = SupportInfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));
    writer.write_header(&original.current_as)?;
    writer.write_support_info(&original)?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let mut reader = SupportInfoXml::new_reader(utils::create_xml_reader(buffer.as_slice()));
    let result = reader.read()?;

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data, original);
        }
    }

    Ok(())
}

// ------- Fixture file tests -------------------------------

static FIXTURE_DIR: &str = "tests/assets/supportinfo_fixtures";

#[test]
fn test_supportinfo_fixtures_v1() -> Result<(), MetadataError> {
    let path = format!("{}/test_support_info_v1.xml", FIXTURE_DIR);
    let f = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(f);
    let mut xml_reader = SupportInfoXml::new_reader(utils::create_xml_reader(reader));

    let result = xml_reader.read()?;
    assert!(result.is_some());

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data.current_as, "2024-01-17T00:00:00");
            assert_eq!(data.lifecycles.len(), 3);
            assert_eq!(data.milestones.len(), 3);
            assert_eq!(data.support_levels.len(), 2);
            assert_eq!(data.packages.len(), 4);
            assert_eq!(data.package_classes.len(), 1);
            assert_eq!(data.package_origins.len(), 1);
            assert_eq!(data.notes.len(), 2);

            assert_eq!(data.lifecycles[0].name, "eol");
            assert_eq!(data.lifecycles[1].name, "eol_php81");
            assert_eq!(data.lifecycles[2].name, "eol_python36");

            assert_eq!(data.lifecycles[0].phases.len(), 1);
            assert_eq!(
                data.lifecycles[0].phases[0].start_milestone.as_deref(),
                Some("al2023_ga")
            );
            assert_eq!(data.lifecycles[1].phases.len(), 1);
            assert_eq!(
                data.lifecycles[1].phases[0].start_milestone.as_deref(),
                Some("al2023_ga")
            );
            assert_eq!(data.lifecycles[2].phases.len(), 1);
            assert_eq!(
                data.lifecycles[2].phases[0].start_date.as_deref(),
                Some("2023-03-15")
            );
        }
    }

    Ok(())
}

#[test]
fn test_supportinfo_fixtures_v1_for_conversion() -> Result<(), MetadataError> {
    let path = format!("{}/supportinfo_v1_for_conversion.xml", FIXTURE_DIR);
    let f = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(f);
    let mut xml_reader = SupportInfoXml::new_reader(utils::create_xml_reader(reader));

    let result = xml_reader.read()?;
    assert!(result.is_some());

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data.lifecycles.len(), 3);
            assert_eq!(data.milestones.len(), 2);
            assert_eq!(data.support_levels.len(), 2);
            assert_eq!(data.packages.len(), 4);

            let lc0 = &data.lifecycles[0];
            assert_eq!(lc0.name, "eol_default_lc");
            assert_eq!(lc0.display_name.as_deref(), Some("Al2023 Lifecycle"));
            assert_eq!(lc0.phases.len(), 2);
            assert_eq!(lc0.phases[0].start_milestone.as_deref(), Some("al2023_ga"));
            assert_eq!(lc0.phases[1].start_milestone.as_deref(), Some("al2023_eol"));

            let lc1 = &data.lifecycles[1];
            assert_eq!(lc1.name, "eol_php81_lc");
            assert_eq!(lc1.note.as_deref(), Some("eol_php81"));
            assert_eq!(lc1.phases.len(), 2);
            assert_eq!(lc1.phases[0].start_date.as_deref(), Some("2023-03-15"));
            assert_eq!(lc1.phases[1].start_date.as_deref(), Some("2026-11-25"));

            assert_eq!(
                data.support_levels[0].severities,
                "Low,Medium,Important,Critical"
            );
            assert_eq!(data.support_levels[1].severities, "");
        }
    }

    Ok(())
}

#[test]
fn test_supportinfo_fixtures_al2023() -> Result<(), MetadataError> {
    let path = format!("{}/AL2023-supportinfo-1.0.xml", FIXTURE_DIR);
    let f = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(f);
    let mut xml_reader = SupportInfoXml::new_reader(utils::create_xml_reader(reader));

    let result = xml_reader.read()?;
    assert!(result.is_some());

    match result.unwrap() {
        SupportInfo::V1(data) => {
            assert_eq!(data.current_as, "2026-07-24T22:18:47.708693");
            assert_eq!(data.lifecycles.len(), 54);
            assert_eq!(data.support_levels.len(), 2);
            assert_eq!(data.packages.len(), 14366);
            assert_eq!(data.package_origins.len(), 1);
            assert_eq!(data.package_classes.len(), 1);

            // display_name on phases
            let lc0 = &data.lifecycles[0];
            assert_eq!(lc0.phases[0].display_name.as_deref(), Some("Supported"));
            assert_eq!(lc0.phases[1].display_name.as_deref(), Some("End of Life"));

            // display_name on support levels
            assert_eq!(
                data.support_levels[0].display_name.as_deref(),
                Some("Full Support")
            );
            assert_eq!(
                data.support_levels[1].display_name.as_deref(),
                Some("End of Support")
            );

            // display_name and description on package origin
            let origin = &data.package_origins[0];
            assert_eq!(origin.display_name.as_deref(), Some("Amazon Linux 2023 Core"));
            assert_eq!(
                origin.description.as_deref(),
                Some("Core Amazon Linux 2023 repository containing base OS packages")
            );
            assert_eq!(origin.signing_key.as_deref(), Some("e951904ad832c631"));
        }
    }

    assert!(xml_reader.read()?.is_none());

    Ok(())
}
