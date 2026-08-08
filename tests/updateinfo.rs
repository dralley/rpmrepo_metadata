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

static EMPTY_UPDATEINFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
</updates>
"#;

static EMPTY_UPDATEINFO_NO_FOOTER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
"#;

#[allow(dead_code)]
static EMPTY_UPDATEINFO_NO_DECL: &str = r#"
<updates>
</updates>
"#;

static COMPLEX_UPDATEINFO: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
<update from="updates@fedoraproject.org" status="stable" type="bugfix" version="2.0">
  <id>FEDORA-2020-15f9382449</id>
  <title>nano-4.9.3-1.fc32</title>
  <issued date="2020-05-27 04:10:31"/>
  <rights>Copyright (C) 2020 Red Hat, Inc. and others</rights>
  <release>Fedora 32</release>
  <severity>Moderate</severity>
  <summary>nano-4.9.3-1.fc32 bugfix update</summary>
  <description>Update to nano 4.9.3</description>
  <references/>
  <pkglist/>
</update>
</updates>
"#;

const UPDATEINFO_FIXTURE_PATH: &str = "./tests/assets/updateinfo_fixture.xml";

#[test]
fn test_updateinfo_xml_writer_empty() -> Result<(), MetadataError> {
    let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));

    writer.write_header()?;
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();

    let actual = std::str::from_utf8(&buffer)?;
    let expected = EMPTY_UPDATEINFO;

    assert_eq!(&actual, &expected);

    Ok(())
}

#[test]
fn test_updateinfo_xml_writer_file() -> Result<(), MetadataError> {
    let working_dir = TempDir::new("")?;
    let other_name = working_dir.path().join("updateinfo.xml");

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(other_name)
        .unwrap();

    let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(f));

    writer.write_header().unwrap();
    // writer.write_package(&common::RPM_EMPTY).unwrap();
    writer.finish()?;

    let mut f = writer.into_inner();

    f.seek(SeekFrom::Start(0))?;
    let mut actual = String::new();

    f.read_to_string(&mut actual).unwrap();

    assert_eq!(actual, EMPTY_UPDATEINFO);

    Ok(())
}

#[test]
fn test_updateinfo_xml_read_header() -> Result<(), MetadataError> {
    // Test that the header parses correctly when there are no packages
    let mut updateinfo_xml =
        UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO.as_bytes()));
    assert_eq!(updateinfo_xml.read_update()?, None);

    // Test that the header parses correctly when there are no packages and the footer element doesn't exist (EOF)
    let mut updateinfo_xml = UpdateinfoXml::new_reader(utils::create_xml_reader(
        EMPTY_UPDATEINFO_NO_FOOTER.as_bytes(),
    ));
    assert_eq!(updateinfo_xml.read_update()?, None);

    // Test that the header parses correctly when there is no XML declaration at the top
    let mut updateinfo_xml = UpdateinfoXml::new_reader(utils::create_xml_reader(
        EMPTY_UPDATEINFO_NO_DECL.as_bytes(),
    ));
    assert_eq!(updateinfo_xml.read_update()?, None);

    // Test that the header parses correctly when there are packages
    let mut updateinfo_xml =
        UpdateinfoXml::new_reader(utils::create_xml_reader(COMPLEX_UPDATEINFO.as_bytes()));
    assert!(matches!(updateinfo_xml.read_update()?, Some(_)));

    Ok(())
}

#[test]
fn test_updateinfo_xml_read_updaterecord() -> Result<(), MetadataError> {
    let mut updateinfo_xml =
        UpdateinfoXml::new_reader(utils::create_xml_reader(EMPTY_UPDATEINFO.as_bytes()));
    assert!(matches!(updateinfo_xml.read_update()?, None));

    let mut updateinfo_xml = UpdateinfoXml::new_reader(utils::create_xml_reader(
        EMPTY_UPDATEINFO_NO_FOOTER.as_bytes(),
    ));
    assert!(matches!(updateinfo_xml.read_update()?, None));

    let mut updateinfo_xml =
        UpdateinfoXml::new_reader(utils::create_xml_reader(COMPLEX_UPDATEINFO.as_bytes()));
    assert!(matches!(updateinfo_xml.read_update()?, Some(_)));
    assert!(matches!(updateinfo_xml.read_update()?, None));

    Ok(())
}

