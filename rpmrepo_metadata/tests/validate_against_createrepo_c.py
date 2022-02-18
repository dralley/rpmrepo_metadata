#!/usr/bin/env python3

# A utility for testing the output of the rpmrepo_metadata library against createrepo_c
# Copyright (C) 2022 Daniel Alley

# The following GPL-2.0 license notice applies to this file (only)
# by virtue of using createrepo_c, a GPL-2.0 licensed library.
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

import createrepo_c as cr
import rpmrepo_metadata as rpmmd


def compare_pkgs(rpmrepo_pkg, createrepo_pkg):
    # API DIFFERENCES vs. createrepo_c
    #
    # * pkgid and checksum_type are read-only
    #   * both are set by the "checksum" getter/setter that takes a tuple of (checksum_type, checksum),
    #     which validates that the length of the checksum matches the checksum type
    # * fields that are always present in the metadata are non-nullable, return "" when unset
    # * text fields are stripped of leading and trailing newlines
    #   * This can be fixed but would require some work on the parser
    # * will return "sha1" as checksum type when "sha" was in the metadata
    # * rpm_hreader_range instead of rpm_header_start, rpm_header_end
    # * libxml appears to replace \t (tab) characters in attribute names with spaces, and quick-xml does not
    # * "files" getter/setter uses a 2-tuple of (type, path) instead of a 3-tuple of (type, base, filename)
    #   * the 3-tuple variant is available as "files_split"

    assert rpmrepo_pkg.name == createrepo_pkg.name, "name"
    assert rpmrepo_pkg.epoch == createrepo_pkg.epoch, "epoch"
    assert rpmrepo_pkg.version == createrepo_pkg.version, "version"
    assert rpmrepo_pkg.release == createrepo_pkg.release, "release"
    assert rpmrepo_pkg.arch == createrepo_pkg.arch, "arch"
    assert rpmrepo_pkg.nevra() == createrepo_pkg.nevra(), "nevra"
    assert rpmrepo_pkg.nvra() == createrepo_pkg.nvra(), "nvra"
    assert rpmrepo_pkg.pkgid == createrepo_pkg.pkgId, "pkgid"
    assert rpmrepo_pkg.checksum_type == createrepo_pkg.checksum_type
    try:
        assert rpmrepo_pkg.checksum == (createrepo_pkg.checksum_type, createrepo_pkg.pkgId), "checksum"
    except AssertionError:
        # rpmrepo will return "sha1" instead of "sha" even when the metadata said "sha"
        if createrepo_pkg.checksum_type != "sha":
            raise
    assert rpmrepo_pkg.summary == (createrepo_pkg.summary or "").strip(), "summary"
    assert rpmrepo_pkg.description == (createrepo_pkg.description or "").strip(), "description"
    assert rpmrepo_pkg.packager == (createrepo_pkg.rpm_packager or ""), "packager"
    assert rpmrepo_pkg.url == (createrepo_pkg.url or ""), "url"
    assert rpmrepo_pkg.location_href == (createrepo_pkg.location_href or ""), "location_href"
    assert rpmrepo_pkg.location_base == createrepo_pkg.location_base, "location_base"
    assert rpmrepo_pkg.time_file == createrepo_pkg.time_file, "time_file"
    assert rpmrepo_pkg.time_build == createrepo_pkg.time_build, "time_build"
    assert rpmrepo_pkg.size_package == createrepo_pkg.size_package, "size_package"
    assert rpmrepo_pkg.size_installed == createrepo_pkg.size_installed, "size_installed"
    assert rpmrepo_pkg.size_archive == createrepo_pkg.size_archive, "size_archive"
    assert rpmrepo_pkg.rpm_license == (createrepo_pkg.rpm_license or "").strip(), "rpm_license"
    assert rpmrepo_pkg.rpm_vendor == (createrepo_pkg.rpm_vendor or "").strip(), "rpm_vendor"
    assert rpmrepo_pkg.rpm_group == (createrepo_pkg.rpm_group or "").strip(), "rpm_group"
    assert rpmrepo_pkg.rpm_buildhost == (createrepo_pkg.rpm_buildhost or "").strip(), "rpm_buildhost"
    assert rpmrepo_pkg.rpm_sourcerpm == (createrepo_pkg.rpm_sourcerpm or "").strip(), "rpm_sourcerpm"
    assert rpmrepo_pkg.rpm_header_range == (createrepo_pkg.rpm_header_start, createrepo_pkg.rpm_header_end), "rpm_header_range"
    # assert rpmrepo_pkg.rpm_header_start == createrepo_pkg.rpm_header_start
    # assert rpmrepo_pkg.rpm_header_end == createrepo_pkg.rpm_header_end

    assert rpmrepo_pkg.files_split == createrepo_pkg.files, "files"
    assert rpmrepo_pkg.changelogs == createrepo_pkg.changelogs, "changelogs"

    assert rpmrepo_pkg.requires == createrepo_pkg.requires, "requires"
    assert rpmrepo_pkg.provides == createrepo_pkg.provides, "provides"
    assert rpmrepo_pkg.obsoletes == createrepo_pkg.obsoletes, "obsoletes"
    assert rpmrepo_pkg.recommends == createrepo_pkg.recommends, "recommends"
    assert rpmrepo_pkg.suggests == createrepo_pkg.suggests, "suggests"
    assert rpmrepo_pkg.enhances == createrepo_pkg.enhances, "enhances"
    assert rpmrepo_pkg.supplements == createrepo_pkg.supplements, "supplements"
    assert rpmrepo_pkg.conflicts == createrepo_pkg.conflicts, "conflicts"


def validate_rpmrepo(repo_path):
    primary_xml_path   = None
    filelists_xml_path = None
    other_xml_path     = None

    repomd = cr.Repomd(os.path.join(repo_path, "repodata/repomd.xml"))
    # TODO: warnings?

    for record in repomd.records:
        if record.type == "primary":
            primary_xml_path = os.path.join(repo_path, record.location_href)
        elif record.type == "filelists":
            filelists_xml_path = os.path.join(repo_path, record.location_href)
        elif record.type == "other":
            other_xml_path = os.path.join(repo_path, record.location_href)

    parser = rpmmd.PackageParser(primary_xml_path, filelists_xml_path, other_xml_path)

    def pkgcb(createrepo_pkg):
        rpmrepo_pkg = next(parser)
        compare_pkgs(rpmrepo_pkg, createrepo_pkg)

    cr.xml_parse_main_metadata_together(primary_xml_path,
                                        filelists_xml_path,
                                        other_xml_path,
                                        None,
                                        pkgcb,
                                        None,
                                        False)

    assert parser.remaining_packages == 0


def find_repos(directory):
    return sorted([path for path in os.listdir(directory) if not path.startswith(".")])


@pytest.mark.parametrize("path", find_repos("tests/assets/external_repos"))
def test_validate_ecosystem_repo(path):
    validate_rpmrepo(os.path.join("tests/assets/external_repos", path))


@pytest.mark.parametrize("path", find_repos("tests/assets/fixture_repos"))
def test_validate_fixture_repo(path):
    validate_rpmrepo(os.path.join("tests/assets/fixture_repos", path))


if __name__ == "__main__":
    repo_path = sys.argv[1]
    GREEN = "\u001b[32;1m"
    RED = "\u001b[31;1m"
    RESET = "\u001b[0m"
    try:
        validate_rpmrepo(repo_path)
        print(GREEN + "OK" + RESET)
    except AssertionError:
        print(RED + "FAIL" + RESET)
        sys.exit(1)
