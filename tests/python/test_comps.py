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

    def test_all_constructors_args_optional(self):
        """Every comps type can be constructed with no arguments."""
        assert r.CompsGroup().id == ""
        assert r.CompsPackageReq().name == ""
        assert r.CompsCategory().id == ""
        assert r.CompsEnvironment().id == ""
        assert r.CompsEnvironmentOption().group_id == ""
        assert r.CompsLangpack().name == ""
        assert r.CompsLangpack().install == ""

    def test_name_by_lang(self):
        group = r.CompsGroup(id="test", name="Test")
        group.name_by_lang = {"fr": "Tester", "de": "Testen"}
        assert len(group.name_by_lang) == 2
        assert group.name_by_lang["fr"] == "Tester"
        assert group.name_by_lang["de"] == "Testen"

    def test_langonly(self):
        group = r.CompsGroup(id="test", name="Test", langonly="sr")
        assert group.langonly == "sr"

    def test_group_to_dict(self):
        group = r.CompsGroup(id="core", name="Core", description="Core packages")
        pkg = r.CompsPackageReq(name="bash", reqtype="mandatory")
        group.packages = [pkg]
        group.name_by_lang = {"fr": "Noyau"}
        d = group.to_dict()
        assert d["id"] == "core"
        assert d["name"] == "Core"
        assert d["description"] == "Core packages"
        assert d["name_by_lang"] == {"fr": "Noyau"}
        assert len(d["packages"]) == 1
        assert d["packages"][0]["name"] == "bash"

    def test_group_to_json(self):
        group = r.CompsGroup(id="core", name="Core")
        json_str = group.to_json()
        assert '"id":"core"' in json_str.replace(" ", "")

    def test_category_to_dict(self):
        cat = r.CompsCategory(id="base-system", name="Base System")
        cat.group_ids = ["core", "base"]
        cat.name_by_lang = {"de": "Basissystem"}
        d = cat.to_dict()
        assert d["id"] == "base-system"
        assert d["group_ids"] == ["core", "base"]
        assert d["name_by_lang"] == {"de": "Basissystem"}

    def test_category_to_json(self):
        cat = r.CompsCategory(id="base-system", name="Base System")
        json_str = cat.to_json()
        assert '"id":"base-system"' in json_str.replace(" ", "")

    def test_environment_to_dict(self):
        opt = r.CompsEnvironmentOption(group_id="debugging", default=False)
        env = r.CompsEnvironment(id="minimal", name="Minimal Install")
        env.group_ids = ["core"]
        env.option_ids = [opt]
        d = env.to_dict()
        assert d["id"] == "minimal"
        assert d["group_ids"] == ["core"]
        assert len(d["option_ids"]) == 1
        assert d["option_ids"][0]["group_id"] == "debugging"

    def test_environment_to_json(self):
        env = r.CompsEnvironment(id="minimal", name="Minimal Install")
        json_str = env.to_json()
        assert '"id":"minimal"' in json_str.replace(" ", "")


class TestCompsSetters:
    """Every readable comps attribute is also writable after construction."""

    def test_group_setters(self):
        group = r.CompsGroup()
        group.id = "core"
        group.name = "Core"
        group.description = "Core packages"
        group.default = True
        group.uservisible = False
        group.biarchonly = True
        group.langonly = "sr"
        group.display_order = 5
        group.packages = [r.CompsPackageReq(name="bash", reqtype="mandatory")]
        group.name_by_lang = {"fr": "Noyau"}
        group.desc_by_lang = {"fr": "Paquets de base"}

        assert group.id == "core"
        assert group.name == "Core"
        assert group.description == "Core packages"
        assert group.default is True
        assert group.uservisible is False
        assert group.biarchonly is True
        assert group.langonly == "sr"
        assert group.display_order == 5
        assert [p.name for p in group.packages] == ["bash"]
        assert group.name_by_lang == {"fr": "Noyau"}
        assert group.desc_by_lang == {"fr": "Paquets de base"}

        # Nullable fields can be cleared back to None.
        group.langonly = None
        group.display_order = None
        assert group.langonly is None
        assert group.display_order is None

    def test_package_req_setters(self):
        pkg = r.CompsPackageReq()
        pkg.name = "ibus-gtk3"
        pkg.reqtype = "conditional"
        pkg.requires = "gtk3"
        pkg.basearchonly = True

        assert pkg.name == "ibus-gtk3"
        assert pkg.reqtype == "conditional"
        assert pkg.requires == "gtk3"
        assert pkg.basearchonly is True

        pkg.requires = None
        pkg.basearchonly = None
        assert pkg.requires is None
        assert pkg.basearchonly is None

    def test_category_setters(self):
        cat = r.CompsCategory()
        cat.id = "servers"
        cat.name = "Servers"
        cat.description = "Server software"
        cat.display_order = 30
        cat.group_ids = ["core", "network-server"]
        cat.name_by_lang = {"ja": "サーバー"}
        cat.desc_by_lang = {"ja": "サーバーソフトウェア"}

        assert cat.id == "servers"
        assert cat.name == "Servers"
        assert cat.description == "Server software"
        assert cat.display_order == 30
        assert cat.group_ids == ["core", "network-server"]
        assert cat.name_by_lang == {"ja": "サーバー"}
        assert cat.desc_by_lang == {"ja": "サーバーソフトウェア"}

    def test_environment_setters(self):
        env = r.CompsEnvironment()
        env.id = "server"
        env.name = "Server"
        env.description = "An easy-to-manage server"
        env.display_order = 2
        env.group_ids = ["core", "server-product"]
        env.option_ids = [r.CompsEnvironmentOption(group_id="debugging", default=False)]
        env.name_by_lang = {"ja": "サーバー"}
        env.desc_by_lang = {"ja": "サーバー"}

        assert env.id == "server"
        assert env.name == "Server"
        assert env.description == "An easy-to-manage server"
        assert env.display_order == 2
        assert env.group_ids == ["core", "server-product"]
        assert [o.group_id for o in env.option_ids] == ["debugging"]
        assert env.name_by_lang == {"ja": "サーバー"}
        assert env.desc_by_lang == {"ja": "サーバー"}

    def test_environment_option_setters(self):
        opt = r.CompsEnvironmentOption()
        opt.group_id = "headless-management"
        opt.default = True
        assert opt.group_id == "headless-management"
        assert opt.default is True

    def test_langpack_setters(self):
        lp = r.CompsLangpack()
        lp.name = "hunspell"
        lp.install = "hunspell-%{lang}"
        assert lp.name == "hunspell"
        assert lp.install == "hunspell-%{lang}"


class TestRepositoryWriterComps:
    def test_write_and_reload_comps(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            pkg1 = r.CompsPackageReq(name="bash", reqtype="mandatory")
            pkg2 = r.CompsPackageReq(name="vim", reqtype="default")
            group = r.CompsGroup(id="core", name="Core")
            group.packages = [pkg1, pkg2]
            group.desc_by_lang = {"en": "Core packages"}

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
