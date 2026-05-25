// Read an RPM repository and print detailed information about its contents:
// packages, advisories, and comps (group/category/environment) data.
//
// Usage: cargo run --example read_repo -- <repo_path>

use std::path::Path;

use rpmrepo_metadata::RepositoryReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: read_repo <repo_path>");
    let base = Path::new(&path);

    let reader = RepositoryReader::new_from_directory(base)?;

    // -- repomd.xml metadata --

    let repomd = reader.repomd();

    if let Some(revision) = repomd.revision() {
        println!("Revision: {revision}");
    }
    println!("Metadata records:");
    for record in repomd.records() {
        print!("  {:16} {}", record.metadata_name, record.location_href.display());
        if record.timestamp != 0 {
            print!("  (timestamp: {})", record.timestamp);
        }
        println!();
    }

    // -- packages --

    println!("\nPackages:");
    let mut pkg_count = 0;
    for result in reader.iter_packages()? {
        let pkg = result?;
        pkg_count += 1;

        println!("  {}", pkg.nevra());
        println!("    Summary:  {}", pkg.summary());
        println!("    Size:     {} bytes (installed: {})", pkg.size_package(), pkg.size_installed());
        println!("    Location: {}", pkg.location_href());
        if !pkg.url().is_empty() {
            println!("    URL:      {}", pkg.url());
        }

        let requires: Vec<_> = pkg.requires().iter().map(|r| r.name.as_str()).collect();
        if !requires.is_empty() {
            println!("    Requires: {}", requires.join(", "));
        }

        let provides: Vec<_> = pkg.provides().iter().map(|r| r.name.as_str()).collect();
        if !provides.is_empty() {
            println!("    Provides: {}", provides.join(", "));
        }

        let files: Vec<_> = pkg.files().iter().map(|f| f.to_path_string()).collect();
        if !files.is_empty() {
            let display: Vec<_> = files.iter().take(10).map(|s| s.as_str()).collect();
            print!("    Files:    {}", display.join(", "));
            if files.len() > 10 {
                print!(" ... and {} more", files.len() - 10);
            }
            println!();
        }

        if !pkg.changelogs().is_empty() {
            println!("    Changelogs: {} entries", pkg.changelogs().len());
            if let Some(latest) = pkg.changelogs().first() {
                println!("      Latest: {} - {}", latest.author, latest.description.lines().next().unwrap_or(""));
            }
        }
    }
    println!("\nTotal packages: {pkg_count}");

    // -- advisories (updateinfo.xml) --

    match reader.iter_advisories() {
        Ok(iter) => {
            println!("\nAdvisories:");
            let mut adv_count = 0;
            for result in iter {
                let advisory = result?;
                adv_count += 1;

                println!(
                    "  [{:10}] {:8} {} - {}",
                    advisory.severity.as_deref().unwrap_or(""), advisory.update_type, advisory.id, advisory.title
                );

                for reference in &advisory.references {
                    println!("    {} {} {}", reference.reftype, reference.id.as_deref().unwrap_or(""), reference.href);
                }

                for collection in &advisory.pkglist {
                    for pkg in &collection.packages {
                        println!(
                            "    Package: {}-{}:{}-{}.{}",
                            pkg.name, pkg.epoch, pkg.version, pkg.release, pkg.arch
                        );
                    }
                }
            }
            println!("Total advisories: {adv_count}");
        }
        Err(_) => println!("\nNo updateinfo.xml found."),
    }

    // -- comps.xml (groups, categories, environments) --

    match reader.read_comps() {
        Ok(Some(comps)) => {
            println!("\nComps data:");

            if !comps.groups.is_empty() {
                println!("  Groups:");
                for group in &comps.groups {
                    let pkg_names: Vec<_> = group.packages.iter().map(|p| p.name.as_str()).collect();
                    println!(
                        "    {} ({}) - {} packages: {}",
                        group.name,
                        group.id,
                        group.packages.len(),
                        pkg_names.join(", ")
                    );
                }
            }

            if !comps.categories.is_empty() {
                println!("  Categories:");
                for cat in &comps.categories {
                    println!("    {} ({}) - groups: {:?}", cat.name, cat.id, cat.group_ids);
                }
            }

            if !comps.environments.is_empty() {
                println!("  Environments:");
                for env in &comps.environments {
                    println!(
                        "    {} ({}) - {} groups, {} optional",
                        env.name,
                        env.id,
                        env.group_ids.len(),
                        env.option_ids.len()
                    );
                }
            }

            if !comps.langpacks.is_empty() {
                println!("  Langpacks:");
                for lp in &comps.langpacks {
                    println!("    {} -> {}", lp.name, lp.install);
                }
            }
        }
        Ok(None) => println!("\nNo comps.xml found."),
        Err(e) => println!("\nError reading comps: {e}"),
    }

    Ok(())
}
