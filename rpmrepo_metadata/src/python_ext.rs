// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::PathBuf;

use pyo3;
use pyo3::prelude::*;

// Create a Python exception type "MetadataError" to mirror the Rust version
// No sub-types though, just string-ify the error message.
pyo3::create_exception!(
    rpmrepo_metadata,
    MetadataError,
    pyo3::exceptions::PyException
);

impl From<crate::MetadataError> for pyo3::PyErr {
    fn from(err: crate::MetadataError) -> Self {
        match err {
            // TODO: IoError doesn't really belong as part of the (Rust) MetadataError type to begin with,
            // might make sense to split it out
            crate::MetadataError::IoError(err) => {
                pyo3::exceptions::PyOSError::new_err(err.to_string())
            }
            _ => MetadataError::new_err(err.to_string()),
        }
    }
}

#[pyclass]
struct Repository {
    inner: crate::Repository,
}

#[pymethods]
impl Repository {
    #[new]
    fn new() -> Self {
        Repository {
            inner: crate::Repository::new(),
        }
    }

    #[staticmethod]
    fn load_from_directory(path: PathBuf) -> PyResult<Self> {
        let repo = crate::Repository::load_from_directory(&path)?;
        let py_repo = Self { inner: repo };
        Ok(py_repo)
    }

    fn write_to_directory(&self, path: PathBuf) -> PyResult<()> {
        let options = crate::RepositoryOptions::default();

        self.inner.write_to_directory(&path, options)?;
        Ok(())
    }
}

#[pyclass]
struct RepositoryWriter {
    inner: crate::RepositoryWriter,
}

#[pymethods]
impl RepositoryWriter {
    #[new]
    fn new(path: PathBuf, num_pkgs: usize) -> PyResult<Self> {
        let repo_writer = crate::RepositoryWriter::new(&path, num_pkgs)?;
        let py_repo_writer = RepositoryWriter { inner: repo_writer };
        Ok(py_repo_writer)
    }

    fn add_package(&mut self, pkg: &Package) -> PyResult<()> {
        self.inner.add_package(&pkg.inner)?;
        Ok(())
    }

    fn finish(&mut self) -> PyResult<()> {
        self.inner.finish()?;
        Ok(())
    }
}

#[pyclass]
struct RepositoryReader {
    inner: crate::RepositoryReader,
}

#[pymethods]
impl RepositoryReader {
    #[new]
    fn new(path: PathBuf) -> PyResult<Self> {
        let repo_reader = crate::RepositoryReader::new_from_directory(&path)?;
        let py_repo_reader = Self { inner: repo_reader };
        Ok(py_repo_reader)
    }

    fn iter_packages(&self) -> PyResult<PackageParser> {
        let pkg_parser = self.inner.iter_packages()?;
        let py_pkg_parser = PackageParser { inner: pkg_parser };
        Ok(py_pkg_parser)
    }
}
#[pyclass]
struct Package {
    inner: crate::Package,
}

#[pymethods]
impl Package {
    #[new]
    fn new() -> Self {
        Package {
            inner: crate::Package::default(),
        }
    }

    fn nvra(&self) -> String {
        self.inner.nvra()
    }

    fn nevra(&self) -> String {
        self.inner.nevra()
    }

    fn nevra_short(&self) -> String {
        self.inner.nevra_short()
    }

    #[setter(name)]
    fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    #[getter(name)]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[setter(epoch)]
    fn set_epoch(&mut self, epoch: &str) {
        self.inner.set_epoch(epoch);
    }

    #[getter(epoch)]
    fn epoch(&self) -> &str {
        self.inner.epoch()
    }

    #[setter(version)]
    fn set_version(&mut self, version: &str) {
        self.inner.set_version(version);
    }

    #[getter(version)]
    fn version(&self) -> &str {
        self.inner.version()
    }

    #[setter(release)]
    fn set_release(&mut self, release: &str) {
        self.inner.set_release(release);
    }

    #[getter(release)]
    fn release(&self) -> &str {
        self.inner.release()
    }