#[test]
fn test_updateinfo_xml_read_fixture() -> Result<(), MetadataError> {
    let fixture = std::fs::read_to_string(UPDATEINFO_FIXTURE_PATH).unwrap();
    let mut reader = UpdateinfoXml::new_reader(utils::create_xml_reader(fixture.as_bytes()));

    let mut records = Vec::new();
    while let Some(rec) = reader.read_update()? {
        records.push(rec);
    }
    assert_eq!(records.len(), 4);

    // First advisory: full security advisory with all fields
    let rec = &records[0];
    assert_eq!(rec.from, "security@redhat.com");
    assert_eq!(rec.status, "final");
    assert_eq!(rec.update_type, "security");
    assert_eq!(rec.version, "3");
    assert_eq!(rec.id, "RHSA-2024:1234");
    assert_eq!(rec.title, "Important: kernel security update");
    assert_eq!(rec.issued_date.as_deref(), Some("2024-03-15 00:00:00"));
    assert_eq!(rec.updated_date.as_deref(), Some("2024-03-16 12:00:00"));
    assert_eq!(rec.rights.as_deref(), Some("Copyright 2024 Red Hat, Inc."));
    assert_eq!(rec.release.as_deref(), Some("Red Hat Enterprise Linux 9"));
    assert_eq!(rec.pushcount.as_deref(), Some("2"));
    assert_eq!(rec.severity.as_deref(), Some("Important"));
    assert_eq!(
        rec.summary.as_deref(),
        Some("An update for kernel is now available.")
    );
    assert!(
        rec.description
            .as_deref()
            .unwrap()
            .contains("CVE-2024-0001")
    );
    assert!(
        rec.solution
            .as_deref()
            .unwrap()
            .contains("previously released")
    );
    assert_eq!(
        rec.message.as_deref(),
        Some("A reboot is required to apply this update to the running kernel.")
    );
    assert_eq!(rec.reboot_suggested, true);

    // References
    assert_eq!(rec.references.len(), 3);
    assert_eq!(rec.references[0].reftype, "self");
    assert_eq!(rec.references[0].id.as_deref(), Some("RHSA-2024:1234"));
    assert_eq!(rec.references[1].reftype, "bugzilla");
    assert_eq!(rec.references[1].id.as_deref(), Some("2261234"));
    assert_eq!(
        rec.references[1].href,
        "https://bugzilla.redhat.com/show_bug.cgi?id=2261234"
    );
    assert_eq!(rec.references[2].reftype, "cve");
    assert_eq!(rec.references[2].id.as_deref(), Some("CVE-2024-0001"));

    // Collections and packages
    assert_eq!(rec.pkglist.len(), 1);
    let coll = &rec.pkglist[0];
    assert_eq!(coll.shortname, "rhel-9-baseos");
    assert_eq!(coll.name, "Red Hat Enterprise Linux 9 BaseOS");
    assert!(coll.module.is_none());
    assert_eq!(coll.packages.len(), 2);

    let pkg = &coll.packages[0];
    assert_eq!(pkg.name, "kernel");
    assert_eq!(pkg.version, "5.14.0");
    assert_eq!(pkg.release, "362.24.1.el9_3");
    assert_eq!(pkg.epoch, "0");
    assert_eq!(pkg.arch, "x86_64");
    assert_eq!(
        pkg.src.as_deref(),
        Some("kernel-5.14.0-362.24.1.el9_3.src.rpm")
    );
    assert_eq!(pkg.filename, "kernel-5.14.0-362.24.1.el9_3.x86_64.rpm");
    assert!(pkg.checksum.is_some());
    assert_eq!(pkg.reboot_suggested, true);
    assert_eq!(pkg.restart_suggested, false);
    assert_eq!(pkg.relogin_suggested, false);

    // Second advisory: enhancement, no message, no reboot_suggested
    let rec = &records[1];
    assert_eq!(rec.id, "FEDORA-2024-abc123def4");
    assert_eq!(rec.update_type, "enhancement");
    assert_eq!(rec.message, None);
    assert_eq!(rec.reboot_suggested, false);
    assert_eq!(rec.pkglist[0].packages.len(), 2);
    // Second package has no checksum
    assert!(rec.pkglist[0].packages[1].checksum.is_none());

    // Third advisory: modular with module metadata and restart_suggested
    let rec = &records[2];
    assert_eq!(rec.id, "FEDORA-2024-modular001");
    let coll = &rec.pkglist[0];
    let module = coll.module.as_ref().unwrap();
    assert_eq!(module.name, "perl-DBI");
    assert_eq!(module.stream, "1.643");
    assert_eq!(module.version, 8010020190322130042);
    assert_eq!(module.context, "16b3ab4d");
    assert_eq!(module.arch, "x86_64");
    assert_eq!(coll.packages[0].restart_suggested, true);
    assert_eq!(coll.packages[0].reboot_suggested, false);

    // Fourth advisory: minimal, no optional fields
    let rec = &records[3];
    assert_eq!(rec.id, "FEDORA-2024-minimal001");
    assert_eq!(rec.rights, None);
    assert_eq!(rec.release, None);
    assert_eq!(rec.pushcount, None);
    assert_eq!(rec.severity, None);
    assert_eq!(rec.summary, None);
    assert_eq!(rec.description, None);
    assert_eq!(rec.solution, None);
    assert_eq!(rec.message, None);
    assert_eq!(rec.reboot_suggested, false);
    assert_eq!(rec.updated_date, None);
    assert!(rec.references.is_empty());
    assert!(rec.pkglist.is_empty());

    Ok(())
}

