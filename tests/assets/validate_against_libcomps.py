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
    assert rpmrepo_group.id == lc_group.id, f"group.id: rpmrepo={rpmrepo_group.id!r} vs libcomps={lc_group.id!r}"
    assert rpmrepo_group.name == (lc_group.name or ""), f"group.name: rpmrepo={rpmrepo_group.name!r} vs libcomps={lc_group.name!r}"
    # API DIFFERENCE: rpmrepo_metadata preserves trailing whitespace from XML (like lxml),
    # while libcomps strips it during parsing
    assert rpmrepo_group.description.strip() == (lc_group.desc or "").strip(), f"group.description: rpmrepo={rpmrepo_group.description!r} vs libcomps={lc_group.desc!r}"
    assert rpmrepo_group.default == lc_group.default, f"group.default: rpmrepo={rpmrepo_group.default!r} vs libcomps={lc_group.default!r}"
    assert rpmrepo_group.uservisible == lc_group.uservisible, f"group.uservisible: rpmrepo={rpmrepo_group.uservisible!r} vs libcomps={lc_group.uservisible!r}"
    assert rpmrepo_group.biarchonly == lc_group.biarchonly, f"group.biarchonly: rpmrepo={rpmrepo_group.biarchonly!r} vs libcomps={lc_group.biarchonly!r}"
    assert rpmrepo_group.langonly == lc_group.lang_only, f"group.langonly: rpmrepo={rpmrepo_group.langonly!r} vs libcomps={lc_group.lang_only!r}"
    assert rpmrepo_group.display_order == lc_group.display_order, f"group.display_order: rpmrepo={rpmrepo_group.display_order!r} vs libcomps={lc_group.display_order!r}"

    lc_name_by_lang = dict(lc_group.name_by_lang)
    assert rpmrepo_group.name_by_lang == lc_name_by_lang, f"group.name_by_lang: rpmrepo={rpmrepo_group.name_by_lang!r} vs libcomps={lc_name_by_lang!r}"
    lc_desc_by_lang = dict(lc_group.desc_by_lang)
    # Strip whitespace from desc_by_lang values for comparison
    rpmrepo_desc_stripped = {k: v.strip() for k, v in rpmrepo_group.desc_by_lang.items()}
    lc_desc_stripped = {k: v.strip() for k, v in lc_desc_by_lang.items()}
    assert rpmrepo_desc_stripped == lc_desc_stripped, f"group.desc_by_lang: rpmrepo={rpmrepo_group.desc_by_lang!r} vs libcomps={lc_desc_by_lang!r}"

    assert len(rpmrepo_group.packages) == len(lc_group.packages), f"group.packages length: rpmrepo={len(rpmrepo_group.packages)} vs libcomps={len(lc_group.packages)}"
    for rpmrepo_pkg, lc_pkg in zip(rpmrepo_group.packages, lc_group.packages):
        assert rpmrepo_pkg.name == lc_pkg.name, f"package.name: rpmrepo={rpmrepo_pkg.name!r} vs libcomps={lc_pkg.name!r}"
        # libcomps maps unrecognized type strings to PACKAGE_TYPE_UNKNOWN (lossy),
        # so we can only compare when the type is one libcomps recognizes.
        if lc_pkg.type != libcomps.PACKAGE_TYPE_UNKNOWN:
            lc_type = LIBCOMPS_PACKAGE_TYPE_TO_STR[lc_pkg.type]
            assert rpmrepo_pkg.reqtype == lc_type, f"package.type: rpmrepo={rpmrepo_pkg.reqtype!r} vs libcomps={lc_type!r}"
        assert rpmrepo_pkg.requires == lc_pkg.requires, f"package.requires: rpmrepo={rpmrepo_pkg.requires!r} vs libcomps={lc_pkg.requires!r}"
        assert rpmrepo_pkg.basearchonly == lc_pkg.basearchonly, f"package.basearchonly: rpmrepo={rpmrepo_pkg.basearchonly!r} vs libcomps={lc_pkg.basearchonly!r}"


