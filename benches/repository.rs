// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::sync::Arc;

use criterion::{self, Criterion, criterion_group, criterion_main};
use rpmrepo_metadata::{
    FilelistsXml, OtherXml, PackageIterator, PrimaryXml, RepomdXml, Repository, utils,
};

struct RepoFixture {
    name: &'static str,
    path: &'static str,
    primary: Arc<[u8]>,
    filelists: Arc<[u8]>,
    other: Arc<[u8]>,
}

impl RepoFixture {
    fn load(name: &'static str, path: &'static str) -> Self {
        let base = Path::new(path);
        let mut repo = Repository::new();
        repo.load_metadata_file::<RepomdXml>(&base.join("repodata/repomd.xml"))
            .unwrap();

        let load_bytes = |record_type: &str| -> Arc<[u8]> {
            let record = repo.repomd().get_record(record_type).unwrap();
            let file_path = base.join(&record.location_href);
            let mut buf = Vec::new();
            utils::reader_from_file(&file_path)
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
            buf.into_boxed_slice().into()
        };

        RepoFixture {
            name,
            path,
            primary: load_bytes("primary"),
            filelists: load_bytes("filelists"),
            other: load_bytes("other"),
        }
    }
}

fn bench_repo_parse(c: &mut Criterion, fixture: &RepoFixture) {
    let mut group = c.benchmark_group(format!("{}/parse", fixture.name));
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(5));

    group.bench_function("primary_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<PrimaryXml>(&fixture.primary)
                .unwrap();
        })
    });

    group.bench_function("filelists_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<FilelistsXml>(&fixture.filelists)
                .unwrap();
        })
    });

    group.bench_function("other_xml", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<OtherXml>(&fixture.other)
                .unwrap();
        })
    });

    group.bench_function("all_together", |b| {
        b.iter(|| {
            let mut repo = Repository::new();
            repo.load_metadata_bytes::<PrimaryXml>(&fixture.primary)
                .unwrap();
            repo.load_metadata_bytes::<FilelistsXml>(&fixture.filelists)
                .unwrap();
            repo.load_metadata_bytes::<OtherXml>(&fixture.other)
                .unwrap();
        })
    });

    group.bench_function("iterative_all_together", |b| {
        b.iter(|| {
            let primary_xml = PrimaryXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(fixture.primary.clone())) as Box<dyn Read + Send>,
            )));
            let filelists_xml = FilelistsXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(fixture.filelists.clone())) as Box<dyn Read + Send>,
            )));
            let other_xml = OtherXml::new_reader(utils::create_xml_reader(BufReader::new(
                Box::new(Cursor::new(fixture.other.clone())) as Box<dyn Read + Send>,
            )));

            let mut parser =
                PackageIterator::from_readers(primary_xml, filelists_xml, other_xml).unwrap();
            while let Some(pkg) = parser.parse_package().unwrap() {
                std::hint::black_box(pkg);
            }
        })
    });

    group.finish();
}

fn bench_repo_write(c: &mut Criterion, fixture: &RepoFixture) {
    let mut group = c.benchmark_group(format!("{}/write", fixture.name));
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(5));

    let repo = Repository::load_from_directory(Path::new(fixture.path)).unwrap();

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

    group.finish();
}

// Fixture selection rationale:
//
// cs10-baseos (4k pkgs): EL stream repo — multiple versions per name, moderate file count.
//   Good baseline for same-name string dedup and typical enterprise workloads.
//
// fedora42 (77k pkgs): Fedora release — one version per name, largest package count.
//   Minimal same-name dedup opportunity, exercises raw throughput at scale.
//
// el9-baseos (14k pkgs): RHEL repo — many accumulated versions per name (avg ~9).
//   High same-name dedup ratio, mid-size. Tests long-lived repos with version history.
//
// grafana (4k pkgs, 94MB compressed filelists): Extreme vendor repo — ~19 unique names
//   with ~215 versions each. Massive file lists with near-identical paths across versions.
//   Stress test for file path memory (dir dedup ~250x, basename dedup ~170x).

fn cs10_baseos(c: &mut Criterion) {
    let fixture = RepoFixture::load(
        "cs10_baseos",
        "./tests/assets/external_repos/centos-stream/cs10-baseos/",
    );
    bench_repo_parse(c, &fixture);
    bench_repo_write(c, &fixture);
}

fn fedora42(c: &mut Criterion) {
    let fixture = RepoFixture::load("fedora42", "./tests/assets/external_repos/fedora/fedora42/");
    bench_repo_parse(c, &fixture);
    bench_repo_write(c, &fixture);
}

fn el9_baseos(c: &mut Criterion) {
    let fixture = RepoFixture::load(
        "el9_baseos",
        "./tests/assets/external_repos/rhel/el9-baseos/",
    );
    bench_repo_parse(c, &fixture);
    bench_repo_write(c, &fixture);
}

fn grafana(c: &mut Criterion) {
    let fixture = RepoFixture::load("grafana", "./tests/assets/external_repos/vendor/grafana/");
    bench_repo_parse(c, &fixture);
    bench_repo_write(c, &fixture);
}

criterion_group!(benches, cs10_baseos, fedora42, el9_baseos, grafana);
criterion_main!(benches);
