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

def compare_updaterecord(rpmrepo_updaterec, cr_updaterec):
    # API DIFFERENCES vs. createrepo_c
    #
    # * sum_type is a string value, not an integer
    assert rpmrepo_updaterec.fromstr == cr_updaterec.fromstr, "fromstr"
    assert rpmrepo_updaterec.status == cr_updaterec.status, "status"
    assert rpmrepo_updaterec.type == cr_updaterec.type, "type"
    assert rpmrepo_updaterec.version == cr_updaterec.version, "version"
    assert rpmrepo_updaterec.id == cr_updaterec.id, "id"
    assert rpmrepo_updaterec.title == cr_updaterec.title, "title"
    assert rpmrepo_updaterec.issued_date == cr_updaterec.issued_date, "issued_date"
    assert rpmrepo_updaterec.updated_date == cr_updaterec.updated_date, "updated_date"
    assert rpmrepo_updaterec.rights == cr_updaterec.rights, "rights"
    assert rpmrepo_updaterec.release == cr_updaterec.release, "release"
    assert rpmrepo_updaterec.pushcount == cr_updaterec.pushcount, "pushcount"
    assert rpmrepo_updaterec.severity == cr_updaterec.severity, "severity"
    assert rpmrepo_updaterec.summary == cr_updaterec.summary, "summary"
    assert rpmrepo_updaterec.description == cr_updaterec.description, "description"
    assert rpmrepo_updaterec.solution == cr_updaterec.solution, "solution"

    for (rpmrepo_updateref, cr_updateref) in zip(rpmrepo_updaterec.references, cr_updaterec.references):
        assert rpmrepo_updateref.href == cr_updateref.href, "href"
        assert rpmrepo_updateref.id == cr_updateref.id, "id"
        assert rpmrepo_updateref.type == cr_updateref.type, "type"
        assert rpmrepo_updateref.title == cr_updateref.title, "title"

    for (rpmrepo_updatecoll, cr_updatecoll) in zip(rpmrepo_updaterec.collections, cr_updaterec.collections):
        assert rpmrepo_updatecoll.shortname == cr_updatecoll.shortname, "shortname"
        assert rpmrepo_updatecoll.name == cr_updatecoll.name, "name"
        assert rpmrepo_updatecoll.module == cr_updatecoll.module, "module"

        for (rpmrepo_updatepkg, cr_updatepkg) in zip(rpmrepo_updatecoll.packages, cr_updatecoll.packages):
            assert rpmrepo_updatepkg.name == cr_updatepkg.name, "name"
            assert rpmrepo_updatepkg.version == cr_updatepkg.version, "version"
            assert rpmrepo_updatepkg.release == cr_updatepkg.release, "release"
            assert rpmrepo_updatepkg.epoch == cr_updatepkg.epoch, "epoch"
            assert rpmrepo_updatepkg.arch == cr_updatepkg.arch, "arch"
            assert rpmrepo_updatepkg.src == cr_updatepkg.src, "src"
            assert rpmrepo_updatepkg.filename == cr_updatepkg.filename, "filename"
            assert rpmrepo_updatepkg.sum == cr_updatepkg.sum, "sum"
            assert rpmrepo_updatepkg.sum_type == cr.checksum_name_str(cr_updatepkg.sum_type)
            assert rpmrepo_updatepkg.reboot_suggested == cr_updatepkg.reboot_suggested, "reboot_suggested"


