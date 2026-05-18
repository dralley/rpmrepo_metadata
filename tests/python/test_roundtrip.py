import tempfile

import rpmrepo_metadata as r

from conftest import COMPLEX_REPO


class TestRepositoryRoundtrip:
    def test_load_write_reload_packages(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = r.Repository.load_from_directory(COMPLEX_REPO)
            repo.write_to_directory(tmpdir)

            reader1 = r.RepositoryReader(COMPLEX_REPO)
            reader2 = r.RepositoryReader(tmpdir)
            pkgs1 = list(reader1.iter_packages())
            pkgs2 = list(reader2.iter_packages())
            assert len(pkgs1) == len(pkgs2)
            for p1, p2 in zip(pkgs1, pkgs2):
                assert p1.name == p2.name
                assert p1.version == p2.version
                assert p1.release == p2.release
                assert p1.arch == p2.arch
                assert p1.checksum == p2.checksum

    def test_repository_comps_setters(self):
        repo = r.Repository()
        group = r.CompsGroup(id="test-group", name="Test Group")
        cat = r.CompsCategory(id="test-cat", name="Test Category")
        env = r.CompsEnvironment(id="test-env", name="Test Environment")
        lp = r.CompsLangpack(name="test", install="test-langpack-%s")
        repo.groups = [group]
        repo.categories = [cat]
        repo.environments = [env]
        repo.langpacks = [lp]
        assert len(repo.groups) == 1
        assert repo.groups[0].id == "test-group"
        assert len(repo.categories) == 1
        assert len(repo.environments) == 1
        assert len(repo.langpacks) == 1

    def test_write_repo_with_comps(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            repo = r.Repository()
            group = r.CompsGroup(id="core", name="Core")
            lp = r.CompsLangpack(name="firefox", install="firefox-langpack-%s")
            repo.groups = [group]
            repo.langpacks = [lp]
            repo.write_to_directory(tmpdir)

            repo2 = r.Repository.load_from_directory(tmpdir)
            assert len(repo2.groups) == 1
            assert repo2.groups[0].id == "core"
            assert len(repo2.langpacks) == 1
            assert repo2.langpacks[0].name == "firefox"