def compare_categories(rpmrepo_cat, lc_cat):
    assert rpmrepo_cat.id == lc_cat.id, f"category.id: rpmrepo={rpmrepo_cat.id!r} vs libcomps={lc_cat.id!r}"
    assert rpmrepo_cat.name == (lc_cat.name or ""), f"category.name: rpmrepo={rpmrepo_cat.name!r} vs libcomps={lc_cat.name!r}"
    # API DIFFERENCE: rpmrepo_metadata preserves trailing whitespace from XML (like lxml),
    # while libcomps strips it during parsing
    assert rpmrepo_cat.description.strip() == (lc_cat.desc or "").strip(), f"category.description: rpmrepo={rpmrepo_cat.description!r} vs libcomps={lc_cat.desc!r}"
    assert rpmrepo_cat.display_order == lc_cat.display_order, f"category.display_order: rpmrepo={rpmrepo_cat.display_order!r} vs libcomps={lc_cat.display_order!r}"

    lc_name_by_lang = dict(lc_cat.name_by_lang)
    # Strip whitespace from name_by_lang values for comparison
    rpmrepo_name_stripped = {k: v.strip() for k, v in rpmrepo_cat.name_by_lang.items()}
    lc_name_stripped = {k: v.strip() for k, v in lc_name_by_lang.items()}
    assert rpmrepo_name_stripped == lc_name_stripped, f"category.name_by_lang: rpmrepo={rpmrepo_cat.name_by_lang!r} vs libcomps={lc_name_by_lang!r}"
    lc_desc_by_lang = dict(lc_cat.desc_by_lang)
    # Strip whitespace from desc_by_lang values for comparison
    rpmrepo_desc_stripped = {k: v.strip() for k, v in rpmrepo_cat.desc_by_lang.items()}
    lc_desc_stripped = {k: v.strip() for k, v in lc_desc_by_lang.items()}
    assert rpmrepo_desc_stripped == lc_desc_stripped, f"category.desc_by_lang: rpmrepo={rpmrepo_cat.desc_by_lang!r} vs libcomps={lc_desc_by_lang!r}"

    rpmrepo_gids = rpmrepo_cat.group_ids
    lc_gids = [gid.name for gid in lc_cat.group_ids]
    assert rpmrepo_gids == lc_gids, f"category.group_ids: rpmrepo={rpmrepo_gids!r} vs libcomps={lc_gids!r}"


def compare_environments(rpmrepo_env, lc_env):
    assert rpmrepo_env.id == lc_env.id, f"environment.id: rpmrepo={rpmrepo_env.id!r} vs libcomps={lc_env.id!r}"
    assert rpmrepo_env.name == (lc_env.name or ""), f"environment.name: rpmrepo={rpmrepo_env.name!r} vs libcomps={lc_env.name!r}"
    # API DIFFERENCE: rpmrepo_metadata preserves trailing whitespace from XML (like lxml),
    # while libcomps strips it during parsing
    assert rpmrepo_env.description.strip() == (lc_env.desc or "").strip(), f"environment.description: rpmrepo={rpmrepo_env.description!r} vs libcomps={lc_env.desc!r}"
    assert rpmrepo_env.display_order == lc_env.display_order, f"environment.display_order: rpmrepo={rpmrepo_env.display_order!r} vs libcomps={lc_env.display_order!r}"

    lc_name_by_lang = dict(lc_env.name_by_lang)
    # Strip whitespace from name_by_lang values for comparison
    rpmrepo_name_stripped = {k: v.strip() for k, v in rpmrepo_env.name_by_lang.items()}
    lc_name_stripped = {k: v.strip() for k, v in lc_name_by_lang.items()}
    assert rpmrepo_name_stripped == lc_name_stripped, f"environment.name_by_lang: rpmrepo={rpmrepo_env.name_by_lang!r} vs libcomps={lc_name_by_lang!r}"
    lc_desc_by_lang = dict(lc_env.desc_by_lang)
    # Strip whitespace from desc_by_lang values for comparison
    rpmrepo_desc_stripped = {k: v.strip() for k, v in rpmrepo_env.desc_by_lang.items()}
    lc_desc_stripped = {k: v.strip() for k, v in lc_desc_by_lang.items()}
    assert rpmrepo_desc_stripped == lc_desc_stripped, f"environment.desc_by_lang: rpmrepo={rpmrepo_env.desc_by_lang!r} vs libcomps={lc_desc_by_lang!r}"

    rpmrepo_gids = rpmrepo_env.group_ids
    lc_gids = [gid.name for gid in lc_env.group_ids]
    assert rpmrepo_gids == lc_gids, f"environment.group_ids: rpmrepo={rpmrepo_gids!r} vs libcomps={lc_gids!r}"

    rpmrepo_opts = [(opt.group_id, opt.default) for opt in rpmrepo_env.option_ids]
    lc_opts = [(opt.name, opt.default) for opt in lc_env.option_ids]
    assert rpmrepo_opts == lc_opts, f"environment.option_ids: rpmrepo={rpmrepo_opts!r} vs libcomps={lc_opts!r}"


def compare_langpacks(rpmrepo_langpacks, lc_langpacks):
    rpmrepo_dict = {lp.name: lp.install for lp in rpmrepo_langpacks}
    lc_dict = dict(lc_langpacks)
    assert rpmrepo_dict == lc_dict, f"langpacks: rpmrepo={rpmrepo_dict!r} vs libcomps={lc_dict!r}"


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