    #[setter(arch)]
    fn set_arch(&mut self, arch: &str) {
        self.inner.set_arch(arch);
    }

    #[getter(arch)]
    fn arch(&self) -> &str {
        self.inner.arch()
    }

    #[setter(checksum)]
    pub fn set_checksum(&mut self, checksum: (&str, &str)) -> PyResult<()> {
        let checksum = crate::metadata::Checksum::try_create(checksum.0, checksum.1)?;
        self.inner.set_checksum(checksum);
        Ok(())
    }

    #[getter(checksum)]
    pub fn checksum(&self) -> (&str, &str) {
        self.inner.checksum().to_values().unwrap() // TODO this unwrap doesn't really need to exist
    }

    #[getter(pkgid)]
    pub fn pkgid(&self) -> &str {
        self.inner.pkgid()
    }

    #[getter(checksum_type)]
    pub fn checksum_type(&self) -> &str {
        self.inner.checksum.to_values().unwrap().0
    }

    #[setter(location_href)]
    pub fn set_location_href(&mut self, location_href: &str) {
        self.inner.set_location_href(location_href);
    }

    #[getter(location_href)]
    pub fn location_href(&self) -> &str {
        self.inner.location_href()
    }

    #[setter(location_base)]
    pub fn set_location_base(&mut self, location_base: Option<&str>) {
        self.inner.set_location_base(location_base);
    }

    #[getter(location_base)]
    pub fn location_base(&self) -> Option<&str> {
        self.inner.location_base()
    }

    #[setter(summary)]
    pub fn set_summary(&mut self, summary: &str) {
        self.inner.set_summary(summary);
    }

    #[getter(summary)]
    pub fn summary(&self) -> &str {
        self.inner.summary()
    }

    #[setter(description)]
    pub fn set_description(&mut self, description: &str) {
        self.inner.set_description(description);
    }

    #[getter(description)]
    pub fn description(&self) -> &str {
        self.inner.description()
    }

    #[setter(packager)]
    pub fn set_packager(&mut self, packager: &str) {
        self.inner.set_packager(packager);
    }

    #[getter(packager)]
    pub fn packager(&self) -> &str {
        self.inner.packager()
    }

    #[setter(url)]
    pub fn set_url(&mut self, url: &str) {
        self.inner.set_url(url);
    }

    #[getter(url)]
    pub fn url(&self) -> &str {
        self.inner.url()
    }

    #[setter(time_file)]
    pub fn set_time_file(&mut self, time_file: u64) {
        self.inner.set_time_file(time_file);
    }

    #[getter(time_file)]
    pub fn time_file(&self) -> u64 {
        self.inner.time_file()
    }

    #[setter(time_build)]
    pub fn set_time_build(&mut self, time_build: u64) {
        self.inner.set_time_build(time_build);
    }

    #[getter(time_build)]
    pub fn time_build(&self) -> u64 {
        self.inner.time_build()
    }

    #[setter(size_package)]
    pub fn set_size_package(&mut self, size_package: u64) {
        self.inner.set_size_package(size_package);
    }

    #[getter(size_package)]
    pub fn size_package(&self) -> u64 {
        self.inner.size_package()
    }

    #[setter(size_installed)]
    pub fn set_size_installed(&mut self, size_installed: u64) {
        self.inner.set_size_installed(size_installed);
    }

    #[getter(size_installed)]
    pub fn size_installed(&self) -> u64 {
        self.inner.size_installed()
    }

    #[setter(size_archive)]
    pub fn set_size_archive(&mut self, size_archive: u64) {
        self.inner.set_size_archive(size_archive);
    }

    #[getter(size_archive)]
    pub fn size_archive(&self) -> u64 {
        self.inner.size_archive()
    }

    #[setter(rpm_license)]
    pub fn set_rpm_license(&mut self, license: &str) {
        self.inner.set_rpm_license(license);
    }

    #[getter(rpm_license)]
    pub fn rpm_license(&self) -> &str {
        self.inner.rpm_license()
    }

    #[setter(rpm_vendor)]
    pub fn set_rpm_vendor(&mut self, vendor: &str) {
        self.inner.set_rpm_vendor(vendor);
    }

