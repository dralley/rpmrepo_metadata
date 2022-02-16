use std::path::Path;

use pyo3;
use pyo3::prelude::*;

use crate as rpmrepo_metadata;

fn into_pyerr(err: rpmrepo_metadata::MetadataError) -> pyo3::PyErr {
    match err {
        // TODO: better error handling than "shove the error message into a value error"
        // TODO: better way to do this than .map_err(into_pyerr)?  orphan trait rules are a pain
        _ => pyo3::exceptions::PyValueError::new_err(err.to_string())
    }
}

#[pyclass]
struct Repository {
    inner: rpmrepo_metadata::Repository,
}

#[pymethods]
impl Repository {
    #[new]
    fn new() -> Self {
        Repository {
            inner: rpmrepo_metadata::Repository::new(),
        }
    }

    #[staticmethod]
    fn load_from_directory(path: &str) -> PyResult<Self> {
        let path = Path::new(path);
        let repo = rpmrepo_metadata::Repository::load_from_directory(&path).map_err(into_pyerr)?;
        let py_repo = Self { inner: repo };
        Ok(py_repo)
    }

    fn write_to_directory(&self, path: &str) -> PyResult<()> {
        let path = Path::new(path);
        let options = rpmrepo_metadata::RepositoryOptions::default();

        self.inner
            .write_to_directory(&path, options)
            .map_err(into_pyerr)?;
        Ok(())
    }
}

#[pyclass]
struct RepositoryWriter {
    inner: rpmrepo_metadata::RepositoryWriter,
}

#[pymethods]
impl RepositoryWriter {
    #[new]
    fn new(path: &str, num_pkgs: usize) -> PyResult<Self> {
        let path = Path::new(path);
        let repo_writer = rpmrepo_metadata::RepositoryWriter::new(path, num_pkgs).map_err(into_pyerr)?;
        let py_repo_writer = RepositoryWriter { inner: repo_writer };
        Ok(py_repo_writer)
    }

    fn add_package(&mut self, pkg: &Package) -> PyResult<()> {
        self.inner.add_package(&pkg.inner).map_err(into_pyerr)?;
        Ok(())
    }

    fn finish(&mut self) -> PyResult<()> {
        self.inner.finish().map_err(into_pyerr)?;
        Ok(())
    }
}

#[pyclass]
struct RepositoryReader {
    inner: rpmrepo_metadata::RepositoryReader,
}

#[pymethods]
impl RepositoryReader {
    #[staticmethod]
    fn new_from_directory(path: &str) -> PyResult<Self> {
        let path = Path::new(path);
        let repo_reader =
            rpmrepo_metadata::RepositoryReader::new_from_directory(&path).map_err(into_pyerr)?;
        let py_repo_reader = Self { inner: repo_reader };
        Ok(py_repo_reader)
    }

    fn iter_packages(&self) -> PyResult<PackageParser> {
        let pkg_parser = self.inner.iter_packages().map_err(into_pyerr)?;
        let py_pkg_parser = PackageParser { inner: pkg_parser };
        Ok(py_pkg_parser)
    }
}

// #[pyclass]
// struct RepositoryOptions {
//     inner: rpmrepo_metadata::RepositoryOptions,
// }

// #[pymethods]
// impl RepositoryOptions {
//     #[new]
//     fn new() -> Self {
//         RepositoryOptions {
//             inner: rpmrepo_metadata::RepositoryOptions::default(),
//         }
//     }
// }

// #[pyclass]
// struct Requirement {
//     inner: rpmrepo_metadata::Requirement,
// }

// impl From<()> for Requirement {
//     fn from() -> Self {

//     }
// }

#[pyclass]
struct Package {
    inner: rpmrepo_metadata::Package,
}

#[pymethods]
impl Package {
    #[new]
    fn new() -> Self {
        Package {
            inner: rpmrepo_metadata::Package::default(),
        }
    }

    fn nevra_short(&self) -> PyResult<String> {
        Ok(self.inner.nevra().short())
    }

