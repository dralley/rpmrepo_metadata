// Copyright (c) 2022 Daniel Alley
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::fmt;
use std::path::PathBuf;
use std::sync::Mutex;

use pyo3;
use pyo3::Py;
use pyo3::basic::CompareOp;
use pyo3::prelude::*;

#[pymodule]
mod rpmrepo_metadata {
    use super::*;
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

    /// In-memory representation of a complete RPM repository.
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

            self.inner.write_to_directory_with_options(&path, options)?;
            Ok(())
        }

        #[setter]
        fn set_groups(&mut self, py: Python<'_>, groups: Vec<Py<CompsGroup>>) {
            *self.inner.groups_mut() = groups.iter().map(|g| g.borrow(py).inner.clone()).collect();
        }

        #[setter]
        fn set_categories(&mut self, py: Python<'_>, categories: Vec<Py<CompsCategory>>) {
            *self.inner.categories_mut() = categories
                .iter()
                .map(|c| c.borrow(py).inner.clone())
                .collect();
        }

        #[setter]
        fn set_environments(&mut self, py: Python<'_>, environments: Vec<Py<CompsEnvironment>>) {
            *self.inner.environments_mut() = environments
                .iter()
                .map(|e| e.borrow(py).inner.clone())
                .collect();
        }

        #[setter]
        fn set_langpacks(&mut self, py: Python<'_>, langpacks: Vec<Py<CompsLangpack>>) {
            *self.inner.langpacks_mut() = langpacks
                .iter()
                .map(|l| l.borrow(py).inner.clone())
                .collect();
        }

        #[getter]
        fn groups(&self) -> Vec<CompsGroup> {
            self.inner
                .groups()
                .iter()
                .map(|g| CompsGroup { inner: g.clone() })
                .collect()
        }

        #[getter]
        fn categories(&self) -> Vec<CompsCategory> {
            self.inner
                .categories()
                .iter()
                .map(|c| CompsCategory { inner: c.clone() })
                .collect()
        }

        #[getter]
        fn environments(&self) -> Vec<CompsEnvironment> {
            self.inner
                .environments()
                .iter()
                .map(|e| CompsEnvironment { inner: e.clone() })
                .collect()
        }

        #[getter]
        fn langpacks(&self) -> Vec<CompsLangpack> {
            self.inner
                .langpacks()
                .iter()
                .map(|l| CompsLangpack { inner: l.clone() })
                .collect()
        }
    }

    /// Streaming writer for constructing RPM repository metadata on disk.
    #[pyclass]
    struct RepositoryWriter {
        // finish() is a bit tricky because Python and Rust do not follow the same rules -
        // calling `RepositoryWriter::finish()` destroys the `RepositoryWriter` but the Python
        // object of course sticks around. Thus we must make it optional, so that finish() can
        // properly consume the object.
        inner: Mutex<Option<crate::RepositoryWriter>>,
    }

    #[pymethods]
    impl RepositoryWriter {
        #[new]
        fn new(path: PathBuf, num_pkgs: usize) -> PyResult<Self> {
            let repo_writer = crate::RepositoryWriter::new(&path, num_pkgs)?;
            let py_repo_writer = RepositoryWriter {
                inner: Mutex::new(Some(repo_writer)),
            };
            Ok(py_repo_writer)
        }

        fn add_package(&mut self, pkg: &Package) -> PyResult<()> {
            self.inner.lock().unwrap().as_mut().expect("finish() has already been called - cannot perform action after the repository has already finished being written").add_package(&pkg.inner)?;
            Ok(())
        }

        fn add_advisory(&mut self, record: &UpdateRecord) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .add_advisory(&record.inner)?;
            Ok(())
        }

        fn add_group(&mut self, group: &CompsGroup) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .add_group(&group.inner)?;
            Ok(())
        }

        fn add_category(&mut self, category: &CompsCategory) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .add_category(&category.inner)?;
            Ok(())
        }

        fn add_environment(&mut self, environment: &CompsEnvironment) -> PyResult<()> {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .add_environment(&environment.inner)?;
            Ok(())
        }

        fn set_langpacks(
            &mut self,
            py: Python<'_>,
            langpacks: Vec<Py<CompsLangpack>>,
        ) -> PyResult<()> {
            let l: Vec<_> = langpacks
                .iter()
                .map(|x| x.borrow(py).inner.clone())
                .collect();
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .set_langpacks(&l)?;
            Ok(())
        }

        fn write_comps(
            &mut self,
            py: Python<'_>,
            groups: Vec<Py<CompsGroup>>,
            categories: Vec<Py<CompsCategory>>,
            environments: Vec<Py<CompsEnvironment>>,
            langpacks: Vec<Py<CompsLangpack>>,
        ) -> PyResult<()> {
            let g: Vec<_> = groups.iter().map(|x| x.borrow(py).inner.clone()).collect();
            let c: Vec<_> = categories
                .iter()
                .map(|x| x.borrow(py).inner.clone())
                .collect();
            let e: Vec<_> = environments
                .iter()
                .map(|x| x.borrow(py).inner.clone())
                .collect();
            let l: Vec<_> = langpacks
                .iter()
                .map(|x| x.borrow(py).inner.clone())
                .collect();
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .expect("finish() has already been called")
                .write_comps(&g, &c, &e, &l)?;
            Ok(())
        }

        fn finish(&mut self) -> PyResult<()> {
            self.inner.lock().unwrap().take().unwrap().finish()?;
            Ok(())
        }
    }

    /// Reader for RPM repository metadata from a directory on disk.
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

        fn iter_packages(&self) -> PyResult<PackageIterator> {
            let pkg_reader = self.inner.iter_packages()?;
            let py_pkg_reader = PackageIterator {
                inner: Mutex::new(pkg_reader),
            };
            Ok(py_pkg_reader)
        }

        fn iter_advisories(&self) -> PyResult<UpdateinfoReader> {
            let updateinfo_reader = self.inner.iter_advisories()?;
            let py_updateinfo_reader = UpdateinfoReader {
                inner: Mutex::new(updateinfo_reader),
            };
            Ok(py_updateinfo_reader)
        }

        fn read_comps(&self) -> PyResult<Option<CompsData>> {
            let result = self.inner.read_comps()?;
            Ok(result.map(|comps| CompsData { inner: comps }))
        }
    }

    /// Parsed comps.xml data containing groups, categories, environments, and langpacks.
    #[pyclass]
    struct CompsData {
        inner: crate::CompsData,
    }

    #[pymethods]
    impl CompsData {
        #[staticmethod]
        fn from_xml(xml: &str) -> PyResult<CompsData> {
            let reader = crate::utils::create_xml_reader(xml.as_bytes());
            let data = crate::metadata::CompsXml::read_data(reader)?;
            Ok(CompsData { inner: data })
        }

        fn to_xml(&self) -> PyResult<String> {
            let mut buf = Vec::new();
            let writer = crate::utils::create_xml_writer(&mut buf);
            crate::metadata::CompsXml::write_data(&self.inner, writer)?;
            Ok(String::from_utf8(buf)
                .map_err(|e| pyo3::exceptions::PyUnicodeDecodeError::new_err(e.to_string()))?)
        }

        #[getter]
        fn groups(&self) -> Vec<CompsGroup> {
            self.inner
                .groups
                .iter()
                .map(|g| CompsGroup { inner: g.clone() })
                .collect()
        }

        #[getter]
        fn categories(&self) -> Vec<CompsCategory> {
            self.inner
                .categories
                .iter()
                .map(|c| CompsCategory { inner: c.clone() })
                .collect()
        }

        #[getter]
        fn environments(&self) -> Vec<CompsEnvironment> {
            self.inner
                .environments
                .iter()
                .map(|e| CompsEnvironment { inner: e.clone() })
                .collect()
        }

        #[getter]
        fn langpacks(&self) -> Vec<CompsLangpack> {
            self.inner
                .langpacks
                .iter()
                .map(|l| CompsLangpack { inner: l.clone() })
                .collect()
        }
    }

    /// An RPM package's metadata.
    #[pyclass]
    struct Package {
        inner: crate::Package,
    }

    #[pymethods]
    impl Package {
        #[new]
        fn new() -> Self {
            Self {
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

        #[cfg(feature = "read_rpm")]
        #[staticmethod]
        fn from_file(path: PathBuf) -> PyResult<Self> {
            let pkg = crate::Package::from_file(&path)?;
            Ok(Package { inner: pkg })
        }

        #[cfg(feature = "read_rpm")]
        #[staticmethod]
        #[pyo3(signature = (path, checksum_type=None, location_href=None, location_base=None, changelog_limit=None))]
        fn from_file_with_options(
            path: PathBuf,
            checksum_type: Option<ChecksumType>,
            location_href: Option<String>,
            location_base: Option<String>,
            changelog_limit: Option<usize>,
        ) -> PyResult<Self> {
            let options = crate::PackageOptions {
                checksum_type: checksum_type.map(Into::into).unwrap_or_default(),
                location_href,
                location_base,
                changelog_limit: changelog_limit.unwrap_or(10),
            };
            let pkg = crate::Package::from_file_with_options(&path, options)?;
            Ok(Package { inner: pkg })
        }

        fn evr(&self) -> EVR {
            EVR {
                inner: self.inner.evr.clone(),
            }
        }

        #[setter(name)]
        fn set_name(&mut self, name: &str) {
            self.inner.set_name(name);
        }

        #[getter(name)]
        fn name(&self) -> &str {
            self.inner.name()
        }

        #[setter(epoch)]
        fn set_epoch(&mut self, epoch: u32) {
            self.inner.set_epoch(epoch);
        }

        #[getter(epoch)]
        fn epoch(&self) -> u32 {
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

        // This can be converted back to (&str, &str) after https://github.com/PyO3/pyo3/pull/4390 merges
        #[setter(checksum)]
        pub fn set_checksum(&mut self, checksum: (String, String)) -> PyResult<()> {
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
        pub fn set_rpm_header_range(&mut self, tuple: (u64, u64)) {
            self.inner.set_rpm_header_range(tuple.0, tuple.1);
        }

        #[getter(rpm_header_range)]
        pub fn rpm_header_range(&self) -> (u64, u64) {
            let range = self.inner.rpm_header_range();
            (range.start, range.end)
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
        pub fn set_files(&mut self, file_tuples: Vec<FileTuple>) -> PyResult<()> {
            let mut files = Vec::with_capacity(file_tuples.len());
            for file in file_tuples.iter() {
                files.push(crate::metadata::PackageFile::try_from(file)?);
            }
            self.inner.set_files(files);
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
        pub fn set_files_split(&mut self, file_tuples: Vec<CrFileTuple>) -> PyResult<()> {
            let mut files = Vec::with_capacity(file_tuples.len());
            for file in file_tuples.iter() {
                files.push(crate::metadata::PackageFile::try_from(file)?);
            }
            self.inner.set_files(files);
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
        pub fn set_changelogs(&mut self, changelog_tuples: Vec<ChangelogTuple>) {
            let changelogs: Vec<_> = changelog_tuples
                .into_iter()
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

        fn __str__(&self) -> PyResult<String> {
            Ok(self.to_string())
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(self.to_string())
        }
    }

    impl fmt::Display for Package {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "<Package {}>", self.nevra())
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

    impl From<ChangelogTuple> for crate::metadata::Changelog {
        fn from(tuple: ChangelogTuple) -> Self {
            crate::metadata::Changelog {
                author: tuple.0,
                timestamp: tuple.1,
                description: tuple.2,
            }
        }
    }

    impl From<&crate::metadata::Changelog> for ChangelogTuple {
        fn from(changelog: &crate::metadata::Changelog) -> Self {
            (
                changelog.author.clone(),
                changelog.timestamp,
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
                    )));
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
                    )));
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

    /// Iterator over packages in repository metadata.
    #[pyclass]
    struct PackageIterator {
        inner: Mutex<crate::PackageIterator>,
    }

    #[pymethods]
    impl PackageIterator {
        #[new]
        fn new(
            primary_path: PathBuf,
            filelists_path: PathBuf,
            other_path: PathBuf,
        ) -> PyResult<Self> {
            let py_pkg_reader = Self {
                inner: Mutex::new(crate::PackageIterator::from_files(
                    &primary_path,
                    &filelists_path,
                    &other_path,
                )?),
            };
            Ok(py_pkg_reader)
        }

        fn parse_package(&mut self) -> PyResult<Option<Package>> {
            let pkg = self.inner.lock().unwrap().parse_package()?;
            let py_pkg = pkg.map(|p| Package { inner: p });
            Ok(py_pkg)
        }

        #[getter]
        fn remaining_packages(&self) -> usize {
            self.inner.lock().unwrap().remaining_packages()
        }

        #[getter]
        fn total_packages(&self) -> usize {
            self.inner.lock().unwrap().total_packages()
        }

        fn __length_hint__(&self) -> usize {
            self.inner.lock().unwrap().remaining_packages()
        }

        fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
            slf
        }

        fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<Package>> {
            slf.parse_package()
        }
    }

    /// An advisory (errata) entry from updateinfo.xml.
    #[pyclass]
    struct UpdateRecord {
        inner: crate::UpdateRecord,
    }

    #[pymethods]
    impl UpdateRecord {
        #[new]
        #[pyo3(signature = (id="".to_string(), title="".to_string(), update_type="".to_string(), from_="".to_string(), status="".to_string(), version="".to_string(), severity="".to_string(), summary="".to_string(), description="".to_string(), solution="".to_string(), rights="".to_string(), release="".to_string(), issued_date=None, updated_date=None, pushcount=None))]
        fn new(
            id: String,
            title: String,
            update_type: String,
            from_: String,
            status: String,
            version: String,
            severity: String,
            summary: String,
            description: String,
            solution: String,
            rights: String,
            release: String,
            issued_date: Option<String>,
            updated_date: Option<String>,
            pushcount: Option<String>,
        ) -> Self {
            Self {
                inner: crate::UpdateRecord {
                    id,
                    title,
                    update_type,
                    from: from_,
                    status,
                    version,
                    severity,
                    summary,
                    description,
                    solution,
                    rights,
                    release,
                    issued_date,
                    updated_date,
                    pushcount,
                    references: Vec::new(),
                    pkglist: Vec::new(),
                },
            }
        }

        #[setter(from_)]
        fn set_from(&mut self, val: String) {
            self.inner.from = val;
        }

        #[setter]
        fn set_update_type(&mut self, val: String) {
            self.inner.update_type = val;
        }

        #[setter]
        fn set_status(&mut self, val: String) {
            self.inner.status = val;
        }

        #[setter(version)]
        fn set_version(&mut self, val: String) {
            self.inner.version = val;
        }

        #[setter]
        fn set_id(&mut self, val: String) {
            self.inner.id = val;
        }

        #[setter]
        fn set_title(&mut self, val: String) {
            self.inner.title = val;
        }

        #[setter]
        fn set_issued_date(&mut self, val: Option<String>) {
            self.inner.issued_date = val;
        }

        #[setter]
        fn set_updated_date(&mut self, val: Option<String>) {
            self.inner.updated_date = val;
        }

        #[setter]
        fn set_rights(&mut self, val: String) {
            self.inner.rights = val;
        }

        #[setter(release)]
        fn set_release(&mut self, val: String) {
            self.inner.release = val;
        }

        #[setter]
        fn set_pushcount(&mut self, val: Option<String>) {
            self.inner.pushcount = val;
        }

        #[setter]
        fn set_severity(&mut self, val: String) {
            self.inner.severity = val;
        }

        #[setter]
        fn set_summary(&mut self, val: String) {
            self.inner.summary = val;
        }

        #[setter(description)]
        fn set_description(&mut self, val: String) {
            self.inner.description = val;
        }

        #[setter]
        fn set_solution(&mut self, val: String) {
            self.inner.solution = val;
        }

        #[setter]
        fn set_references(&mut self, py: Python<'_>, refs: Vec<Py<UpdateReference>>) {
            self.inner.references = refs.iter().map(|r| r.borrow(py).inner.clone()).collect();
        }

        #[setter]
        fn set_pkglist(&mut self, py: Python<'_>, pkglist: Vec<Py<UpdateCollection>>) {
            self.inner.pkglist = pkglist.iter().map(|c| c.borrow(py).inner.clone()).collect();
        }

        #[getter(from_)]
        fn from_(&self) -> &str {
            &self.inner.from
        }

        #[getter]
        fn update_type(&self) -> &str {
            &self.inner.update_type
        }

        #[getter]
        fn status(&self) -> &str {
            &self.inner.status
        }

        #[getter]
        fn version(&self) -> &str {
            &self.inner.version
        }

        #[getter]
        fn id(&self) -> &str {
            &self.inner.id
        }

        #[getter]
        fn title(&self) -> &str {
            &self.inner.title
        }

        #[getter]
        fn issued_date(&self) -> Option<&str> {
            self.inner.issued_date.as_deref()
        }

        #[getter]
        fn updated_date(&self) -> Option<&str> {
            self.inner.updated_date.as_deref()
        }

        #[getter]
        fn rights(&self) -> &str {
            &self.inner.rights
        }

        #[getter]
        fn release(&self) -> &str {
            &self.inner.release
        }

        #[getter]
        fn pushcount(&self) -> Option<&str> {
            self.inner.pushcount.as_deref()
        }

        #[getter]
        fn severity(&self) -> &str {
            &self.inner.severity
        }

        #[getter]
        fn summary(&self) -> &str {
            &self.inner.summary
        }

        #[getter]
        fn description(&self) -> &str {
            &self.inner.description
        }

        #[getter]
        fn solution(&self) -> &str {
            &self.inner.solution
        }

        #[getter]
        fn references(&self) -> Vec<UpdateReference> {
            self.inner
                .references
                .iter()
                .map(|r| UpdateReference { inner: r.clone() })
                .collect()
        }

        #[getter]
        fn pkglist(&self) -> Vec<UpdateCollection> {
            self.inner
                .pkglist
                .iter()
                .map(|c| UpdateCollection { inner: c.clone() })
                .collect()
        }

        fn __str__(&self) -> String {
            format!("<UpdateRecord {}>", self.inner.id)
        }

        fn __repr__(&self) -> String {
            format!("<UpdateRecord {}>", self.inner.id)
        }
    }

    /// A reference (bugzilla, CVE, etc.) associated with an advisory.
    #[pyclass]
    struct UpdateReference {
        inner: crate::UpdateReference,
    }

    #[pymethods]
    impl UpdateReference {
        #[new]
        #[pyo3(signature = (href="".to_string(), id="".to_string(), title="".to_string(), reftype="".to_string()))]
        fn new(href: String, id: String, title: String, reftype: String) -> Self {
            Self {
                inner: crate::UpdateReference {
                    href,
                    id,
                    title,
                    reftype,
                },
            }
        }

        #[getter]
        fn href(&self) -> &str {
            &self.inner.href
        }

        #[getter]
        fn id(&self) -> &str {
            &self.inner.id
        }

        #[getter]
        fn title(&self) -> &str {
            &self.inner.title
        }

        #[getter]
        fn reftype(&self) -> &str {
            &self.inner.reftype
        }
    }

    /// A collection of packages affected by an advisory update.
    #[pyclass]
    struct UpdateCollection {
        inner: crate::UpdateCollection,
    }

    #[pymethods]
    impl UpdateCollection {
        #[new]
        #[pyo3(signature = (name="".to_string(), shortname="".to_string()))]
        fn new(name: String, shortname: String) -> Self {
            Self {
                inner: crate::UpdateCollection {
                    name,
                    shortname,
                    packages: Vec::new(),
                    module: None,
                },
            }
        }

        #[setter]
        fn set_packages(&mut self, py: Python<'_>, pkgs: Vec<Py<UpdateCollectionPackage>>) {
            self.inner.packages = pkgs.iter().map(|p| p.borrow(py).inner.clone()).collect();
        }

        #[setter]
        fn set_module(&mut self, module: Option<UpdateCollectionModule>) {
            self.inner.module = module.map(|m| m.inner);
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn shortname(&self) -> &str {
            &self.inner.shortname
        }

        #[getter]
        fn packages(&self) -> Vec<UpdateCollectionPackage> {
            self.inner
                .packages
                .iter()
                .map(|p| UpdateCollectionPackage { inner: p.clone() })
                .collect()
        }

        #[getter]
        fn module(&self) -> Option<UpdateCollectionModule> {
            self.inner
                .module
                .as_ref()
                .map(|m| UpdateCollectionModule { inner: m.clone() })
        }
    }

    /// A package within an advisory update collection.
    #[pyclass]
    struct UpdateCollectionPackage {
        inner: crate::UpdateCollectionPackage,
    }

    #[pymethods]
    impl UpdateCollectionPackage {
        #[new]
        #[pyo3(signature = (name="".to_string(), version="".to_string(), release="".to_string(), arch="".to_string(), epoch="".to_string(), filename="".to_string(), src="".to_string(), reboot_suggested=false, restart_suggested=false, relogin_suggested=false))]
        fn new(
            name: String,
            version: String,
            release: String,
            arch: String,
            epoch: String,
            filename: String,
            src: String,
            reboot_suggested: bool,
            restart_suggested: bool,
            relogin_suggested: bool,
        ) -> Self {
            Self {
                inner: crate::UpdateCollectionPackage {
                    name,
                    version,
                    release,
                    arch,
                    epoch,
                    filename,
                    src,
                    reboot_suggested,
                    restart_suggested,
                    relogin_suggested,
                    checksum: None,
                },
            }
        }

        #[getter]
        fn epoch(&self) -> &str {
            &self.inner.epoch
        }

        #[getter]
        fn filename(&self) -> &str {
            &self.inner.filename
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn reboot_suggested(&self) -> bool {
            self.inner.reboot_suggested
        }

        #[getter]
        fn restart_suggested(&self) -> bool {
            self.inner.restart_suggested
        }

        #[getter]
        fn relogin_suggested(&self) -> bool {
            self.inner.relogin_suggested
        }

        #[getter]
        fn release(&self) -> &str {
            &self.inner.release
        }

        #[getter]
        fn src(&self) -> &str {
            &self.inner.src
        }

        #[getter]
        fn arch(&self) -> &str {
            &self.inner.arch
        }

        #[getter]
        fn checksum(&self) -> Option<(&str, &str)> {
            self.inner
                .checksum
                .as_ref()
                .and_then(|c| c.to_values().ok())
        }

        #[getter]
        fn version(&self) -> &str {
            &self.inner.version
        }
    }

    /// Module stream information for a modular advisory update.
    #[pyclass(from_py_object)]
    #[derive(Clone)]
    struct UpdateCollectionModule {
        inner: crate::UpdateCollectionModule,
    }

    #[pymethods]
    impl UpdateCollectionModule {
        #[new]
        fn new(name: String, stream: String, version: u64, context: String, arch: String) -> Self {
            Self {
                inner: crate::UpdateCollectionModule {
                    name,
                    stream,
                    version,
                    context,
                    arch,
                },
            }
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn stream(&self) -> &str {
            &self.inner.stream
        }

        #[getter]
        fn version(&self) -> u64 {
            self.inner.version
        }

        #[getter]
        fn context(&self) -> &str {
            &self.inner.context
        }

        #[getter]
        fn arch(&self) -> &str {
            &self.inner.arch
        }
    }

    /// Iterator over advisory records from updateinfo.xml.
    #[pyclass]
    struct UpdateinfoReader {
        inner: Mutex<crate::UpdateinfoIterator>,
    }

    #[pymethods]
    impl UpdateinfoReader {
        fn parse_updaterecord(&mut self) -> PyResult<Option<UpdateRecord>> {
            if let Some(rec) = self.inner.lock().unwrap().next() {
                return Ok(Some(UpdateRecord { inner: rec? }));
            }
            Ok(None)
        }

        fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
            slf
        }

        fn __next__(mut slf: PyRefMut<Self>) -> PyResult<Option<UpdateRecord>> {
            slf.parse_updaterecord()
        }
    }

    /// A package group from comps.xml.
    #[pyclass]
    struct CompsGroup {
        inner: crate::CompsGroup,
    }

    #[pymethods]
    impl CompsGroup {
        #[new]
        #[pyo3(signature = (id="".to_string(), name="".to_string(), description="".to_string(), default=false, uservisible=true, biarchonly=false, langonly=None, display_order=None))]
        fn new(
            id: String,
            name: String,
            description: String,
            default: bool,
            uservisible: bool,
            biarchonly: bool,
            langonly: Option<String>,
            display_order: Option<u32>,
        ) -> Self {
            Self {
                inner: crate::CompsGroup {
                    id,
                    name,
                    description,
                    default,
                    uservisible,
                    biarchonly,
                    langonly,
                    display_order,
                    name_by_lang: Vec::new(),
                    desc_by_lang: Vec::new(),
                    packages: Vec::new(),
                },
            }
        }

        #[setter]
        fn set_packages(&mut self, py: Python<'_>, pkgs: Vec<Py<CompsPackageReq>>) {
            self.inner.packages = pkgs.iter().map(|p| p.borrow(py).inner.clone()).collect();
        }

        #[setter]
        fn set_name_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.name_by_lang = val;
        }

        #[setter]
        fn set_desc_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.desc_by_lang = val;
        }

        #[getter]
        fn id(&self) -> &str {
            &self.inner.id
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn name_by_lang(&self) -> Vec<(String, String)> {
            self.inner.name_by_lang.clone()
        }

        #[getter]
        fn description(&self) -> &str {
            &self.inner.description
        }

        #[getter]
        fn desc_by_lang(&self) -> Vec<(String, String)> {
            self.inner.desc_by_lang.clone()
        }

        #[getter]
        fn default(&self) -> bool {
            self.inner.default
        }

        #[getter]
        fn uservisible(&self) -> bool {
            self.inner.uservisible
        }

        #[getter]
        fn biarchonly(&self) -> bool {
            self.inner.biarchonly
        }

        #[getter]
        fn langonly(&self) -> Option<&str> {
            self.inner.langonly.as_deref()
        }

        #[getter]
        fn display_order(&self) -> Option<u32> {
            self.inner.display_order
        }

        #[getter]
        fn packages(&self) -> Vec<CompsPackageReq> {
            self.inner
                .packages
                .iter()
                .map(|p| CompsPackageReq { inner: p.clone() })
                .collect()
        }

        fn __str__(&self) -> String {
            format!("<CompsGroup {}>", self.inner.id)
        }

        fn __repr__(&self) -> String {
            format!("<CompsGroup {}>", self.inner.id)
        }
    }

    /// A package requirement within a comps group.
    #[pyclass]
    struct CompsPackageReq {
        inner: crate::CompsPackageReq,
    }

    #[pymethods]
    impl CompsPackageReq {
        #[new]
        #[pyo3(signature = (name="".to_string(), reqtype="default".to_string(), requires=None, basearchonly=false))]
        fn new(
            name: String,
            reqtype: String,
            requires: Option<String>,
            basearchonly: bool,
        ) -> Self {
            Self {
                inner: crate::CompsPackageReq {
                    name,
                    reqtype,
                    requires,
                    basearchonly,
                },
            }
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn reqtype(&self) -> &str {
            &self.inner.reqtype
        }

        #[getter]
        fn requires(&self) -> Option<&str> {
            self.inner.requires.as_deref()
        }

        #[getter]
        fn basearchonly(&self) -> bool {
            self.inner.basearchonly
        }
    }

    /// A category from comps.xml, organizing groups into higher-level groupings.
    #[pyclass]
    struct CompsCategory {
        inner: crate::CompsCategory,
    }

    #[pymethods]
    impl CompsCategory {
        #[new]
        #[pyo3(signature = (id="".to_string(), name="".to_string(), description="".to_string(), display_order=None))]
        fn new(id: String, name: String, description: String, display_order: Option<u32>) -> Self {
            Self {
                inner: crate::CompsCategory {
                    id,
                    name,
                    description,
                    display_order,
                    name_by_lang: Vec::new(),
                    desc_by_lang: Vec::new(),
                    group_ids: Vec::new(),
                },
            }
        }

        #[setter]
        fn set_group_ids(&mut self, val: Vec<String>) {
            self.inner.group_ids = val;
        }

        #[setter]
        fn set_name_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.name_by_lang = val;
        }

        #[setter]
        fn set_desc_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.desc_by_lang = val;
        }

        #[getter]
        fn id(&self) -> &str {
            &self.inner.id
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn name_by_lang(&self) -> Vec<(String, String)> {
            self.inner.name_by_lang.clone()
        }

        #[getter]
        fn description(&self) -> &str {
            &self.inner.description
        }

        #[getter]
        fn desc_by_lang(&self) -> Vec<(String, String)> {
            self.inner.desc_by_lang.clone()
        }

        #[getter]
        fn display_order(&self) -> Option<u32> {
            self.inner.display_order
        }

        #[getter]
        fn group_ids(&self) -> Vec<String> {
            self.inner.group_ids.clone()
        }

        fn __str__(&self) -> String {
            format!("<CompsCategory {}>", self.inner.id)
        }

        fn __repr__(&self) -> String {
            format!("<CompsCategory {}>", self.inner.id)
        }
    }

    /// An environment from comps.xml, defining a complete installation profile.
    #[pyclass]
    struct CompsEnvironment {
        inner: crate::CompsEnvironment,
    }

    #[pymethods]
    impl CompsEnvironment {
        #[new]
        #[pyo3(signature = (id="".to_string(), name="".to_string(), description="".to_string(), display_order=None))]
        fn new(id: String, name: String, description: String, display_order: Option<u32>) -> Self {
            Self {
                inner: crate::CompsEnvironment {
                    id,
                    name,
                    description,
                    display_order,
                    name_by_lang: Vec::new(),
                    desc_by_lang: Vec::new(),
                    group_ids: Vec::new(),
                    option_ids: Vec::new(),
                },
            }
        }

        #[setter]
        fn set_group_ids(&mut self, val: Vec<String>) {
            self.inner.group_ids = val;
        }

        #[setter]
        fn set_option_ids(&mut self, py: Python<'_>, opts: Vec<Py<CompsEnvironmentOption>>) {
            self.inner.option_ids = opts.iter().map(|o| o.borrow(py).inner.clone()).collect();
        }

        #[setter]
        fn set_name_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.name_by_lang = val;
        }

        #[setter]
        fn set_desc_by_lang(&mut self, val: Vec<(String, String)>) {
            self.inner.desc_by_lang = val;
        }

        #[getter]
        fn id(&self) -> &str {
            &self.inner.id
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn name_by_lang(&self) -> Vec<(String, String)> {
            self.inner.name_by_lang.clone()
        }

        #[getter]
        fn description(&self) -> &str {
            &self.inner.description
        }

        #[getter]
        fn desc_by_lang(&self) -> Vec<(String, String)> {
            self.inner.desc_by_lang.clone()
        }

        #[getter]
        fn display_order(&self) -> Option<u32> {
            self.inner.display_order
        }

        #[getter]
        fn group_ids(&self) -> Vec<String> {
            self.inner.group_ids.clone()
        }

        #[getter]
        fn option_ids(&self) -> Vec<CompsEnvironmentOption> {
            self.inner
                .option_ids
                .iter()
                .map(|o| CompsEnvironmentOption { inner: o.clone() })
                .collect()
        }

        fn __str__(&self) -> String {
            format!("<CompsEnvironment {}>", self.inner.id)
        }

        fn __repr__(&self) -> String {
            format!("<CompsEnvironment {}>", self.inner.id)
        }
    }

    /// An optional group within a comps environment, with a default selection state.
    #[pyclass]
    struct CompsEnvironmentOption {
        inner: crate::CompsEnvironmentOption,
    }

    #[pymethods]
    impl CompsEnvironmentOption {
        #[new]
        #[pyo3(signature = (group_id="".to_string(), default=false))]
        fn new(group_id: String, default: bool) -> Self {
            Self {
                inner: crate::CompsEnvironmentOption { group_id, default },
            }
        }

        #[getter]
        fn group_id(&self) -> &str {
            &self.inner.group_id
        }

        #[getter]
        fn default(&self) -> bool {
            self.inner.default
        }
    }

    /// A langpack mapping from comps.xml.
    #[pyclass]
    struct CompsLangpack {
        inner: crate::CompsLangpack,
    }

    #[pymethods]
    impl CompsLangpack {
        #[new]
        fn new(name: String, install: String) -> Self {
            Self {
                inner: crate::CompsLangpack { name, install },
            }
        }

        #[getter]
        fn name(&self) -> &str {
            &self.inner.name
        }

        #[getter]
        fn install(&self) -> &str {
            &self.inner.install
        }
    }

    /// Checksum algorithm used for package and metadata file verification.
    #[pyclass(eq, eq_int, from_py_object)]
    #[derive(Clone, PartialEq)]
    enum ChecksumType {
        Md5,
        Sha1,
        Sha224,
        Sha256,
        Sha384,
        Sha512,
    }

    impl From<ChecksumType> for crate::ChecksumType {
        fn from(val: ChecksumType) -> Self {
            match val {
                ChecksumType::Md5 => crate::ChecksumType::Md5,
                ChecksumType::Sha1 => crate::ChecksumType::Sha1,
                ChecksumType::Sha224 => crate::ChecksumType::Sha224,
                ChecksumType::Sha256 => crate::ChecksumType::Sha256,
                ChecksumType::Sha384 => crate::ChecksumType::Sha384,
                ChecksumType::Sha512 => crate::ChecksumType::Sha512,
            }
        }
    }

    /// An RPM Epoch-Version-Release version specifier.
    #[pyclass]
    struct EVR {
        inner: crate::EVR,
    }

    #[pymethods]
    impl EVR {
        #[new]
        fn new(epoch: &str, version: &str, release: &str) -> EVR {
            EVR {
                inner: crate::EVR::new(epoch, version, release),
            }
        }

        fn components(&self) -> (&str, &str, &str) {
            (self.epoch(), self.version(), self.release())
        }

        #[staticmethod]
        fn parse(evr: &str) -> PyResult<Self> {
            let py_evr = EVR {
                inner: crate::EVR::parse_values(evr).try_into()?,
            };
            Ok(py_evr)
        }

        #[getter]
        fn epoch(&self) -> &str {
            &self.inner.epoch
        }

        #[getter]
        fn version(&self) -> &str {
            &self.inner.version
        }

        #[getter]
        fn release(&self) -> &str {
            &self.inner.release
        }

        fn __str__(&self) -> PyResult<String> {
            Ok(self.to_string())
        }

        fn __repr__(&self) -> PyResult<String> {
            Ok(self.to_string())
        }

        fn __richcmp__(&self, other: &Self, op: CompareOp) -> PyResult<bool> {
            match op {
                CompareOp::Lt => Ok(self.inner < other.inner),
                CompareOp::Le => Ok(self.inner <= other.inner),
                CompareOp::Eq => Ok(self.inner == other.inner),
                CompareOp::Ne => Ok(self.inner != other.inner),
                CompareOp::Gt => Ok(self.inner > other.inner),
                CompareOp::Ge => Ok(self.inner >= other.inner),
            }
        }
    }

    impl fmt::Display for EVR {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "<EVR ({:?}, {:?}, {:?})>",
                self.epoch(),
                self.version(),
                self.release()
            )
        }
    }
}
