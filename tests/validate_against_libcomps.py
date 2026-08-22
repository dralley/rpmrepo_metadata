#!/usr/bin/env python3

# A utility for testing the output of the rpmrepo_metadata library against libcomps
# Copyright (C) 2022 Daniel Alley

# The following GPL-2.0 license notice applies to this file (only)
# by virtue of using libcomps, a GPL-2.0 licensed library.
# =============================================================

# This program is free software; you can redistribute it and/or
# modify it under the terms of the GNU General Public License
# version 2 as published by the Free Software Foundation.

# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

# You should have received a copy of the GNU General Public License
# along with this program; If not, see <http://www.gnu.org/licenses/>.

import os
import os.path
import sys

import pytest

import libcomps
import rpmrepo_metadata as rpmmd


# API DIFFERENCES vs. libcomps
#
# * libcomps uses `desc` for descriptions, rpmrepo_metadata uses `description`
# * libcomps uses `lang_only` for groups, rpmrepo_metadata uses `langonly`
# * libcomps Package `type` is an integer constant, rpmrepo_metadata uses a string
# * libcomps uses StrDict for name_by_lang/desc_by_lang, rpmrepo_metadata uses dict[str, str]
# * libcomps uses IdList for group_ids/option_ids, rpmrepo_metadata uses list[str]/list[CompsEnvironmentOption]
# * libcomps Langpacks is a Dict (name -> install), rpmrepo_metadata uses list[CompsLangpack]

LIBCOMPS_PACKAGE_TYPE_TO_STR = {
    libcomps.PACKAGE_TYPE_DEFAULT: "default",
    libcomps.PACKAGE_TYPE_MANDATORY: "mandatory",
    libcomps.PACKAGE_TYPE_OPTIONAL: "optional",
    libcomps.PACKAGE_TYPE_CONDITIONAL: "conditional",
}


def compare_groups(rpmrepo_group, lc_group):
    assert rpmrepo_group.id == lc_group.id, "group.id"
    assert rpmrepo_group.name == (lc_group.name or ""), "group.name"
    assert rpmrepo_group.description == (lc_group.desc or ""), "group.description"
    assert rpmrepo_group.default == lc_group.default, "group.default"
    assert rpmrepo_group.uservisible == lc_group.uservisible, "group.uservisible"
    assert rpmrepo_group.biarchonly == lc_group.biarchonly, "group.biarchonly"
    assert rpmrepo_group.langonly == lc_group.lang_only, "group.langonly"
    assert rpmrepo_group.display_order == lc_group.display_order, "group.display_order"

    assert rpmrepo_group.name_by_lang == dict(lc_group.name_by_lang), "group.name_by_lang"
    assert rpmrepo_group.desc_by_lang == dict(lc_group.desc_by_lang), "group.desc_by_lang"

    assert len(rpmrepo_group.packages) == len(lc_group.packages), "group.packages length"
    for rpmrepo_pkg, lc_pkg in zip(rpmrepo_group.packages, lc_group.packages):
        assert rpmrepo_pkg.name == lc_pkg.name, "package.name"
        # libcomps maps unrecognized type strings to PACKAGE_TYPE_UNKNOWN (lossy),
        # so we can only compare when the type is one libcomps recognizes.
        if lc_pkg.type != libcomps.PACKAGE_TYPE_UNKNOWN:
            assert rpmrepo_pkg.reqtype == LIBCOMPS_PACKAGE_TYPE_TO_STR[lc_pkg.type], "package.type"
        assert rpmrepo_pkg.requires == lc_pkg.requires, "package.requires"
        assert rpmrepo_pkg.basearchonly == lc_pkg.basearchonly, "package.basearchonly"


def compare_categories(rpmrepo_cat, lc_cat):
    assert rpmrepo_cat.id == lc_cat.id, "category.id"
    assert rpmrepo_cat.name == (lc_cat.name or ""), "category.name"
    assert rpmrepo_cat.description == (lc_cat.desc or ""), "category.description"
    assert rpmrepo_cat.display_order == lc_cat.display_order, "category.display_order"

    assert rpmrepo_cat.name_by_lang == dict(lc_cat.name_by_lang), "category.name_by_lang"
    assert rpmrepo_cat.desc_by_lang == dict(lc_cat.desc_by_lang), "category.desc_by_lang"

    rpmrepo_gids = rpmrepo_cat.group_ids
    lc_gids = [gid.name for gid in lc_cat.group_ids]
    assert rpmrepo_gids == lc_gids, "category.group_ids"


