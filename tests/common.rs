extern crate rpmrepo;

use once_cell::sync::{Lazy, OnceCell};
use pretty_assertions::assert_eq;
use rpmrepo::metadata::{
    Changelog, Checksum, FileType, FilelistsXml, HeaderRange, Package, PackageFile, Requirement,
    Size, Time, EVR,
};
use std::path::Path;

// const FIXTURE_FILELIST_PATH: &str = "./tests/assets/complex_repo/repodata/filelists.xml.gz";

pub const COMPLEX_REPO_FIXTURE_PATH: &str = "./tests/assets/complex_repo/";
pub const EMPTY_REPO_FIXTURE_PATH: &str = "./tests/assets/empty_repo/";

pub static COMPLEX_PACKAGE: Lazy<Package> = Lazy::new(|| {
    // let mut package = Package::default();
    // package.name = "complex_package".to_owned();
    // package.arch = "x86_64".to_owned();
    Package {
        checksum: Checksum::Sha256(
            "6e46283a16954c9cecd3799246eb1a426d7d8a8b1bc8d57c55c3da4253e200e5".to_owned(),
        ),
        name: "complex-package".to_owned(),
        arch: "x86_64".to_owned(),
        evr: EVR::new("1", "2.3.4", "5.el8"),
        summary: "A package for exercising many different features of RPM metadata".to_owned(),
        description: "Complex package".to_owned(),
        packager: "Michael Bluth".to_owned(),
        url: "http://bobloblaw.com".to_owned(),
        location_href: "complex-package-2.3.4-5.el8.x86_64.rpm".to_owned(),
        time: Time {
            build: 1624680153,
            file: 1624680154,
        },
        size: Size {
            package: 8641,
            installed: 117,
            archive: 932,
        },
        rpm_license: "MPLv2".to_owned(),
        rpm_buildhost: "localhost".to_owned(),
        rpm_sourcerpm: "complex-package-2.3.4-5.el8.src.rpm".to_owned(),
        rpm_group: "Development/Tools".to_owned(),
        rpm_header_range: HeaderRange {
            start: 4504,
            end: 8377,
        },
        rpm_vendor: "Bluth Company".to_owned(),
        rpm_provides: vec![
            Requirement {
                name: "complex-package".to_owned(),
                flags: Some("EQ".to_owned()),
                epoch: Some("1".to_owned()),
                version: Some("2.3.4".to_owned()),
                release: Some("5.el8".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "complex-package(x86-64)".to_owned(),
                flags: Some("EQ".to_owned()),
                epoch: Some("1".to_owned()),
                version: Some("2.3.4".to_owned()),
                release: Some("5.el8".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "laughter".to_owned(),
                flags: Some("EQ".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("33".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "narration(ronhoward)".to_owned(),
                ..Requirement::default()
            },
        ],
        rpm_requires: vec![
            Requirement {
                name: "/usr/sbin/useradd".to_owned(),
                preinstall: Some(true),
                ..Requirement::default()
            },
            Requirement {
                name: "arson".to_owned(),
                flags: Some("GE".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("1.0.0".to_owned()),
                release: Some("1".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "fur".to_owned(),
                flags: Some("LE".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("2".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "staircar".to_owned(),
                flags: Some("LE".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("99.1".to_owned()),
                release: Some("3".to_owned()),
                ..Requirement::default()
            },
        ],
        rpm_conflicts: vec![Requirement {
            name: "foxnetwork".to_owned(),
            flags: Some("GT".to_owned()),
            epoch: Some("0".to_owned()),
            version: Some("5555".to_owned()),
            ..Requirement::default()
        }],
        rpm_obsoletes: vec![
            Requirement {
                name: "bluemangroup".to_owned(),
                flags: Some("LT".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("32.1".to_owned()),
                release: Some("0".to_owned()),
                ..Requirement::default()
            },
            Requirement {
                name: "cornballer".to_owned(),
                flags: Some("LT".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("444".to_owned()),
                ..Requirement::default()
            },
        ],
        rpm_suggests: vec![
            Requirement {
                name: "(bobloblaw >= 1.1 if maritimelaw else anyone < 0.5.1-2)".to_owned(),
                ..Requirement::default()
            },
            Requirement {
                name: "(dove and return)".to_owned(),
                ..Requirement::default()
            },
            Requirement {
                name: "(job or money > 9000)".to_owned(),
                ..Requirement::default()
            },
        ],
        rpm_enhances: vec![Requirement {
            name: "(bananas or magic)".to_owned(),
            ..Requirement::default()
        }],
        rpm_recommends: vec![
            Requirement {
                name: "((hiding and attic) if light-treason)".to_owned(),
                ..Requirement::default()
            },
            Requirement {
                name: "GeneParmesan(PI)".to_owned(),
                ..Requirement::default()
            },
            Requirement {
                name: "yacht".to_owned(),
                flags: Some("GT".to_owned()),
                epoch: Some("9".to_owned()),
                version: Some("11.0".to_owned()),
                release: Some("0".to_owned()),
                ..Requirement::default()
            },
        ],
        rpm_supplements: vec![
            Requirement {
                name: "((hiding and illusion) unless alliance-of-magicians)".to_owned(),
                ..Requirement::default()
            },
            Requirement {
                name: "comedy".to_owned(),
                flags: Some("EQ".to_owned()),
                epoch: Some("0".to_owned()),
                version: Some("11.1".to_owned()),
                release: Some("4".to_owned()),
                ..Requirement::default()
            },
        ],
        rpm_files: vec![
            PackageFile {
                filetype: FileType::File,
                path: "/etc/complex/pkg.cfg".to_owned(),
            },
            PackageFile {
                filetype: FileType::File,
                path: "/usr/bin/complex_a".to_owned(),
            },
            PackageFile {
                filetype: FileType::Dir,
                path: "/usr/share/doc/complex-package".to_owned(),
            },
            PackageFile {
                filetype: FileType::File,
                path: "/usr/share/doc/complex-package/README".to_owned(),
            },
            PackageFile {
                filetype: FileType::Dir,
                path: "/var/lib/complex".to_owned(),
            },
            PackageFile {
                filetype: FileType::Ghost,
                path: "/var/log/complex.log".to_owned(),
            },
        ],
        rpm_changelogs: vec![
            Changelog {
                author: "Lucille Bluth <lucille@bluthcompany.com> - 1.1.1-1".to_owned(),
                date: 1617192000,
                description: "- It's a banana, Michael. How much could it cost, $10?".to_owned(),
            },
            Changelog {
                author: "Job Bluth <job@alliance-of-magicians.com> - 2.2.2-2".to_owned(),
                date: 1619352000,
                description: "- I've made a huge mistake".to_owned(),
            },
            Changelog {
                author: "George Bluth <george@federalprison.gov> - 3.3.3-3".to_owned(),
                date: 1623672000,
                description: "- There’s always money in the banana stand".to_owned(),
            },
        ],
        ..Package::default()
    }
});

pub static RPM_WITH_INVALID_CHARS: Lazy<Package> = Lazy::new(|| Package {
    checksum: Checksum::Sha256(
        "64f1444f8e86a9ae6accdc2c4b12cb4a87fb2414c0998df461a8623a52eb3cc4".to_owned(),
    ),
    name: "rpm-with-invalid-chars".to_owned(),
    arch: "noarch".to_owned(),
    evr: EVR::new("0", "1", "1.fc33"),
    rpm_files: vec![],
    ..Package::default()
});

pub static RPM_EMPTY: Lazy<Package> = Lazy::new(|| Package {
    checksum: Checksum::Sha256(
        "90fbba546300f507473547f33e229ee7bad94bbbe6e84b21d485e8e43b5f1132".to_owned(),
    ),
    name: "rpm-empty".to_owned(),
    arch: "x86_64".to_owned(),
    evr: EVR::new("0", "0", "0"),
    rpm_files: vec![],
    ..Package::default()
});

pub static RPM_WITH_NON_ASCII: Lazy<Package> = Lazy::new(|| Package {
    checksum: Checksum::Sha256(
        "957de8a966af8fe8e55102489099d8b20bbecc23954c8c2bd88fb59625260393".to_owned(),
    ),
    name: "rpm-with-non-ascii".to_owned(),
    arch: "noarch".to_owned(),
    evr: EVR::new("0", "1", "1.fc33"),
    rpm_files: vec![],
    ..Package::default()
});

// /// Fixture should cover all fields / tag types for repomd.xml
// /// Started w/ Fedora 33 updates repodata, added repo, content, distro tags
// /// FilelistsDb covers standard fields + database_version, UpdateInfoZck covers header_size, header_checksum
pub fn complex_repo_fixture_data() -> Vec<&'static Package> {
    // static INSTANCE: OnceCell<&[&Package]> = OnceCell::new();
    // INSTANCE.get_or_init(|| {
    vec![
        &*COMPLEX_PACKAGE,
        &*RPM_WITH_INVALID_CHARS,
        &*RPM_EMPTY,
        &*RPM_WITH_NON_ASCII,
    ]
    // })
}
