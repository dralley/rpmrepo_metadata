use std::path::Path;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rpmrepo_metadata::{Repository, RepositoryOptions, PrimaryXml, FilelistsXml, OtherXml, RepomdXml};
use tempdir::TempDir;
use std::fs;

// fn repository_write_benchmark(c: &mut Criterion) {
//     let mut group = c.benchmark_group("repository_write");
//     group.sample_size(12);

//     let fedora35 = Repository::load_from_directory(Path::new(
//         "./tests/assets/external_repos/fedora35-updates/",
//     ))
//     .unwrap();

//     let options = RepositoryOptions::default();

//     group.bench_function("fedora35-updates", |b| {
//         b.iter(|| {
//             let path = TempDir::new("prof_repo_write").unwrap();
//             fedora35
//                 .write_to_directory(path.path(), options)
//                 .unwrap()
//         })
//     });
// }

const F35_REPO_PATH: &str = "./tests/assets/external_repos/fedora35-updates/";

/// Test parsing metadata
///
/// Benchmark does not perform any IO
fn metadata_parse_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_parse");
    group.sample_size(12);

    let path = Path::new(F35_REPO_PATH);
    let mut repo = Repository::new();
    repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml")).unwrap();

    // Load metadata files to memory, then parse from memory, to avoid IO interactions.
    // Do it in a block to avoid keeping the entire metadata in memory.

    {
        let primary_href = path.join(
            &repo
                .repomd()
                .get_record("primary")
                .unwrap()
                .location_href,
        );
        let primary = fs::read(primary_href).unwrap();
        group.bench_function("primary_xml", |b| {
            b.iter(|| {
                let mut repo = Repository::new();
                repo.load_metadata_bytes::<PrimaryXml>(&primary).unwrap();
            })
        });
    }

    {
        let filelists_href = path.join(
            &repo
                .repomd()
                .get_record("filelists")
                .unwrap()
                .location_href,
        );
        let filelists = fs::read(filelists_href).unwrap();
        group.bench_function("filelists_xml", |b| {
            b.iter(|| {
                let mut repo = Repository::new();
                repo.load_metadata_bytes::<FilelistsXml>(&filelists).unwrap();
            })
        });
    }

    {
        let other_href = path.join(
            &repo
                .repomd()
                .get_record("other")
                .unwrap()
                .location_href,
        );
        let other = fs::read(other_href).unwrap();
        group.bench_function("other_xml", |b| {
            b.iter(|| {
                let mut repo = Repository::new();
                repo.load_metadata_bytes::<OtherXml>(&other).unwrap();
            })
        });
    }
}


/// Test writing metadata out to a memory-backed Vec<u8>
///
/// Benchmark code performs no IO
fn metadata_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_write");
    group.sample_size(12);

    let repo = Repository::load_from_directory(Path::new(F35_REPO_PATH)).unwrap();

    group.bench_function("primary_xml", |b| {
        b.iter(|| {
            repo.write_metadata_bytes::<PrimaryXml>()
        })
    });

    group.bench_function("filelists_xml", |b| {
        b.iter(|| {
            repo.write_metadata_bytes::<FilelistsXml>()
        })
    });

    group.bench_function("other_xml", |b| {
        b.iter(|| {
            repo.write_metadata_bytes::<OtherXml>()
        })
    });
}

criterion_group!(benches, metadata_parse_benchmark, metadata_write_benchmark);
criterion_main!(benches);