#[test]
fn test_updateinfo_xml_roundtrip() -> Result<(), MetadataError> {
    let fixture = std::fs::read_to_string(UPDATEINFO_FIXTURE_PATH).unwrap();

    // Read
    let mut reader = UpdateinfoXml::new_reader(utils::create_xml_reader(fixture.as_bytes()));
    let mut records = Vec::new();
    while let Some(rec) = reader.read_update()? {
        records.push(rec);
    }

    // Write
    let mut writer = UpdateinfoXml::new_writer(utils::create_xml_writer(Cursor::new(Vec::new())));
    writer.write_header()?;
    for rec in &records {
        writer.write_updaterecord(rec)?;
    }
    writer.finish()?;

    let buffer = writer.into_inner().into_inner();
    let written = std::str::from_utf8(&buffer)?;

    // Re-read
    let mut reader2 = UpdateinfoXml::new_reader(utils::create_xml_reader(written.as_bytes()));
    let mut records2 = Vec::new();
    while let Some(rec) = reader2.read_update()? {
        records2.push(rec);
    }

    assert_eq!(records, records2);

    Ok(())
}

#[test]
fn test_updateinfo_suggested_boolean_formats() -> Result<(), MetadataError> {
    // Test various boolean representations for *_suggested flags.
    // Matches libsolv behavior: first character T/t/1 = true, everything else = false.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<updates>
<update from="test@test.com" status="final" type="bugfix" version="1">
  <id>BOOL-TEST-TRUE-CAPS</id>
  <title>Boolean True</title>
  <issued date="2024-01-01 00:00:00"/>
  <reboot_suggested>True</reboot_suggested>
  <references/>
  <pkglist>
    <collection short="test">
      <name>Test</name>
      <package name="pkg-true-caps" version="1.0" release="1" epoch="0" arch="x86_64">
        <filename>pkg-true-caps-1.0-1.x86_64.rpm</filename>
        <reboot_suggested>True</reboot_suggested>
      </package>
    </collection>
  </pkglist>
</update>
<update from="test@test.com" status="final" type="bugfix" version="1">
  <id>BOOL-TEST-TRUE-LOWER</id>
  <title>Boolean true</title>
  <issued date="2024-01-01 00:00:00"/>
  <reboot_suggested>true</reboot_suggested>
  <references/>
  <pkglist>
    <collection short="test">
      <name>Test</name>
      <package name="pkg-true-lower" version="1.0" release="1" epoch="0" arch="x86_64">
        <filename>pkg-true-lower-1.0-1.x86_64.rpm</filename>
        <reboot_suggested>true</reboot_suggested>
      </package>
    </collection>
  </pkglist>
</update>
<update from="test@test.com" status="final" type="bugfix" version="1">
  <id>BOOL-TEST-ONE</id>
  <title>Boolean 1</title>
  <issued date="2024-01-01 00:00:00"/>
  <reboot_suggested>1</reboot_suggested>
  <references/>
  <pkglist>
    <collection short="test">
      <name>Test</name>
      <package name="pkg-one" version="1.0" release="1" epoch="0" arch="x86_64">
        <filename>pkg-one-1.0-1.x86_64.rpm</filename>
        <reboot_suggested>1</reboot_suggested>
      </package>
    </collection>
  </pkglist>
</update>
<update from="test@test.com" status="final" type="bugfix" version="1">
  <id>BOOL-TEST-FALSE</id>
  <title>Boolean False</title>
  <issued date="2024-01-01 00:00:00"/>
  <reboot_suggested>False</reboot_suggested>
  <references/>
  <pkglist>
    <collection short="test">
      <name>Test</name>
      <package name="pkg-false" version="1.0" release="1" epoch="0" arch="x86_64">
        <filename>pkg-false-1.0-1.x86_64.rpm</filename>
        <reboot_suggested>False</reboot_suggested>
      </package>
    </collection>
  </pkglist>
</update>
<update from="test@test.com" status="final" type="bugfix" version="1">
  <id>BOOL-TEST-ZERO</id>
  <title>Boolean 0</title>
  <issued date="2024-01-01 00:00:00"/>
  <reboot_suggested>0</reboot_suggested>
  <references/>
  <pkglist>
    <collection short="test">
      <name>Test</name>
      <package name="pkg-zero" version="1.0" release="1" epoch="0" arch="x86_64">
        <filename>pkg-zero-1.0-1.x86_64.rpm</filename>
        <reboot_suggested>0</reboot_suggested>
      </package>
    </collection>
  </pkglist>
</update>
</updates>
"#;

    let mut reader = UpdateinfoXml::new_reader(utils::create_xml_reader(xml.as_bytes()));
    let mut records = Vec::new();
    while let Some(rec) = reader.read_update()? {
        records.push(rec);
    }
    assert_eq!(records.len(), 5);

    // "True" -> true (both record and package level)
    assert_eq!(records[0].reboot_suggested, true, "record: True");
    assert_eq!(
        records[0].pkglist[0].packages[0].reboot_suggested, true,
        "package: True"
    );

    // "true" -> true
    assert_eq!(records[1].reboot_suggested, true, "record: true");
    assert_eq!(
        records[1].pkglist[0].packages[0].reboot_suggested, true,
        "package: true"
    );

    // "1" -> true
    assert_eq!(records[2].reboot_suggested, true, "record: 1");
    assert_eq!(
        records[2].pkglist[0].packages[0].reboot_suggested, true,
        "package: 1"
    );

    // "False" -> false
    assert_eq!(records[3].reboot_suggested, false, "record: False");
    assert_eq!(
        records[3].pkglist[0].packages[0].reboot_suggested, false,
        "package: False"
    );

    // "0" -> false
    assert_eq!(records[4].reboot_suggested, false, "record: 0");
    assert_eq!(
        records[4].pkglist[0].packages[0].reboot_suggested, false,
        "package: 0"
    );

    Ok(())
}

#[test]
fn test_updateinfo_iterator() -> Result<(), MetadataError> {
    let fixture = std::fs::read_to_string(UPDATEINFO_FIXTURE_PATH).unwrap();
    let reader = UpdateinfoXml::new_reader(utils::create_xml_reader(fixture.as_bytes()));

    let records: Result<Vec<_>, _> = reader.collect();
    let records = records?;

    assert_eq!(records.len(), 4);
    assert_eq!(records[0].id, "RHSA-2024:1234");
    assert_eq!(records[1].id, "FEDORA-2024-abc123def4");
    assert_eq!(records[2].id, "FEDORA-2024-modular001");
    assert_eq!(records[3].id, "FEDORA-2024-minimal001");

    Ok(())
}
