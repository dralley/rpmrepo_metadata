// Build an RPM repository from scratch using the RepositoryWriter API.
//
// This example demonstrates constructing repository metadata programmatically,
// including packages (built from RPM files on disk), advisories, and comps data.
//
// Usage: cargo run --example write_repo --features read_rpm -- <output_dir> [rpm_files...]
//
// If no RPM files are provided, an empty repository is created with sample
// advisory and comps metadata.

use std::path::Path;

use rpmrepo_metadata::{
    ChecksumType, CompressionType, CompsCategory, CompsEnvironment, CompsEnvironmentOption,
    CompsGroup, CompsLangpack, CompsPackageReq, Package, RepositoryOptions, RepositoryWriter,
    UpdateCollection, UpdateCollectionPackage, UpdateRecord, UpdateReference,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: write_repo <output_dir> [rpm_files...]");
        std::process::exit(1);
    }
    let output_dir = Path::new(&args[1]);
    let rpm_files = &args[2..];

    // Scan RPM files first to know the package count (required by the writer header).
    let packages: Vec<Package> = rpm_files
        .iter()
        .map(|path| {
            Package::from_file(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
        })
        .collect();

    let options = RepositoryOptions::default()
        .compression_type(CompressionType::Gzip)
        .checksum_type(ChecksumType::Sha256)
        .simple_metadata_filenames(true);

    let mut writer = RepositoryWriter::new_with_options(output_dir, packages.len(), options)?;

    // Write packages
    for pkg in &packages {
        writer.add_package(pkg)?;
        println!("Added package: {}", pkg.nevra());
    }

    // Write a sample advisory
    let advisory = UpdateRecord {
        from: "security@example.com".to_string(),
        status: "final".to_string(),
        update_type: "security".to_string(),
        version: "1".to_string(),
        id: "EXAMPLE-2025:0001".to_string(),
        title: "Important: example security update".to_string(),
        severity: Some("Important".to_string()),
        issued_date: Some("2025-01-15".to_string()),
        updated_date: Some("2025-01-16".to_string()),
        rights: Some("Copyright 2025 Example Corp.".to_string()),
        release: Some("Example Linux 9".to_string()),
        summary: Some("An example security advisory demonstrating the updateinfo API.".to_string()),
        description: Some(
            "This advisory fixes a critical vulnerability in the example package.\n\
             All users are advised to upgrade."
                .to_string(),
        ),
        solution: Some("Update to the latest version.".to_string()),
        references: vec![
            UpdateReference {
                href: "https://cve.example.com/CVE-2025-0001".to_string(),
                id: Some("CVE-2025-0001".to_string()),
                reftype: "cve".to_string(),
                title: "CVE-2025-0001".to_string(),
            },
            UpdateReference {
                href: "https://bugzilla.example.com/12345".to_string(),
                id: Some("12345".to_string()),
                reftype: "bugzilla".to_string(),
                title: "Example package crashes on startup".to_string(),
            },
        ],
        pushcount: None,
        pkglist: vec![UpdateCollection {
            shortname: "example-9".to_string(),
            name: "Example Linux 9".to_string(),
            module: None,
            packages: vec![UpdateCollectionPackage {
                name: "example".to_string(),
                epoch: "0".to_string(),
                version: "1.0.1".to_string(),
                release: "2.el9".to_string(),
                arch: "x86_64".to_string(),
                src: Some("example-1.0.1-2.el9.src.rpm".to_string()),
                filename: "example-1.0.1-2.el9.x86_64.rpm".to_string(),
                checksum: None,
                reboot_suggested: false,
                restart_suggested: false,
                relogin_suggested: false,
            }],
        }],
    };
    writer.add_advisory(&advisory)?;
    println!("Added advisory: {}", advisory.id);

    // Write sample comps data
    let groups = vec![
        CompsGroup {
            id: "core".to_string(),
            name: "Core".to_string(),
            description: "Smallest possible installation.".to_string(),
            default: true,
            uservisible: false,
            packages: vec![
                CompsPackageReq {
                    name: "bash".to_string(),
                    reqtype: "mandatory".to_string(),
                    requires: None,
                    basearchonly: false,
                },
                CompsPackageReq {
                    name: "coreutils".to_string(),
                    reqtype: "mandatory".to_string(),
                    requires: None,
                    basearchonly: false,
                },
                CompsPackageReq {
                    name: "vim-minimal".to_string(),
                    reqtype: "default".to_string(),
                    requires: None,
                    basearchonly: false,
                },
            ],
            ..Default::default()
        },
        CompsGroup {
            id: "development-tools".to_string(),
            name: "Development Tools".to_string(),
            description: "Basic development tools.".to_string(),
            default: false,
            uservisible: true,
            packages: vec![
                CompsPackageReq {
                    name: "gcc".to_string(),
                    reqtype: "mandatory".to_string(),
                    requires: None,
                    basearchonly: false,
                },
                CompsPackageReq {
                    name: "make".to_string(),
                    reqtype: "mandatory".to_string(),
                    requires: None,
                    basearchonly: false,
                },
                CompsPackageReq {
                    name: "gdb".to_string(),
                    reqtype: "optional".to_string(),
                    requires: None,
                    basearchonly: false,
                },
            ],
            ..Default::default()
        },
    ];

    let categories = vec![CompsCategory {
        id: "development".to_string(),
        name: "Development".to_string(),
        description: "Tools for software development.".to_string(),
        group_ids: vec!["development-tools".to_string()],
        ..Default::default()
    }];

    let environments = vec![CompsEnvironment {
        id: "minimal-environment".to_string(),
        name: "Minimal Install".to_string(),
        description: "Basic functionality.".to_string(),
        group_ids: vec!["core".to_string()],
        option_ids: vec![CompsEnvironmentOption {
            group_id: "development-tools".to_string(),
            default: false,
        }],
        ..Default::default()
    }];

    let langpacks = vec![CompsLangpack {
        name: "bash".to_string(),
        install: "bash-langpack-%s".to_string(),
    }];

    writer.write_comps(&groups, &categories, &environments, &langpacks)?;
    println!("Added {} groups, {} categories, {} environments", groups.len(), categories.len(), environments.len());

    writer.finish()?;
    println!("\nRepository written to: {}", output_dir.display());

    Ok(())
}