    fn nevra_canonical(&self) -> PyResult<String> {
        Ok(self.inner.nevra().canonical())
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
        let checksum =
            rpmrepo_metadata::Checksum::try_create(checksum.0, checksum.1).map_err(into_pyerr)?;
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

    #[setter(time)]
    pub fn set_time(&mut self, tuple: (u64, u64)) {
        self.inner.set_time(tuple.0, tuple.1);
    }

    #[getter(time)]
    pub fn time(&self) -> (u64, u64) {
        let time = self.inner.time();
        (time.file, time.build)
    }

    #[setter(time)]
    pub fn set_size(&mut self, tuple: (u64, u64, u64)) {
        self.inner.set_size(tuple.0, tuple.1, tuple.2);
    }

    #[getter(size)]
    pub fn size(&self) -> (u64, u64, u64) {
        let size = self.inner.size();
        (size.package, size.installed, size.archive)
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

    // #[setter(requires)]
    // pub fn set_requires(&mut self, requires: Vec<Requirement>) {
    //     self.inner.set_rpm_requires(requires);
    // }

    // #[getter(requires)]
    // pub fn requires(&self) -> &[Requirement] {
    //     self.inner.rpm_requires()
    // }

    // #[setter(provides)]
    // pub fn set_provides(&mut self, provides: Vec<Requirement>) {
    //     self.inner.set_rpm_provides(provides);
    // }

    // #[getter(provides)]
    // pub fn provides(&self) -> &[Requirement] {
    //     self.inner.rpm_provides()
    // }

    // #[setter(conflicts)]
    // pub fn set_conflicts(&mut self, conflicts: Vec<Requirement>) {
    //     self.inner.set_rpm_conflicts(conflicts);
    // }

    // #[getter(conflicts)]
    // pub fn conflicts(&self) -> &[Requirement] {
    //     self.inner.rpm_conflicts()
    // }

    // #[setter(obsoletes)]
    // pub fn set_obsoletes(&mut self, obsoletes: Vec<Requirement>) {
    //     self.inner.set_rpm_obsoletes(obsoletes);
    // }

    // #[getter(obsoletes)]
    // pub fn obsoletes(&self) -> &[Requirement] {
    //     self.inner.rpm_obsoletes()
    // }

    // #[setter(suggests)]
    // pub fn set_suggests(&mut self, suggests: Vec<Requirement>) {
    //     self.inner.set_rpm_suggests(suggests);
    // }

    // #[getter(suggests)]
    // pub fn suggests(&self) -> &[Requirement] {
    //     self.inner.rpm_suggests()
    // }

    // #[setter(enhances)]
    // pub fn set_enhances(&mut self, enhances: Vec<Requirement>) {
    //     self.inner.set_rpm_enhances(enhances);
    // }

    // #[getter(enhances)]
    // pub fn enhances(&self) -> &[Requirement] {
    //     self.inner.rpm_enhances()
    // }

    // #[setter(recommends)]
    // pub fn set_recommends(&mut self, recommends: Vec<Requirement>) {
    //     self.inner.set_rpm_recommends(recommends);
    // }

    // #[getter(recommends)]
    // pub fn recommends(&self) -> &[Requirement] {
    //     self.inner.rpm_recommends()
    // }

    // #[setter(supplements)]
    // pub fn set_supplements(&mut self, supplements: Vec<Requirement>) {
    //     self.inner.set_rpm_supplements(supplements);
    // }

    // #[getter(supplements)]
    // pub fn supplements(&self) -> &[Requirement] {
    //     self.inner.rpm_supplements()
    // }

    //     pub fn add_file(&mut self, filetype: FileType, path: &str) -> &mut Self {
    //         self.rpm_files.push(PackageFile {
    //             filetype,
    //             path: path.to_owned(),
    //         });
    //         self
    //     }

    //     pub fn files(&self) -> &[PackageFile] {
    //         &self.rpm_files
    //     }

    //     pub fn add_changelog(&mut self, author: &str, description: &str, date: u64) -> &mut Self {
    //         self.rpm_changelogs.push(Changelog {
    //             author: author.to_owned(),
    //             date: date,
    //             description: description.to_owned(),
    //         });
    //         self
    //     }

    //     pub fn changelogs(&self) -> &[Changelog] {
    //         &self.rpm_changelogs
    //     }
}

#[pyclass]
struct PackageParser {
    inner: rpmrepo_metadata::PackageParser,
}

#[pymethods]
impl PackageParser {
    fn parse_package(&mut self) -> PyResult<Option<Package>> {
        let pkg = self.inner.parse_package().map_err(into_pyerr)?;
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
#[pyo3(name = "rpmrepo_metadata")]
fn rpmrepo_metadata_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Repository>()?;
    m.add_class::<RepositoryWriter>()?;
    m.add_class::<RepositoryReader>()?;
    // m.add_class::<RepositoryOptions>()?;
    m.add_class::<Package>()?;
    m.add_class::<PackageParser>()?;

    Ok(())
}
