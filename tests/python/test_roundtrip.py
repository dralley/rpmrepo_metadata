import os
import tempfile

import pytest

import rpmrepo_metadata as r

from compare import (
    compare_package_lists,
    compare_advisory_lists,
    compare_comps,
    compare_packages,
    compare_update_records,
    compare_comps_groups,
    compare_comps_categories,
    compare_comps_environments,
    compare_comps_langpacks,
)
from conftest import COMPLEX_REPO, COMPS_FIXTURE, FIXTURES_DIR

EXTERNAL_REPOS = os.path.join(FIXTURES_DIR, "external_repos")


def _repos_with_updateinfo():
    if not os.path.isdir(EXTERNAL_REPOS):
        return []
    repos = []
    for dirpath, dirnames, _filenames in os.walk(EXTERNAL_REPOS):
        dirnames[:] = [d for d in dirnames if not d.startswith(".") and d != "repodata"]
        if "repodata" in os.listdir(dirpath):
            repodata = os.path.join(dirpath, "repodata")
            if any("updateinfo" in f for f in os.listdir(repodata)):
                repos.append(dirpath)
    return sorted(repos)


def _repos_with_comps():
    if not os.path.isdir(EXTERNAL_REPOS):
        return []
    repos = []
    for dirpath, dirnames, _filenames in os.walk(EXTERNAL_REPOS):
        dirnames[:] = [d for d in dirnames if not d.startswith(".") and d != "repodata"]
        if "repodata" in os.listdir(dirpath):
            repodata = os.path.join(dirpath, "repodata")
            if any("comps" in f for f in os.listdir(repodata)):
                repos.append(dirpath)
    return sorted(repos)


# ---------------------------------------------------------------------------
# Package roundtrip via Repository.load_from_directory / write_to_directory
# ---------------------------------------------------------------------------