    #[getter(rpm_vendor)]
    pub fn rpm_vendor(&self) -> &str {
        self.inner.rpm_vendor()
    }

    #[setter(rpm_group)]
    pub fn set_rpm_group(&mut self, group: &str) {
        self.inner.set_rpm_group(group);
    }

    #[getter(rpm_group)]
    pub fn rpm_group(&self) -> &str {
        self.inner.rpm_group()
    }

    #[setter(rpm_buildhost)]
    pub fn set_rpm_buildhost(&mut self, rpm_buildhost: &str) {
        self.inner.set_rpm_buildhost(rpm_buildhost);
    }

    #[getter(rpm_buildhost)]
    pub fn rpm_buildhost(&self) -> &str {
        self.inner.rpm_buildhost()
    }

    #[setter(rpm_sourcerpm)]
    pub fn set_rpm_sourcerpm(&mut self, rpm_sourcerpm: &str) {
        self.inner.set_rpm_sourcerpm(rpm_sourcerpm);
    }

    #[getter(rpm_sourcerpm)]
    pub fn rpm_sourcerpm(&self) -> &str {
        self.inner.rpm_sourcerpm()
    }

    #[setter(rpm_header_range)]
    pub fn set_rpm_header_range(&mut self, tuple: (u64, u64)) -> PyResult<()> {
        self.inner.set_rpm_header_range(tuple.0, tuple.1);
        Ok(())
    }

    #[getter(rpm_header_range)]
    pub fn rpm_header_range(&self) -> PyResult<(u64, u64)> {
        let range = self.inner.rpm_header_range();
        Ok((range.start, range.end))
    }

    #[setter(requires)]
    pub fn set_requires(&mut self, requires: Vec<RequirementTuple>) {
        let requires: Vec<_> = requires
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_requires(requires);
    }

    #[getter(requires)]
    pub fn requires(&self) -> Vec<RequirementTuple> {
        self.inner
            .requires()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(provides)]
    pub fn set_provides(&mut self, provides: Vec<RequirementTuple>) {
        let provides: Vec<_> = provides
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_provides(provides);
    }

    #[getter(provides)]
    pub fn provides(&self) -> Vec<RequirementTuple> {
        self.inner
            .provides()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(conflicts)]
    pub fn set_conflicts(&mut self, conflicts: Vec<RequirementTuple>) {
        let conflicts: Vec<_> = conflicts
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_conflicts(conflicts);
    }

    #[getter(conflicts)]
    pub fn conflicts(&self) -> Vec<RequirementTuple> {
        self.inner
            .conflicts()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(obsoletes)]
    pub fn set_obsoletes(&mut self, obsoletes: Vec<RequirementTuple>) {
        let obsoletes: Vec<_> = obsoletes
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_obsoletes(obsoletes);
    }

    #[getter(obsoletes)]
    pub fn obsoletes(&self) -> Vec<RequirementTuple> {
        self.inner
            .obsoletes()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(suggests)]
    pub fn set_suggests(&mut self, suggests: Vec<RequirementTuple>) {
        let suggests: Vec<_> = suggests
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_suggests(suggests);
    }

    #[getter(suggests)]
    pub fn suggests(&self) -> Vec<RequirementTuple> {
        self.inner
            .suggests()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(enhances)]
    pub fn set_enhances(&mut self, enhances: Vec<RequirementTuple>) {
        let enhances: Vec<_> = enhances
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_enhances(enhances);
    }

    #[getter(enhances)]
    pub fn enhances(&self) -> Vec<RequirementTuple> {
        self.inner
            .enhances()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(recommends)]
    pub fn set_recommends(&mut self, recommends: Vec<RequirementTuple>) {
        let recommends: Vec<_> = recommends
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_recommends(recommends);
    }

    #[getter(recommends)]
    pub fn recommends(&self) -> Vec<RequirementTuple> {
        self.inner
            .recommends()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(supplements)]
    pub fn set_supplements(&mut self, supplements: Vec<RequirementTuple>) {
        let supplements: Vec<_> = supplements
            .iter()
            .map(|r| crate::metadata::Requirement::from(r))
            .collect();
        self.inner.set_supplements(supplements);
    }

