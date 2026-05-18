import os
import tempfile

import pytest

import rpmrepo_metadata as r

from conftest import FIXTURES_DIR


class TestCompsTypes:
    @pytest.fixture
    def comps_data(self):
        """Parse the comps fixture using the Rust reader via Repository.load_from_directory
        is not possible since the fixture is standalone XML. Test the types via RepositoryReader
        if a repo with comps exists, otherwise skip."""
        pytest.skip("No repo fixture with comps data; comps types tested via Rust tests")

    def test_comps_group_type_exists(self):
        assert hasattr(r, "CompsGroup")

    def test_comps_category_type_exists(self):
        assert hasattr(r, "CompsCategory")

    def test_comps_environment_type_exists(self):
        assert hasattr(r, "CompsEnvironment")

    def test_comps_environment_option_type_exists(self):
        assert hasattr(r, "CompsEnvironmentOption")

    def test_comps_package_req_type_exists(self):
        assert hasattr(r, "CompsPackageReq")

    def test_comps_langpack_type_exists(self):
        assert hasattr(r, "CompsLangpack")

    def test_repository_comps_accessors(self):
        repo = r.Repository()
        assert repo.groups == []
        assert repo.categories == []
        assert repo.environments == []
        assert repo.langpacks == []


class TestCompsConstruction:
    def test_comps_group(self):
        pkg = r.CompsPackageReq(name="bash", reqtype="mandatory")
        group = r.CompsGroup(id="core", name="Core")
        group.packages = [pkg]
        assert group.id == "core"
        assert group.name == "Core"
        assert len(group.packages) == 1
        assert group.packages[0].name == "bash"

    def test_comps_category(self):
        cat = r.CompsCategory(id="base-system", name="Base System")
        cat.group_ids = ["core", "base"]
        assert cat.id == "base-system"
        assert cat.group_ids == ["core", "base"]

    def test_comps_environment(self):
        opt = r.CompsEnvironmentOption(group_id="debugging", default=False)
        env = r.CompsEnvironment(id="minimal", name="Minimal Install")
        env.group_ids = ["core"]
        env.option_ids = [opt]
        assert env.id == "minimal"
        assert env.group_ids == ["core"]
        assert len(env.option_ids) == 1
        assert env.option_ids[0].group_id == "debugging"

    def test_comps_langpack(self):
        lp = r.CompsLangpack(name="firefox", install="firefox-langpack-%s")
        assert lp.name == "firefox"
        assert lp.install == "firefox-langpack-%s"

    def test_name_by_lang(self):
        group = r.CompsGroup(id="test", name="Test")
        group.name_by_lang = [("fr", "Tester"), ("de", "Testen")]
        assert len(group.name_by_lang) == 2
        assert group.name_by_lang[0] == ("fr", "Tester")


class TestRepositoryWriterComps:
    def test_write_and_reload_comps(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg1 = r.CompsPackageReq(name="bash", reqtype="mandatory")
            pkg2 = r.CompsPackageReq(name="vim", reqtype="default")
            group = r.CompsGroup(id="core", name="Core")
            group.packages = [pkg1, pkg2]
            group.desc_by_lang = [("en", "Core packages")]

            cat = r.CompsCategory(id="base", name="Base System")
            cat.group_ids = ["core"]

            opt = r.CompsEnvironmentOption(group_id="debugging", default=True)
            env = r.CompsEnvironment(id="minimal", name="Minimal Install")
            env.group_ids = ["core"]
            env.option_ids = [opt]

            lp = r.CompsLangpack(name="firefox", install="firefox-langpack-%s")

            writer = r.RepositoryWriter(tmpdir, 0)
            writer.write_comps([group], [cat], [env], [lp])
            writer.finish()

            repo = r.Repository.load_from_directory(tmpdir)
            assert len(repo.groups) == 1
            assert repo.groups[0].id == "core"
            assert len(repo.groups[0].packages) == 2
            assert len(repo.categories) == 1
            assert repo.categories[0].id == "base"
            assert len(repo.environments) == 1
            assert repo.environments[0].id == "minimal"
            assert len(repo.environments[0].option_ids) == 1
            assert len(repo.langpacks) == 1
            assert repo.langpacks[0].name == "firefox"

    def test_write_comps_incrementally(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            group1 = r.CompsGroup(id="core", name="Core")
            group2 = r.CompsGroup(id="base", name="Base")
            cat = r.CompsCategory(id="base-system", name="Base System")
            env = r.CompsEnvironment(id="minimal", name="Minimal Install")
            lp = r.CompsLangpack(name="firefox", install="firefox-langpack-%s")

            writer = r.RepositoryWriter(tmpdir, 0)
            writer.add_group(group1)
            writer.add_group(group2)
            writer.add_category(cat)
            writer.add_environment(env)
            writer.set_langpacks([lp])
            writer.finish()

            repo = r.Repository.load_from_directory(tmpdir)
            assert len(repo.groups) == 2
            assert repo.groups[0].id == "core"
            assert repo.groups[1].id == "base"
            assert len(repo.categories) == 1
            assert len(repo.environments) == 1
            assert len(repo.langpacks) == 1