class TestPackageRoundtrip:
    def test_complex_repo_all_fields(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = r.Repository.load_from_directory(COMPLEX_REPO)
            repo.write_to_directory(tmpdir)

            reader1 = r.RepositoryReader(COMPLEX_REPO)
            reader2 = r.RepositoryReader(tmpdir)
            compare_package_lists(
                list(reader1.iter_packages()),
                list(reader2.iter_packages()),
            )

    def test_streaming_writer_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            reader = r.RepositoryReader(COMPLEX_REPO)
            pkgs_orig = list(reader.iter_packages())

            writer = r.RepositoryWriter(tmpdir, len(pkgs_orig))
            for pkg in pkgs_orig:
                writer.add_package(pkg)
            writer.finish()

            reader2 = r.RepositoryReader(tmpdir)
            compare_package_lists(pkgs_orig, list(reader2.iter_packages()))

    def test_empty_repo_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            writer = r.RepositoryWriter(tmpdir, 0)
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            assert list(reader.iter_packages()) == []

    def test_constructed_package_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg = r.Package()
            pkg.name = "roundtrip-test"
            pkg.epoch = 2
            pkg.version = "3.1.4"
            pkg.release = "1.el9"
            pkg.arch = "x86_64"
            pkg.checksum = ("sha256", "ab" * 32)
            pkg.summary = "Test summary"
            pkg.description = "Test description\nwith newlines"
            pkg.packager = "Test Packager <test@example.com>"
            pkg.url = "https://example.com"
            pkg.location_href = "roundtrip-test-3.1.4-1.el9.x86_64.rpm"
            pkg.location_base = "https://mirror.example.com"
            pkg.time_file = 1700000000
            pkg.time_build = 1699999999
            pkg.size_package = 12345
            pkg.size_installed = 67890
            pkg.size_archive = 11111
            pkg.rpm_license = "MIT"
            pkg.rpm_vendor = "Example Corp"
            pkg.rpm_group = "Development/Libraries"
            pkg.rpm_buildhost = "builder.example.com"
            pkg.rpm_sourcerpm = "roundtrip-test-3.1.4-1.el9.src.rpm"
            pkg.rpm_header_range = (280, 5000)
            pkg.files = [
                (None, "/usr/bin/roundtrip"),
                ("dir", "/usr/share/roundtrip"),
                ("ghost", "/var/log/roundtrip.log"),
            ]
            pkg.changelogs = [
                ("Author One <one@example.com>", 1699000000, "- First change"),
                ("Author Two <two@example.com>", 1699500000, "- Second change"),
            ]
            pkg.requires = [
                ("libc.so.6()(64bit)", "EQ", "0", "2.17", "", False),
                ("/usr/bin/bash", None, None, None, None, True),
            ]
            pkg.provides = [
                ("roundtrip-test", "EQ", "2", "3.1.4", "1.el9", False),
                ("roundtrip-test(x86-64)", "EQ", "2", "3.1.4", "1.el9", False),
            ]
            pkg.conflicts = [("old-roundtrip", "LT", "0", "2.0", "", False)]
            pkg.obsoletes = [("legacy-roundtrip", "LT", "0", "1.0", "", False)]
            pkg.suggests = [("optional-dep", None, None, None, None, False)]
            pkg.enhances = [("enhancement", None, None, None, None, False)]
            pkg.recommends = [("recommended-dep", "GE", "0", "1.0", "", False)]
            pkg.supplements = [("supplementary", None, None, None, None, False)]

            writer = r.RepositoryWriter(tmpdir, 1)
            writer.add_package(pkg)
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            pkgs = list(reader.iter_packages())
            assert len(pkgs) == 1
            compare_packages(pkg, pkgs[0])


# ---------------------------------------------------------------------------
# Advisory / UpdateInfo roundtrip
# ---------------------------------------------------------------------------

class TestAdvisoryRoundtrip:
    def test_constructed_advisory_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            rec = r.UpdateRecord(
                id="RHSA-2024:9999",
                update_type="security",
                title="Critical security fix for widget",
                severity="Critical",
                summary="Fixes CVE-2024-0001 and CVE-2024-0002",
                description="A critical vulnerability was found in widget.",
                solution="Update to the latest version.",
                fromstr="security@example.com",
                status="final",
                version="2",
                rights="Copyright 2024 Example Corp",
                release="Example Release",
                issued_date="2024-06-15",
                updated_date="2024-06-16",
                pushcount="1",
            )
            ref1 = r.UpdateReference(
                href="https://cve.org/CVE-2024-0001",
                id="CVE-2024-0001",
                title="Buffer overflow in widget",
                reftype="cve",
            )
            ref2 = r.UpdateReference(
                href="https://bugzilla.example.com/12345",
                id="12345",
                title="Widget crashes on large input",
                reftype="bugzilla",
            )
            rec.references = [ref1, ref2]

            cpkg1 = r.UpdateCollectionPackage(
                name="widget",
                version="2.0",
                release="1.el9",
                arch="x86_64",
                epoch="0",
                filename="widget-2.0-1.el9.x86_64.rpm",
                src="widget-2.0-1.el9.src.rpm",
            )
            cpkg2 = r.UpdateCollectionPackage(
                name="widget-libs",
                version="2.0",
                release="1.el9",
                arch="x86_64",
                epoch="0",
                filename="widget-libs-2.0-1.el9.x86_64.rpm",
            )
            coll = r.UpdateCollection(name="Example Collection", shortname="example")
            coll.packages = [cpkg1, cpkg2]
            rec.pkglist = [coll]

            writer = r.RepositoryWriter(tmpdir, 0)
            writer.add_advisory(rec)
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            advisories = list(reader.iter_advisories())
            assert len(advisories) == 1
            compare_update_records(rec, advisories[0])

    def test_multiple_advisories_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            recs = []
            for i in range(3):
                rec = r.UpdateRecord(
                    id=f"RHSA-2024:{i:04d}",
                    update_type="security" if i % 2 == 0 else "bugfix",
                    title=f"Advisory {i}",
                    severity="Important",
                    summary=f"Summary for advisory {i}",
                )
                ref = r.UpdateReference(
                    href=f"https://cve.org/CVE-2024-{i:04d}",
                    id=f"CVE-2024-{i:04d}",
                    title=f"Issue {i}",
                    reftype="cve",
                )
                rec.references = [ref]
                cpkg = r.UpdateCollectionPackage(
                    name=f"pkg{i}",
                    version="1.0",
                    release="1.el9",
                    arch="x86_64",
                    epoch="0",
                    filename=f"pkg{i}-1.0-1.el9.x86_64.rpm",
                )
                coll = r.UpdateCollection(name=f"Collection {i}", shortname=f"c{i}")
                coll.packages = [cpkg]
                rec.pkglist = [coll]
                recs.append(rec)

            writer = r.RepositoryWriter(tmpdir, 0)
            for rec in recs:
                writer.add_advisory(rec)
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            compare_advisory_lists(recs, list(reader.iter_advisories()))

    @pytest.mark.parametrize("repo_path", _repos_with_updateinfo(),
                             ids=lambda p: os.path.relpath(p, EXTERNAL_REPOS))
    def test_external_repo_advisory_roundtrip(self, repo_path):
        reader1 = r.RepositoryReader(repo_path)
        advisories_orig = list(reader1.iter_advisories())
        if not advisories_orig:
            pytest.skip("No advisories in repo")

        with tempfile.TemporaryDirectory() as tmpdir:
            writer = r.RepositoryWriter(tmpdir, 0)
            for adv in advisories_orig:
                writer.add_advisory(adv)
            writer.finish()

            reader2 = r.RepositoryReader(tmpdir)
            compare_advisory_lists(advisories_orig, list(reader2.iter_advisories()))


# ---------------------------------------------------------------------------
# Comps roundtrip
# ---------------------------------------------------------------------------

class TestCompsRoundtrip:
    def test_comps_xml_roundtrip(self):
        with open(COMPS_FIXTURE) as f:
            xml_str = f.read()
        comps1 = r.CompsData.from_xml(xml_str)

        xml_out = comps1.to_xml()
        comps2 = r.CompsData.from_xml(xml_out)
        compare_comps(comps1, comps2)

    def test_repository_comps_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg1 = r.CompsPackageReq(name="bash", reqtype="mandatory")
            pkg2 = r.CompsPackageReq(name="vim", reqtype="default")
            pkg3 = r.CompsPackageReq(name="emacs", reqtype="optional")

            group = r.CompsGroup(
                id="core",
                name="Core",
                description="Core system packages",
                default=True,
                uservisible=True,
                display_order=10,
            )
            group.name_by_lang = [("fr", "Noyau"), ("de", "Kern")]
            group.desc_by_lang = [("fr", "Paquets systeme de base")]
            group.packages = [pkg1, pkg2, pkg3]

            cat = r.CompsCategory(id="base-system", name="Base System")
            cat.name_by_lang = [("fr", "Systeme de base")]
            cat.group_ids = ["core"]

            opt = r.CompsEnvironmentOption(group_id="debugging", default=True)
            env = r.CompsEnvironment(id="minimal", name="Minimal Install")
            env.name_by_lang = [("fr", "Installation minimale")]
            env.group_ids = ["core"]
            env.option_ids = [opt]

            lp1 = r.CompsLangpack(name="firefox", install="firefox-langpack-%s")
            lp2 = r.CompsLangpack(name="libreoffice-core", install="libreoffice-langpack-%s")

            repo = r.Repository()
            repo.groups = [group]
            repo.categories = [cat]
            repo.environments = [env]
            repo.langpacks = [lp1, lp2]
            repo.write_to_directory(tmpdir)

            repo2 = r.Repository.load_from_directory(tmpdir)
            compare_comps(repo, repo2)

    def test_streaming_writer_comps_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            group1 = r.CompsGroup(id="core", name="Core")
            group1.packages = [r.CompsPackageReq(name="bash", reqtype="mandatory")]
            group2 = r.CompsGroup(id="devel", name="Development")
            group2.packages = [r.CompsPackageReq(name="gcc", reqtype="mandatory")]

            cat = r.CompsCategory(id="base", name="Base")
            cat.group_ids = ["core", "devel"]

            env = r.CompsEnvironment(id="workstation", name="Workstation")
            env.group_ids = ["core"]
            env.option_ids = [r.CompsEnvironmentOption(group_id="devel", default=False)]

            lp = r.CompsLangpack(name="vim", install="vim-lang-%s")

            writer = r.RepositoryWriter(tmpdir, 0)
            writer.add_group(group1)
            writer.add_group(group2)
            writer.add_category(cat)
            writer.add_environment(env)
            writer.set_langpacks([lp])
            writer.finish()

            repo = r.Repository.load_from_directory(tmpdir)
            assert len(repo.groups) == 2
            compare_comps_groups(group1, repo.groups[0])
            compare_comps_groups(group2, repo.groups[1])
            assert len(repo.categories) == 1
            compare_comps_categories(cat, repo.categories[0])
            assert len(repo.environments) == 1
            compare_comps_environments(env, repo.environments[0])
            assert len(repo.langpacks) == 1
            compare_comps_langpacks(lp, repo.langpacks[0])

    @pytest.mark.parametrize("repo_path", _repos_with_comps(),
                             ids=lambda p: os.path.relpath(p, EXTERNAL_REPOS))
    def test_external_repo_comps_roundtrip(self, repo_path):
        try:
            reader = r.RepositoryReader(repo_path)
            comps1 = reader.read_comps()
        except r.MetadataError:
            pytest.skip("Cannot parse repo metadata")
        if comps1 is None:
            pytest.skip("No comps data in repo")

        comps2 = r.CompsData.from_xml(comps1.to_xml())
        compare_comps(comps1, comps2)


# ---------------------------------------------------------------------------
# Full repository roundtrip (packages + comps + advisories together)
# ---------------------------------------------------------------------------

class TestFullRepositoryRoundtrip:
    def test_complex_repo_full(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo1 = r.Repository.load_from_directory(COMPLEX_REPO)
            repo1.write_to_directory(tmpdir)
            repo2 = r.Repository.load_from_directory(tmpdir)

            reader1 = r.RepositoryReader(COMPLEX_REPO)
            reader2 = r.RepositoryReader(tmpdir)
            compare_package_lists(
                list(reader1.iter_packages()),
                list(reader2.iter_packages()),
            )
            compare_comps(repo1, repo2)

    def test_constructed_full_repo_roundtrip(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg = r.Package()
            pkg.name = "full-test"
            pkg.epoch = 0
            pkg.version = "1.0"
            pkg.release = "1"
            pkg.arch = "noarch"
            pkg.checksum = ("sha256", "cc" * 32)
            pkg.location_href = "full-test-1.0-1.noarch.rpm"
            pkg.summary = "Full roundtrip test"
            pkg.rpm_license = "GPL-2.0"

            group = r.CompsGroup(id="test-group", name="Test Group")
            group.packages = [r.CompsPackageReq(name="full-test", reqtype="mandatory")]

            cat = r.CompsCategory(id="test-cat", name="Test Category")
            cat.group_ids = ["test-group"]

            env = r.CompsEnvironment(id="test-env", name="Test Environment")
            env.group_ids = ["test-group"]

            lp = r.CompsLangpack(name="full-test", install="full-test-lang-%s")

            advisory = r.UpdateRecord(
                id="TEST-2024:0001",
                update_type="bugfix",
                title="Fix for full-test",
                summary="Bug fix",
            )
            cpkg = r.UpdateCollectionPackage(
                name="full-test",
                version="1.0",
                release="1",
                arch="noarch",
                epoch="0",
                filename="full-test-1.0-1.noarch.rpm",
            )
            coll = r.UpdateCollection(name="Test", shortname="t")
            coll.packages = [cpkg]
            advisory.pkglist = [coll]

            writer = r.RepositoryWriter(tmpdir, 1)
            writer.add_package(pkg)
            writer.add_advisory(advisory)
            writer.add_group(group)
            writer.add_category(cat)
            writer.add_environment(env)
            writer.set_langpacks([lp])
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            pkgs = list(reader.iter_packages())
            assert len(pkgs) == 1
            compare_packages(pkg, pkgs[0])

            advisories = list(reader.iter_advisories())
            assert len(advisories) == 1
            compare_update_records(advisory, advisories[0])

            repo = r.Repository.load_from_directory(tmpdir)
            assert len(repo.groups) == 1
            compare_comps_groups(group, repo.groups[0])
            assert len(repo.categories) == 1
            compare_comps_categories(cat, repo.categories[0])
            assert len(repo.environments) == 1
            compare_comps_environments(env, repo.environments[0])
            assert len(repo.langpacks) == 1
            compare_comps_langpacks(lp, repo.langpacks[0])