def compare_pkgs(rpmrepo_pkg, cr_pkg):
    # API DIFFERENCES vs. createrepo_c
    #
    # * pkgid and checksum_type are read-only
    #   * both are set by the "checksum" getter/setter that takes a tuple of (checksum_type, checksum),
    #     which validates that the length of the checksum matches the checksum type
    # * returns epoch as an integer whereas createrepo_c returns string e.g. '0'
    # * fields that are always present in the metadata are non-nullable, return "" when unset
    # * text fields are stripped of leading and trailing newlines
    #   * This can be fixed but would require some work on the parser
    # * will return "sha1" as checksum type when "sha" was in the metadata
    # * rpm_hreader_range instead of rpm_header_start, rpm_header_end
    # * libxml appears to replace \t (tab) characters in attribute names with spaces, and quick-xml does not
    # * "files" getter/setter uses a 2-tuple of (type, path) instead of a 3-tuple of (type, base, filename)
    #   * the 3-tuple variant is available as "files_split"
    # * rpm_packager -> packager, since the tag name isn't in the rpm: namespace

    assert rpmrepo_pkg.name == cr_pkg.name, "name"
    assert rpmrepo_pkg.epoch == int(cr_pkg.epoch), "epoch"
    assert rpmrepo_pkg.version == cr_pkg.version, "version"
    assert rpmrepo_pkg.release == cr_pkg.release, "release"
    assert rpmrepo_pkg.arch == cr_pkg.arch, "arch"
    assert rpmrepo_pkg.nevra() == cr_pkg.nevra(), "nevra"
    assert rpmrepo_pkg.nvra() == cr_pkg.nvra(), "nvra"
    assert rpmrepo_pkg.pkgid == cr_pkg.pkgId, "pkgid"
    # assert rpmrepo_pkg.checksum_type == createrepo_pkg.checksum_type
    try:
        assert rpmrepo_pkg.checksum == (cr_pkg.checksum_type, cr_pkg.pkgId), "checksum"
    except AssertionError:
        # rpmrepo will return "sha1" instead of "sha" even when the metadata said "sha"
        if cr_pkg.checksum_type != "sha":
            raise
    assert rpmrepo_pkg.summary == (cr_pkg.summary or "").strip(), "summary"
    assert rpmrepo_pkg.description == (cr_pkg.description or "").strip(), "description"
    assert rpmrepo_pkg.packager == (cr_pkg.rpm_packager or ""), "packager"
    assert rpmrepo_pkg.url == (cr_pkg.url or ""), "url"
    assert rpmrepo_pkg.location_href == (cr_pkg.location_href or ""), "location_href"
    assert rpmrepo_pkg.location_base == cr_pkg.location_base, "location_base"
    assert rpmrepo_pkg.time_file == cr_pkg.time_file, "time_file"
    assert rpmrepo_pkg.time_build == cr_pkg.time_build, "time_build"
    assert rpmrepo_pkg.size_package == cr_pkg.size_package, "size_package"
    assert rpmrepo_pkg.size_installed == cr_pkg.size_installed, "size_installed"
    assert rpmrepo_pkg.size_archive == cr_pkg.size_archive, "size_archive"
    assert rpmrepo_pkg.rpm_license == (cr_pkg.rpm_license or "").strip(), "rpm_license"
    assert rpmrepo_pkg.rpm_vendor == (cr_pkg.rpm_vendor or "").strip(), "rpm_vendor"
    assert rpmrepo_pkg.rpm_group == (cr_pkg.rpm_group or "").strip(), "rpm_group"
    assert rpmrepo_pkg.rpm_buildhost == (cr_pkg.rpm_buildhost or "").strip(), "rpm_buildhost"
    assert rpmrepo_pkg.rpm_sourcerpm == (cr_pkg.rpm_sourcerpm or "").strip(), "rpm_sourcerpm"
    assert rpmrepo_pkg.rpm_header_range == (cr_pkg.rpm_header_start, cr_pkg.rpm_header_end), "rpm_header_range"
    # assert rpmrepo_pkg.rpm_header_start == createrepo_pkg.rpm_header_start
    # assert rpmrepo_pkg.rpm_header_end == createrepo_pkg.rpm_header_end

    assert rpmrepo_pkg.files_split == cr_pkg.files, "files"
    assert rpmrepo_pkg.changelogs == cr_pkg.changelogs, "changelogs"

    assert rpmrepo_pkg.requires == cr_pkg.requires, "requires"
    assert rpmrepo_pkg.provides == cr_pkg.provides, "provides"
    assert rpmrepo_pkg.obsoletes == cr_pkg.obsoletes, "obsoletes"
    assert rpmrepo_pkg.recommends == cr_pkg.recommends, "recommends"
    assert rpmrepo_pkg.suggests == cr_pkg.suggests, "suggests"
    assert rpmrepo_pkg.enhances == cr_pkg.enhances, "enhances"
    assert rpmrepo_pkg.supplements == cr_pkg.supplements, "supplements"
    assert rpmrepo_pkg.conflicts == cr_pkg.conflicts, "conflicts"


def validate_rpmrepo(repo_path):
    rpmrepo_reader = rpmmd.RepositoryReader(repo_path)
    cr_reader = cr.RepositoryReader.from_path(repo_path)

    rpmrepo_pkg_parser = rpmrepo_reader.iter_packages()
    cr_pkg_parser = cr_reader.iter_packages()

    for (rpmrepo_pkg, createrepo_pkg) in zip(rpmrepo_pkg_parser, cr_pkg_parser):
        compare_pkgs(rpmrepo_pkg, createrepo_pkg)

    assert rpmrepo_pkg_parser.remaining_packages == 0

    rpmrepo_updates = rpmrepo_reader.iter_advisories()
    cr_updates = cr_reader.advisories()

    for (rpmrepo_updaterecord, createrepo_updaterecord) in zip(rpmrepo_updates, cr_updates):
        compare_updaterecord(rpmrepo_updaterecord, createrepo_updaterecord)

def find_repos(directory):
    def ignorable(path):
        return path.startswith(".") or path.endswith(".md")
    return sorted([path for path in os.listdir(directory) if not ignorable(path)])


@pytest.mark.parametrize("path", find_repos("tests/assets/external_repos"))
def test_validate_ecosystem_repo(path):
    validate_rpmrepo(os.path.join("tests/assets/external_repos", path))


@pytest.mark.parametrize("path", find_repos("tests/assets/fixture_repos"))
def test_validate_fixture_repo(path):
    validate_rpmrepo(os.path.join("tests/assets/fixture_repos", path))


@pytest.mark.parametrize("path", find_repos("tests/assets/broken_fixture_repos"))
def test_validate_broken_repo(path):
    validate_rpmrepo(os.path.join("tests/assets/broken_fixture_repos", path))


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
        raise