def compare_environments(rpmrepo_env, lc_env):
    assert rpmrepo_env.id == lc_env.id, "environment.id"
    assert rpmrepo_env.name == (lc_env.name or ""), "environment.name"
    assert rpmrepo_env.description == (lc_env.desc or ""), "environment.description"
    assert rpmrepo_env.display_order == lc_env.display_order, "environment.display_order"

    assert rpmrepo_env.name_by_lang == dict(lc_env.name_by_lang), "environment.name_by_lang"
    assert rpmrepo_env.desc_by_lang == dict(lc_env.desc_by_lang), "environment.desc_by_lang"

    rpmrepo_gids = rpmrepo_env.group_ids
    lc_gids = [gid.name for gid in lc_env.group_ids]
    assert rpmrepo_gids == lc_gids, "environment.group_ids"

    rpmrepo_opts = [(opt.group_id, opt.default) for opt in rpmrepo_env.option_ids]
    lc_opts = [(opt.name, opt.default) for opt in lc_env.option_ids]
    assert rpmrepo_opts == lc_opts, "environment.option_ids"


def compare_langpacks(rpmrepo_langpacks, lc_langpacks):
    rpmrepo_dict = {lp.name: lp.install for lp in rpmrepo_langpacks}
    lc_dict = dict(lc_langpacks)
    assert rpmrepo_dict == lc_dict, "langpacks"


def validate_comps(comps_xml_path):
    lc_comps = libcomps.Comps()
    lc_comps.fromxml_f(comps_xml_path)

    with open(comps_xml_path) as f:
        rpmrepo_comps = rpmmd.CompsData.from_xml(f.read())

    assert len(rpmrepo_comps.groups) == len(lc_comps.groups), "groups length"
    for rpmrepo_group, lc_group in zip(rpmrepo_comps.groups, lc_comps.groups):
        compare_groups(rpmrepo_group, lc_group)
        group_dict = rpmrepo_group.to_dict()
        assert group_dict["id"] == rpmrepo_group.id, "group.to_dict()['id']"
        assert group_dict["name_by_lang"] == rpmrepo_group.name_by_lang, (
            "group.to_dict()['name_by_lang']"
        )

    assert len(rpmrepo_comps.categories) == len(lc_comps.categories), "categories length"
    for rpmrepo_cat, lc_cat in zip(rpmrepo_comps.categories, lc_comps.categories):
        compare_categories(rpmrepo_cat, lc_cat)

    assert len(rpmrepo_comps.environments) == len(lc_comps.environments), "environments length"
    for rpmrepo_env, lc_env in zip(rpmrepo_comps.environments, lc_comps.environments):
        compare_environments(rpmrepo_env, lc_env)

    compare_langpacks(rpmrepo_comps.langpacks, lc_comps.langpacks)


def find_comps_files(directory):
    comps_files = []
    for dirpath, _dirnames, filenames in os.walk(directory):
        for filename in filenames:
            if "comps" in filename and filename.endswith(".xml"):
                comps_files.append(os.path.relpath(os.path.join(dirpath, filename), directory))
    return sorted(comps_files)


@pytest.mark.parametrize("path", find_comps_files("tests/assets/external_repos"))
def test_validate_ecosystem_comps(path):
    validate_comps(os.path.join("tests/assets/external_repos", path))


@pytest.mark.parametrize("path", find_comps_files("tests/assets/fixture_repos"))
def test_validate_fixture_comps(path):
    validate_comps(os.path.join("tests/assets/fixture_repos", path))


@pytest.mark.parametrize("path", find_comps_files("tests/assets/broken_fixture_repos"))
def test_validate_broken_comps(path):
    validate_comps(os.path.join("tests/assets/broken_fixture_repos", path))


# Also validate the standalone comps fixture
def test_validate_comps_fixture():
    validate_comps("tests/assets/comps_fixture.xml")


if __name__ == "__main__":
    comps_path = sys.argv[1]
    GREEN = "[32;1m"
    RED = "[31;1m"
    RESET = "[0m"
    try:
        validate_comps(comps_path)
        print(GREEN + "OK" + RESET)
    except AssertionError:
        print(RED + "FAIL" + RESET)
        raise