    #[getter(supplements)]
    pub fn supplements(&self) -> Vec<RequirementTuple> {
        self.inner
            .supplements()
            .iter()
            .map(|r| RequirementTuple::from(r))
            .collect()
    }

    #[setter(files)]
    pub fn set_files(&mut self, files: Vec<FileTuple>) -> PyResult<()> {
        let mut rusty_files: Vec<crate::metadata::PackageFile> = Vec::with_capacity(files.len());
        for file in files.iter() {
            rusty_files.push(crate::metadata::PackageFile::try_from(file)?);
        }
        self.inner.set_files(rusty_files);
        Ok(())
    }

    #[getter(files)]
    pub fn files(&self) -> Vec<FileTuple> {
        self.inner
            .files()
            .iter()
            .map(|r| FileTuple::from(r))
            .collect()
    }

    #[setter(files_split)]
    pub fn set_files_split(&mut self, files: Vec<CrFileTuple>) -> PyResult<()> {
        let mut rusty_files: Vec<crate::metadata::PackageFile> = Vec::with_capacity(files.len());
        for file in files.iter() {
            rusty_files.push(crate::metadata::PackageFile::try_from(file)?);
        }
        self.inner.set_files(rusty_files);
        Ok(())
    }

    #[getter(files_split)]
    pub fn files_split(&self) -> Vec<CrFileTuple> {
        self.inner
            .files()
            .iter()
            .map(|r| CrFileTuple::from(r))
            .collect()
    }

    #[setter(changelogs)]
    pub fn set_changelogs(&mut self, changelogs: Vec<ChangelogTuple>) {
        let changelogs: Vec<_> = changelogs
            .iter()
            .map(|r| crate::metadata::Changelog::from(r))
            .collect();
        self.inner.set_changelogs(changelogs);
    }

    #[getter(changelogs)]
    pub fn changelogs(&self) -> Vec<ChangelogTuple> {
        self.inner
            .changelogs()
            .iter()
            .map(|r| ChangelogTuple::from(r))
            .collect()
    }
}

#[pyproto]
impl pyo3::PyObjectProtocol for Package {
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("<Package {}>", self.nevra()))
    }
}

// name, flags, epoch, version, release, preinstall
type RequirementTuple = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    bool,
);

// TODO: figure out how to do this without cloning?
impl From<&RequirementTuple> for crate::metadata::Requirement {
    fn from(tuple: &RequirementTuple) -> Self {
        crate::metadata::Requirement {
            name: tuple.0.clone(),
            flags: tuple.1.clone(),
            epoch: tuple.2.clone(),
            version: tuple.3.clone(),
            release: tuple.4.clone(),
            preinstall: tuple.5,
        }
    }
}

impl From<&crate::metadata::Requirement> for RequirementTuple {
    fn from(req: &crate::metadata::Requirement) -> Self {
        (
            req.name.clone(),
            req.flags.clone(),
            req.epoch.clone(),
            req.version.clone(),
            req.release.clone(),
            req.preinstall.clone(),
        )
    }
}

// Author, Date, Description
type ChangelogTuple = (String, u64, String);

impl From<&ChangelogTuple> for crate::metadata::Changelog {
    fn from(tuple: &ChangelogTuple) -> Self {
        crate::metadata::Changelog {
            author: tuple.0.clone(),
            date: tuple.1,
            description: tuple.2.clone(),
        }
    }
}

impl From<&crate::metadata::Changelog> for ChangelogTuple {
    fn from(changelog: &crate::metadata::Changelog) -> Self {
        (
            changelog.author.clone(),
            changelog.date,
            changelog.description.clone(),
        )
    }
}

// Type, path
type FileTuple = (Option<String>, String);

impl TryFrom<&FileTuple> for crate::metadata::PackageFile {
    type Error = pyo3::PyErr;

