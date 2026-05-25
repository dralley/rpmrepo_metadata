// Use the visitor API to stream through primary.xml, filelists.xml, and other.xml
// without building full Package objects. This approach is ideal for large repositories
// where you only need specific fields, or when you want to intern strings directly
// into your own data structures.
//
// Usage: cargo run --example visitor_primary -- <repo_path>

use std::collections::HashMap;
use std::path::Path;

use rpmrepo_metadata::visitor::{
    ChangelogData, FilelistsVisitor, OtherVisitor, PrimaryVisitor, RequirementData,
    parse_filelists, parse_other, parse_primary,
};
use rpmrepo_metadata::{FileType, RepositoryReader};

// ---------------------------------------------------------------------------
// Primary visitor: collect per-package dependency counts and source RPM info
// ---------------------------------------------------------------------------

struct PrimaryStats {
    packages: Vec<PackageInfo>,
    current: PackageInfo,
    arch_counts: HashMap<String, usize>,
}

#[derive(Default, Clone)]
struct PackageInfo {
    name: String,
    evr: String,
    arch: String,
    source_rpm: String,
    num_requires: usize,
    num_provides: usize,
    num_conflicts: usize,
    num_obsoletes: usize,
    num_recommends: usize,
    num_files: usize,
    installed_size: u64,
}

impl PrimaryStats {
    fn new() -> Self {
        Self {
            packages: Vec::new(),
            current: PackageInfo::default(),
            arch_counts: HashMap::new(),
        }
    }
}

impl PrimaryVisitor for PrimaryStats {
    fn begin_package(&mut self, name: &str, arch: &str, _checksum_type: &str, _pkgid: &str) {
        self.current = PackageInfo::default();
        self.current.name = name.to_owned();
        self.current.arch = arch.to_owned();
    }

    fn set_evr(&mut self, epoch: &str, version: &str, release: &str) {
        self.current.evr = if epoch != "0" {
            format!("{epoch}:{version}-{release}")
        } else {
            format!("{version}-{release}")
        };
    }

    fn set_size(&mut self, _package: u64, installed: u64, _archive: u64) {
        self.current.installed_size = installed;
    }

    fn set_rpm_sourcerpm(&mut self, sourcerpm: &str) {
        self.current.source_rpm = sourcerpm.to_owned();
    }

    fn add_require(&mut self, _req: RequirementData<'_>) {
        self.current.num_requires += 1;
    }

    fn add_provide(&mut self, _req: RequirementData<'_>) {
        self.current.num_provides += 1;
    }

    fn add_conflict(&mut self, _req: RequirementData<'_>) {
        self.current.num_conflicts += 1;
    }

    fn add_obsolete(&mut self, _req: RequirementData<'_>) {
        self.current.num_obsoletes += 1;
    }

    fn add_recommend(&mut self, _req: RequirementData<'_>) {
        self.current.num_recommends += 1;
    }

    fn add_file(&mut self, _filetype: FileType, _path: &str) {
        self.current.num_files += 1;
    }

    fn end_package(&mut self) {
        *self
            .arch_counts
            .entry(self.current.arch.clone())
            .or_insert(0) += 1;
        self.packages.push(self.current.clone());
    }
}

// ---------------------------------------------------------------------------
// Filelists visitor: count total files per package (by name)
// ---------------------------------------------------------------------------

struct FilelistsStats {
    current_name: String,
    current_count: usize,
    file_counts: HashMap<String, usize>,
    total_files: usize,
    dir_count: usize,
    ghost_count: usize,
}

impl FilelistsStats {
    fn new() -> Self {
        Self {
            current_name: String::new(),
            current_count: 0,
            file_counts: HashMap::new(),
            total_files: 0,
            dir_count: 0,
            ghost_count: 0,
        }
    }
}

impl FilelistsVisitor for FilelistsStats {
    fn begin_package(&mut self, _pkgid: &str, name: &str, _arch: &str) {
        self.current_name = name.to_owned();
        self.current_count = 0;
    }

    fn add_file(&mut self, filetype: FileType, _path: &str) {
        self.current_count += 1;
        self.total_files += 1;
        match filetype {
            FileType::Dir => self.dir_count += 1,
            FileType::Ghost => self.ghost_count += 1,
            FileType::File => {}
        }
    }

    fn end_package(&mut self) {
        self.file_counts
            .insert(std::mem::take(&mut self.current_name), self.current_count);
    }
}

// ---------------------------------------------------------------------------
// Other visitor: collect changelog statistics
// ---------------------------------------------------------------------------

struct OtherStats {
    total_changelogs: usize,
    max_changelogs: usize,
    max_changelogs_pkg: String,
    current_name: String,
    current_count: usize,
    earliest_timestamp: u64,
    latest_timestamp: u64,
}

impl OtherStats {
    fn new() -> Self {
        Self {
            total_changelogs: 0,
            max_changelogs: 0,
            max_changelogs_pkg: String::new(),
            current_name: String::new(),
            current_count: 0,
            earliest_timestamp: u64::MAX,
            latest_timestamp: 0,
        }
    }
}

