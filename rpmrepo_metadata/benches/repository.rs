use std::path::Path;
use std::rc::Rc;

use criterion::{self, criterion_group, criterion_main, Criterion};
use rpmrepo_metadata::{
    utils, FilelistsXml, OtherXml, PackageParser, PrimaryXml, RepomdXml, Repository,
    RepositoryOptions, RepositoryWriter,
};
use std::fs;
use std::io::{BufReader, Cursor, Read};
use tempdir::TempDir;

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
    let primary: Rc<[u8]> = primary.into_boxed_slice().into();
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
    let filelists: Rc<[u8]> = filelists.into_boxed_slice().into();
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
    let other: Rc<[u8]> = other.into_boxed_slice().into();
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
                Box::new(Cursor::new(primary.clone())) as Box<dyn Read>,
            )));
            let filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(filelists.clone())) as Box<dyn Read>,
            )));
            let other_xml = OtherXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(other.clone())) as Box<dyn Read>,
            )));

            let mut parser =
                PackageParser::from_readers(primary_xml, filelists_xml, other_xml).unwrap();
            while let Some(pkg) = parser.parse_package().unwrap() {
                criterion::black_box(pkg);
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
        let tmp_dir = TempDir::new("prof_repo_write").unwrap();
        let num_pkgs = repo.packages().len();
        let mut repo_writer = RepositoryWriter::new(&tmp_dir.path(), num_pkgs).unwrap();

        b.iter(|| {
            // replace the existing writers w/ memory backed ones
            repo_writer.primary_xml_writer = Some(PrimaryXml::new_writer(
                quick_xml::Writer::new_with_indent(Box::new(Cursor::new(Vec::new())), b' ', 2),
            ));
            repo_writer
                .primary_xml_writer
                .as_mut()
                .unwrap()
                .write_header(num_pkgs)
                .unwrap();
            repo_writer.filelists_xml_writer = Some(FilelistsXml::new_writer(
                quick_xml::Writer::new_with_indent(Box::new(Cursor::new(Vec::new())), b' ', 2),
            ));
            repo_writer
                .filelists_xml_writer
                .as_mut()
                .unwrap()
                .write_header(num_pkgs)
                .unwrap();
            repo_writer.other_xml_writer = Some(OtherXml::new_writer(
                quick_xml::Writer::new_with_indent(Box::new(Cursor::new(Vec::new())), b' ', 2),
            ));
            repo_writer
                .other_xml_writer
                .as_mut()
                .unwrap()
                .write_header(num_pkgs)
                .unwrap();

            for package in repo.packages().values() {
                repo_writer.add_package(package).unwrap();
            }
            repo_writer.finish().unwrap();
        })
    });
}

criterion_group!(benches, metadata_parse_benchmark, metadata_write_benchmark);
criterion_main!(benches);
