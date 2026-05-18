import os
import tempfile

import pytest

import rpmrepo_metadata as r

from conftest import COMPLEX_REPO


class TestRepository:
    def test_new_empty(self):
        repo = r.Repository()
        assert repo.groups == []
        assert repo.categories == []
        assert repo.environments == []
        assert repo.langpacks == []

    def test_load_from_directory(self):
        repo = r.Repository.load_from_directory(COMPLEX_REPO)
        assert repo is not None

    def test_load_nonexistent_directory(self):
        with pytest.raises(Exception):
            r.Repository.load_from_directory("/nonexistent/path")


class TestRepositoryReader:
    def test_iter_packages(self):
        reader = r.RepositoryReader(COMPLEX_REPO)
        pkgs = list(reader.iter_packages())
        assert len(pkgs) > 0
        pkg = pkgs[0]
        assert pkg.name != ""
        assert pkg.arch != ""

    def test_read_complex_package(self):
        reader = r.RepositoryReader(COMPLEX_REPO)
        pkg = next(reader.iter_packages())
        assert pkg.name == "complex-package"
        assert pkg.epoch == 1
        assert pkg.version == "2.3.4"
        assert pkg.release == "5.el8"
        assert pkg.arch == "x86_64"
        assert pkg.checksum[0] == "sha256"
        assert len(pkg.checksum[1]) > 0
        assert pkg.location_href == "complex-package-2.3.4-5.el8.x86_64.rpm"
        assert len(pkg.files) == 6
        assert len(pkg.requires) == 5
        assert len(pkg.provides) == 5
        assert len(pkg.changelogs) == 3
        assert pkg.rpm_license == "MPLv2"


class TestRepositoryWriter:
    def test_write_empty(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = r.RepositoryWriter(tmpdir, 0)
            writer.finish()
            assert os.path.exists(os.path.join(tmpdir, "repodata", "repomd.xml"))

    def test_write_package(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg = r.Package()
            pkg.name = "test"
            pkg.version = "1.0"
            pkg.release = "1"
            pkg.arch = "noarch"
            pkg.checksum = ("sha256", "a" * 64)
            pkg.location_href = "test-1.0-1.noarch.rpm"

            writer = r.RepositoryWriter(tmpdir, 1)
            writer.add_package(pkg)
            writer.finish()
            assert os.path.exists(os.path.join(tmpdir, "repodata", "repomd.xml"))

    def test_finish_twice_panics(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = r.RepositoryWriter(tmpdir, 0)
            writer.finish()
            with pytest.raises(BaseException):
                writer.finish()


class TestPackageIterator:
    def test_total_and_remaining(self):
        reader = r.RepositoryReader(COMPLEX_REPO)
        it = reader.iter_packages()
        total = it.total_packages
        assert total > 0
        assert it.remaining_packages == total

    def test_iteration_protocol(self):
        reader = r.RepositoryReader(COMPLEX_REPO)
        it = reader.iter_packages()
        pkgs = list(it)
        assert len(pkgs) > 0

    def test_length_hint(self):
        reader = r.RepositoryReader(COMPLEX_REPO)
        it = reader.iter_packages()
        assert it.__length_hint__() > 0
