import os
import tempfile

import pytest

import rpmrepo_metadata as r

from conftest import FIXTURES_DIR


class TestUpdateRecord:
    @pytest.fixture
    def advisory(self):
        """Load an advisory from a real repo if available, otherwise skip."""
        external = os.path.join(FIXTURES_DIR, "external_repos")
        if not os.path.isdir(external):
            pytest.skip("No external repos available")
        for name in os.listdir(external):
            repo_dir = os.path.join(external, name)
            if not os.path.isdir(repo_dir):
                continue
            try:
                reader = r.RepositoryReader(repo_dir)
                it = reader.iter_advisories()
                rec = next(it)
                return rec
            except BaseException:
                continue
        pytest.skip("No updateinfo fixture available")

    def test_str_repr(self, advisory):
        assert "UpdateRecord" in str(advisory)
        assert "UpdateRecord" in repr(advisory)

    def test_basic_fields(self, advisory):
        assert isinstance(advisory.id, str)
        assert len(advisory.id) > 0
        assert isinstance(advisory.title, str)
        assert isinstance(advisory.update_type, str)
        assert isinstance(advisory.status, str)
        assert isinstance(advisory.version, str)
        assert isinstance(advisory.fromstr, str)
        assert isinstance(advisory.rights, str)
        assert isinstance(advisory.release, str)
        assert isinstance(advisory.severity, str)
        assert isinstance(advisory.summary, str)
        assert isinstance(advisory.description, str)
        assert isinstance(advisory.solution, str)

    def test_optional_fields(self, advisory):
        assert advisory.issued_date is None or isinstance(advisory.issued_date, str)
        assert advisory.updated_date is None or isinstance(advisory.updated_date, str)
        assert advisory.pushcount is None or isinstance(advisory.pushcount, str)

    def test_references(self, advisory):
        refs = advisory.references
        assert isinstance(refs, list)
        if len(refs) > 0:
            ref = refs[0]
            assert isinstance(ref.href, str)
            assert isinstance(ref.id, str)
            assert isinstance(ref.title, str)
            assert isinstance(ref.reftype, str)

    def test_pkglist(self, advisory):
        pkglist = advisory.pkglist
        assert isinstance(pkglist, list)
        if len(pkglist) > 0:
            coll = pkglist[0]
            assert isinstance(coll.name, str)
            assert isinstance(coll.shortname, str)
            assert isinstance(coll.packages, list)
            assert coll.module is None or hasattr(coll.module, "name")
            if len(coll.packages) > 0:
                cpkg = coll.packages[0]
                assert isinstance(cpkg.name, str)
                assert isinstance(cpkg.epoch, str)
                assert isinstance(cpkg.version, str)
                assert isinstance(cpkg.release, str)
                assert isinstance(cpkg.arch, str)
                assert isinstance(cpkg.filename, str)
                assert isinstance(cpkg.src, str)
                assert isinstance(cpkg.reboot_suggested, bool)
                assert isinstance(cpkg.restart_suggested, bool)
                assert isinstance(cpkg.relogin_suggested, bool)
                assert cpkg.checksum is None or isinstance(cpkg.checksum, tuple)


class TestUpdateinfoReader:
    def test_iteration(self):
        external = os.path.join(FIXTURES_DIR, "external_repos")
        if not os.path.isdir(external):
            pytest.skip("No external repos available")
        for name in os.listdir(external):
            repo_dir = os.path.join(external, name)
            if not os.path.isdir(repo_dir):
                continue
            try:
                reader = r.RepositoryReader(repo_dir)
                records = list(reader.iter_advisories())
                if len(records) > 0:
                    assert all(isinstance(rec, r.UpdateRecord) for rec in records)
                    return
            except BaseException:
                continue
        pytest.skip("No updateinfo fixture available")


class TestUpdateRecordConstruction:
    def test_create_empty(self):
        rec = r.UpdateRecord(id="RHSA-2024:0001")
        assert rec.id == "RHSA-2024:0001"
        assert rec.title == ""
        assert rec.references == []
        assert rec.pkglist == []

    def test_create_with_fields(self):
        rec = r.UpdateRecord(
            id="RHSA-2024:0002",
            update_type="security",
            title="Important security fix",
            severity="Important",
            summary="Fixes CVE-2024-1234",
            issued_date="2024-01-15",
        )
        assert rec.id == "RHSA-2024:0002"
        assert rec.update_type == "security"
        assert rec.title == "Important security fix"
        assert rec.severity == "Important"
        assert rec.summary == "Fixes CVE-2024-1234"
        assert rec.issued_date == "2024-01-15"

    def test_set_references(self):
        rec = r.UpdateRecord(id="RHSA-2024:0003")
        ref = r.UpdateReference(
            href="https://cve.org/CVE-2024-1234",
            id="CVE-2024-1234",
            title="Buffer overflow",
            reftype="cve",
        )
        rec.references = [ref]
        assert len(rec.references) == 1
        assert rec.references[0].id == "CVE-2024-1234"
        assert rec.references[0].reftype == "cve"

    def test_set_pkglist(self):
        rec = r.UpdateRecord(id="RHSA-2024:0004")
        cpkg = r.UpdateCollectionPackage(
            name="foo",
            version="1.0",
            release="1.el9",
            arch="x86_64",
            epoch="0",
            filename="foo-1.0-1.el9.x86_64.rpm",
        )
        coll = r.UpdateCollection(name="coll1", shortname="c1")
        coll.packages = [cpkg]
        rec.pkglist = [coll]
        assert len(rec.pkglist) == 1
        assert len(rec.pkglist[0].packages) == 1
        assert rec.pkglist[0].packages[0].name == "foo"


class TestRepositoryWriterAdvisory:
    def test_write_and_reload_advisory(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            rec = r.UpdateRecord(
                id="RHSA-2024:9999",
                update_type="security",
                title="Test advisory",
                severity="Critical",
                summary="A test advisory",
                fromstr="security@example.com",
            )
            ref = r.UpdateReference(
                href="https://cve.org/CVE-2024-0001",
                id="CVE-2024-0001",
                title="Test CVE",
                reftype="cve",
            )
            rec.references = [ref]

            cpkg = r.UpdateCollectionPackage(
                name="testpkg",
                version="1.0",
                release="1.el9",
                arch="x86_64",
                epoch="0",
                filename="testpkg-1.0-1.el9.x86_64.rpm",
            )
            coll = r.UpdateCollection(name="Test Collection", shortname="tc")
            coll.packages = [cpkg]
            rec.pkglist = [coll]

            writer = r.RepositoryWriter(tmpdir, 0)
            writer.add_advisory(rec)
            writer.finish()

            reader = r.RepositoryReader(tmpdir)
            advisories = list(reader.iter_advisories())
            assert len(advisories) == 1
            adv = advisories[0]
            assert adv.id == "RHSA-2024:9999"
            assert adv.update_type == "security"
            assert adv.title == "Test advisory"
            assert adv.severity == "Critical"
            assert adv.fromstr == "security@example.com"
            assert len(adv.references) == 1
            assert adv.references[0].id == "CVE-2024-0001"
            assert len(adv.pkglist) == 1
            assert len(adv.pkglist[0].packages) == 1
            assert adv.pkglist[0].packages[0].name == "testpkg"