impl OtherVisitor for OtherStats {
    fn begin_package(&mut self, _pkgid: &str, name: &str, _arch: &str) {
        self.current_name = name.to_owned();
        self.current_count = 0;
    }

    fn add_changelog(&mut self, changelog: ChangelogData<'_>) {
        self.total_changelogs += 1;
        self.current_count += 1;
        if changelog.timestamp < self.earliest_timestamp {
            self.earliest_timestamp = changelog.timestamp;
        }
        if changelog.timestamp > self.latest_timestamp {
            self.latest_timestamp = changelog.timestamp;
        }
    }

    fn end_package(&mut self) {
        if self.current_count > self.max_changelogs {
            self.max_changelogs = self.current_count;
            self.max_changelogs_pkg = std::mem::take(&mut self.current_name);
        }
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: visitor_primary <repo_path>");
    let base = Path::new(&path);

    let reader = RepositoryReader::new_from_directory(base)?;
    let repomd = reader.repomd();

    // -- Parse primary.xml --
    let primary_href = &repomd.get_record("primary").unwrap().location_href;
    let mut xml_reader = rpmrepo_metadata::utils::xml_reader_from_file(&base.join(primary_href))?;

    let mut primary_stats = PrimaryStats::new();
    let num_pkgs = parse_primary(&mut xml_reader, &mut primary_stats)?;

    println!("=== Primary.xml ===");
    println!("Declared package count: {num_pkgs}");
    println!("Parsed packages: {}", primary_stats.packages.len());
    println!();

    // Architecture breakdown
    let mut archs: Vec<_> = primary_stats.arch_counts.iter().collect();
    archs.sort_by(|a, b| b.1.cmp(a.1));
    println!("Architecture breakdown:");
    for (arch, count) in &archs {
        println!("  {arch:16} {count}");
    }
    println!();

    // Top packages by dependency count
    let mut by_requires = primary_stats.packages.clone();
    by_requires.sort_by(|a, b| b.num_requires.cmp(&a.num_requires));
    println!("Top 10 packages by number of requirements:");
    for pkg in by_requires.iter().take(10) {
        println!(
            "  {}-{}.{}: {} requires, {} provides",
            pkg.name, pkg.evr, pkg.arch, pkg.num_requires, pkg.num_provides
        );
    }
    println!();

    // Largest packages
    let mut by_size = primary_stats.packages.clone();
    by_size.sort_by(|a, b| b.installed_size.cmp(&a.installed_size));
    println!("Top 10 largest packages (installed size):");
    for pkg in by_size.iter().take(10) {
        let size_mb = pkg.installed_size as f64 / (1024.0 * 1024.0);
        println!("  {}-{}.{}: {:.1} MB", pkg.name, pkg.evr, pkg.arch, size_mb);
    }
    println!();

    // Packages with conflicts or obsoletes
    let with_conflicts: Vec<_> = primary_stats
        .packages
        .iter()
        .filter(|p| p.num_conflicts > 0)
        .collect();
    let with_obsoletes: Vec<_> = primary_stats
        .packages
        .iter()
        .filter(|p| p.num_obsoletes > 0)
        .collect();
    println!(
        "Packages with conflicts: {}, with obsoletes: {}",
        with_conflicts.len(),
        with_obsoletes.len()
    );

    // -- Parse filelists.xml --
    if let Some(filelists_record) = repomd.get_record("filelists") {
        let mut xml_reader = rpmrepo_metadata::utils::xml_reader_from_file(
            &base.join(&filelists_record.location_href),
        )?;

        let mut filelists_stats = FilelistsStats::new();
        parse_filelists(&mut xml_reader, &mut filelists_stats)?;

        println!("\n=== Filelists.xml ===");
        println!("Total files: {}", filelists_stats.total_files);
        println!(
            "  Regular files: {}",
            filelists_stats.total_files - filelists_stats.dir_count - filelists_stats.ghost_count
        );
        println!("  Directories:   {}", filelists_stats.dir_count);
        println!("  Ghost files:   {}", filelists_stats.ghost_count);

        let mut by_files: Vec<_> = filelists_stats.file_counts.iter().collect();
        by_files.sort_by(|a, b| b.1.cmp(a.1));
        println!("\nTop 10 packages by file count:");
        for (name, count) in by_files.iter().take(10) {
            println!("  {name:40} {count} files");
        }
    }

    // -- Parse other.xml --
    if let Some(other_record) = repomd.get_record("other") {
        let mut xml_reader =
            rpmrepo_metadata::utils::xml_reader_from_file(&base.join(&other_record.location_href))?;

        let mut other_stats = OtherStats::new();
        parse_other(&mut xml_reader, &mut other_stats)?;

        println!("\n=== Other.xml (changelogs) ===");
        println!("Total changelog entries: {}", other_stats.total_changelogs);
        if other_stats.max_changelogs > 0 {
            println!(
                "Most changelogs: {} ({} entries)",
                other_stats.max_changelogs_pkg, other_stats.max_changelogs
            );
        }
        if other_stats.earliest_timestamp != u64::MAX {
            println!("Earliest timestamp: {}", other_stats.earliest_timestamp);
            println!("Latest timestamp:   {}", other_stats.latest_timestamp);
        }
    }

    Ok(())
}