    fn try_from(tuple: &FileTuple) -> Result<Self, pyo3::PyErr> {
        let ftype = match tuple.0.as_deref() {
            None => crate::metadata::FileType::File,
            Some("dir") => crate::metadata::FileType::Dir,
            Some("ghost") => crate::metadata::FileType::Ghost,
            Some(bad_val) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "{} is not a permitted file type",
                    bad_val
                )))
            }
        };
        let pkgfile = crate::metadata::PackageFile {
            filetype: ftype,
            path: tuple.1.clone(),
        };
        Ok(pkgfile)
    }
}

impl From<&crate::metadata::PackageFile> for FileTuple {
    fn from(pkgfile: &crate::metadata::PackageFile) -> Self {
        let filetype = match pkgfile.filetype {
            crate::metadata::FileType::File => None,
            crate::metadata::FileType::Dir => Some("dir".to_owned()),
            crate::metadata::FileType::Ghost => Some("ghost".to_owned()),
        };

        (filetype, pkgfile.path.clone())
    }
}

// Type, basedir, filename
type CrFileTuple = (Option<String>, String, String);

impl TryFrom<&CrFileTuple> for crate::metadata::PackageFile {
    type Error = pyo3::PyErr;

    fn try_from(tuple: &CrFileTuple) -> Result<Self, pyo3::PyErr> {
        let ftype = match tuple.0.as_deref() {
            None => crate::metadata::FileType::File,
            Some("dir") => crate::metadata::FileType::Dir,
            Some("ghost") => crate::metadata::FileType::Ghost,
            Some(bad_val) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "'{}' is not a permitted file type",
                    bad_val
                )))
            }
        };
        let pkgfile = crate::metadata::PackageFile {
            filetype: ftype,
            path: [tuple.1.as_str(), tuple.2.as_str()].join("/"),
        };
        Ok(pkgfile)
    }
}

impl From<&crate::metadata::PackageFile> for CrFileTuple {
    fn from(pkgfile: &crate::metadata::PackageFile) -> Self {
        let (base, file) = if let Some(idx) = pkgfile.path.rfind('/') {
            pkgfile.path.split_at(idx + 1)
        } else {
            ("", pkgfile.path.as_str())
        };

        let filetype = match pkgfile.filetype {
            crate::metadata::FileType::File => None,
            crate::metadata::FileType::Dir => Some("dir".to_owned()),
            crate::metadata::FileType::Ghost => Some("ghost".to_owned()),
        };

        (filetype, base.to_owned(), file.to_owned())
    }
}

#[pyclass]
struct PackageParser {
    inner: crate::PackageParser,
}

#[pymethods]
impl PackageParser {
    #[new]
    pub fn new(primary_path: PathBuf, filelists_path: PathBuf, other_path: PathBuf) -> PyResult<Self> {
        let py_pkg_parser = Self {
            inner: crate::PackageParser::from_files(&primary_path, &filelists_path, &other_path)?,
        };
        Ok(py_pkg_parser)
    }

    fn parse_package(&mut self) -> PyResult<Option<Package>> {
        let pkg = self.inner.parse_package()?;
        let py_pkg = pkg.map(|p| Package { inner: p });
        Ok(py_pkg)
    }

    #[getter]
    fn remaining_packages(&self) -> usize {
        self.inner.remaining_packages()
    }

    #[getter]
    fn total_packages(&self) -> usize {
        self.inner.total_packages()
    }

    fn __length_hint__(&self) -> usize {
        self.inner.remaining_packages()
    }
}

#[pyproto]
impl pyo3::PyIterProtocol for PackageParser {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<Package>> {
        slf.parse_package()
    }
}

#[pymodule]
fn rpmrepo_metadata(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_class::<Repository>()?;
    m.add_class::<RepositoryWriter>()?;
    m.add_class::<RepositoryReader>()?;
    // m.add_class::<RepositoryOptions>()?;
    m.add_class::<Package>()?;
    m.add_class::<PackageParser>()?;
    // m.add_class::<RepomdXml>()?;
    // m.add_class::<PrimaryXml>()?;
    // m.add_class::<FilelistsXml>()?;
    // m.add_class::<OtherXml>()?;

    Ok(())
}
