use std::path::Path;
use std::sync::Arc;

use criterion::{self, criterion_group, criterion_main, Criterion};
use rpmrepo_metadata::{
    utils, FilelistsXml, OtherXml, PackageParser, PrimaryXml, RepomdXml, Repository,
};
use std::io::{BufReader, Cursor, Read};

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

const REPO_PATH: &str = "./tests/assets/external_repos/fedora35-updates/";

/// Test parsing metadata
///
/// Benchmark does not perform any IO
fn metadata_parse_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_parse");
    group.sample_size(20);

    let path = Path::new(REPO_PATH);
    let mut repo = Repository::new();
    repo.load_metadata_file::<RepomdXml>(&path.join("repodata/repomd.xml"))
        .unwrap();

    // Load metadata files to memory, then parse from memory, to avoid IO interactions.

    let primary_path = path.join(&repo.repomd().get_record("primary").unwrap().location_href);
    let mut primary = Vec::new();
    utils::reader_from_file(&primary_path)
        .unwrap()
        .read_to_end(&mut primary)
        .unwrap();
    let primary: Arc<[u8]> = primary.into_boxed_slice().into();
    group.bench_function("primary_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<PrimaryXml>(&primary).unwrap();
        })
    });

    let filelists_path = path.join(&repo.repomd().get_record("filelists").unwrap().location_href);
    let mut filelists = Vec::new();
    utils::reader_from_file(&filelists_path)
        .unwrap()
        .read_to_end(&mut filelists)
        .unwrap();
    let filelists: Arc<[u8]> = filelists.into_boxed_slice().into();
    group.bench_function("filelists_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<FilelistsXml>(&filelists).unwrap();
        })
    });

    let other_path = path.join(&repo.repomd().get_record("other").unwrap().location_href);
    let mut other = Vec::new();
    utils::reader_from_file(&other_path)
        .unwrap()
        .read_to_end(&mut other)
        .unwrap();
    let other: Arc<[u8]> = other.into_boxed_slice().into();
    group.bench_function("other_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<OtherXml>(&other).unwrap();
        })
    });

    group.bench_function("all_together", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<PrimaryXml>(&primary).unwrap();
            repo.load_metadata_bytes::<FilelistsXml>(&filelists).unwrap();
            repo.load_metadata_bytes::<OtherXml>(&other).unwrap();
        })
    });

    group.bench_function("iterative_all_together", |b| {
        b.iter(|| {
            let primary_xml = PrimaryXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(primary.clone())) as Box<dyn Read + Send>,
            )));
            let filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(filelists.clone())) as Box<dyn Read + Send>,
            )));
            let other_xml = OtherXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(other.clone())) as Box<dyn Read + Send>,
            )));

            let mut parser =
                PackageParser::from_readers(primary_xml, filelists_xml, other_xml).unwrap();
            while let Some(pkg) = parser.parse_package().unwrap() {
                criterion::black_box(pkg);
            }
        })
    });

    group.bench_function("iterative_all_together_manual", |b| {
        b.iter(|| {
            let mut primary_xml = PrimaryXml::new_reader(utils::create_xml_reader(&*primary));
            let mut filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(&*filelists));
            let mut other_xml = OtherXml::new_reader(utils::create_xml_reader(&*other));

            primary_xml.read_header().unwrap();
            filelists_xml.read_header().unwrap();
            other_xml.read_header().unwrap();

            let mut in_progress_package = None;

            loop {
                primary_xml
                    .read_package(&mut in_progress_package).unwrap();
                filelists_xml
                    .read_package(&mut in_progress_package).unwrap();
                other_xml.read_package(&mut in_progress_package).unwrap();

                let package = in_progress_package.take();
                match package {
                    Some(package) => { criterion::black_box(package); },
                    None => break,
                }
            }
        })
    });

}

/// Test writing metadata out to a memory-backed Vec<u8>
///
/// Benchmark code performs no IO
fn metadata_write_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_write");
    group.sample_size(20);

    let repo = Repository::load_from_directory(Path::new(REPO_PATH)).unwrap();

    group.bench_function("primary_xml", |b| {
        b.iter(|| repo.write_metadata_bytes::<PrimaryXml>())
    });

    group.bench_function("filelists_xml", |b| {
        b.iter(|| repo.write_metadata_bytes::<FilelistsXml>())
    });

    group.bench_function("other_xml", |b| {
        b.iter(|| repo.write_metadata_bytes::<OtherXml>())
    });

    group.bench_function("all_together", |b| {
        b.iter(|| {
            repo.write_metadata_bytes::<PrimaryXml>().unwrap();
            repo.write_metadata_bytes::<FilelistsXml>().unwrap();
            repo.write_metadata_bytes::<OtherXml>().unwrap();
        })
    });

    group.bench_function("iterative_all_together", |b| {
        let num_pkgs = repo.packages().values().count();

        b.iter(|| {
            let mut primary_xml_writer = PrimaryXml::new_writer(utils::create_xml_writer(Vec::new()));
            let mut filelists_xml_writer = FilelistsXml::new_writer(utils::create_xml_writer(Vec::new()));
            let mut other_xml_writer = OtherXml::new_writer(utils::create_xml_writer(Vec::new()));

            primary_xml_writer.write_header(num_pkgs).unwrap();
            filelists_xml_writer.write_header(num_pkgs).unwrap();
            other_xml_writer.write_header(num_pkgs).unwrap();

            for package in repo.packages().values() {
                primary_xml_writer.write_package(package).unwrap();
                filelists_xml_writer.write_package(package).unwrap();
                other_xml_writer.write_package(package).unwrap();
            }

            primary_xml_writer.finish().unwrap();
            filelists_xml_writer.finish().unwrap();
            other_xml_writer.finish().unwrap();
        })
    });
}

criterion_group!(benches, metadata_parse_benchmark, metadata_write_benchmark);
criterion_main!(benches);
