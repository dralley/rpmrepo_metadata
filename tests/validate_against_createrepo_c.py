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
    # * rpmrepo_metadata returns "" for empty/missing IDs, while createrepo_c returns None
    assert rpmrepo_updaterec.fromstr == cr_updaterec.fromstr, f"fromstr: rpmrepo={rpmrepo_updaterec.fromstr!r} vs createrepo_c={cr_updaterec.fromstr!r}"
    assert rpmrepo_updaterec.status == cr_updaterec.status, f"status: rpmrepo={rpmrepo_updaterec.status!r} vs createrepo_c={cr_updaterec.status!r}"
    assert rpmrepo_updaterec.update_type == cr_updaterec.type, f"type: rpmrepo={rpmrepo_updaterec.update_type!r} vs createrepo_c={cr_updaterec.type!r}"
    assert rpmrepo_updaterec.version == cr_updaterec.version, f"version: rpmrepo={rpmrepo_updaterec.version!r} vs createrepo_c={cr_updaterec.version!r}"
    # API DIFFERENCE: rpmrepo_metadata returns "" for empty IDs, createrepo_c returns None
    rpmrepo_id = rpmrepo_updaterec.id or None
    cr_id = cr_updaterec.id or None
    assert rpmrepo_id == cr_id, f"id: rpmrepo={rpmrepo_updaterec.id!r} vs createrepo_c={cr_updaterec.id!r}"
    assert rpmrepo_updaterec.title == cr_updaterec.title, f"title: rpmrepo={rpmrepo_updaterec.title!r} vs createrepo_c={cr_updaterec.title!r}"
    # API DIFFERENCE: rpmrepo_metadata returns the raw date string from the XML attribute,
    # while createrepo_c parses it into a datetime object.
    # Normalize both to just the date portion (YYYY-MM-DD) by:
    # - For createrepo_c: use .date() to get just the date part
    # - For rpmrepo: split on space/time and take first part (handles "YYYY-MM-DD", "YYYY-MM-DD HH:MM:SS", "YYYY-MM-DD UTC")
    cr_issued = str(cr_updaterec.issued_date.date()) if cr_updaterec.issued_date is not None else None
    cr_updated = str(cr_updaterec.updated_date.date()) if cr_updaterec.updated_date is not None else None
    rpmrepo_issued = rpmrepo_updaterec.issued_date.split()[0] if rpmrepo_updaterec.issued_date else None
    rpmrepo_updated = rpmrepo_updaterec.updated_date.split()[0] if rpmrepo_updaterec.updated_date else None
    assert rpmrepo_issued == cr_issued, f"issued_date: rpmrepo={rpmrepo_issued!r} vs createrepo_c={cr_issued!r}"
    assert rpmrepo_updated == cr_updated, f"updated_date: rpmrepo={rpmrepo_updated!r} vs createrepo_c={cr_updated!r}"
    assert rpmrepo_updaterec.rights == cr_updaterec.rights, f"rights: rpmrepo={rpmrepo_updaterec.rights!r} vs createrepo_c={cr_updaterec.rights!r}"
    assert rpmrepo_updaterec.release == cr_updaterec.release, f"release: rpmrepo={rpmrepo_updaterec.release!r} vs createrepo_c={cr_updaterec.release!r}"
    assert rpmrepo_updaterec.pushcount == cr_updaterec.pushcount, f"pushcount: rpmrepo={rpmrepo_updaterec.pushcount!r} vs createrepo_c={cr_updaterec.pushcount!r}"
    assert rpmrepo_updaterec.severity == cr_updaterec.severity, f"severity: rpmrepo={rpmrepo_updaterec.severity!r} vs createrepo_c={cr_updaterec.severity!r}"
    assert rpmrepo_updaterec.summary == cr_updaterec.summary, f"summary: rpmrepo={rpmrepo_updaterec.summary!r} vs createrepo_c={cr_updaterec.summary!r}"
    assert rpmrepo_updaterec.description == cr_updaterec.description, f"description: rpmrepo={rpmrepo_updaterec.description!r} vs createrepo_c={cr_updaterec.description!r}"
    assert rpmrepo_updaterec.solution == cr_updaterec.solution, f"solution: rpmrepo={rpmrepo_updaterec.solution!r} vs createrepo_c={cr_updaterec.solution!r}"
    assert rpmrepo_updaterec.reboot_suggested == cr_updaterec.reboot_suggested, f"reboot_suggested: rpmrepo={rpmrepo_updaterec.reboot_suggested!r} vs createrepo_c={cr_updaterec.reboot_suggested!r}"

    for rpmrepo_updateref, cr_updateref in zip(rpmrepo_updaterec.references, cr_updaterec.references):
        assert rpmrepo_updateref.href == cr_updateref.href, f"href: rpmrepo={rpmrepo_updateref.href!r} vs createrepo_c={cr_updateref.href!r}"
        # API DIFFERENCE: rpmrepo_metadata returns "" for empty reference IDs, createrepo_c returns None
        rpmrepo_ref_id = rpmrepo_updateref.id or None
        cr_ref_id = cr_updateref.id or None
        assert rpmrepo_ref_id == cr_ref_id, f"id: rpmrepo={rpmrepo_updateref.id!r} vs createrepo_c={cr_updateref.id!r}"
        assert rpmrepo_updateref.reftype == cr_updateref.type, f"type: rpmrepo={rpmrepo_updateref.reftype!r} vs createrepo_c={cr_updateref.type!r}"
        assert rpmrepo_updateref.title == cr_updateref.title, f"title: rpmrepo={rpmrepo_updateref.title!r} vs createrepo_c={cr_updateref.title!r}"

    for rpmrepo_updatecoll, cr_updatecoll in zip(rpmrepo_updaterec.pkglist, cr_updaterec.collections):
        assert rpmrepo_updatecoll.shortname == cr_updatecoll.shortname, f"shortname: rpmrepo={rpmrepo_updatecoll.shortname!r} vs createrepo_c={cr_updatecoll.shortname!r}"
        assert rpmrepo_updatecoll.name == cr_updatecoll.name, f"name: rpmrepo={rpmrepo_updatecoll.name!r} vs createrepo_c={cr_updatecoll.name!r}"
        if rpmrepo_updatecoll.module is not None:
            assert cr_updatecoll.module is not None, f"module: rpmrepo={rpmrepo_updatecoll.module!r} vs createrepo_c={cr_updatecoll.module!r}"
            assert rpmrepo_updatecoll.module.name == cr_updatecoll.module.name, f"module.name: rpmrepo={rpmrepo_updatecoll.module.name!r} vs createrepo_c={cr_updatecoll.module.name!r}"
            assert rpmrepo_updatecoll.module.stream == cr_updatecoll.module.stream, f"module.stream: rpmrepo={rpmrepo_updatecoll.module.stream!r} vs createrepo_c={cr_updatecoll.module.stream!r}"
            assert rpmrepo_updatecoll.module.version == cr_updatecoll.module.version, f"module.version: rpmrepo={rpmrepo_updatecoll.module.version!r} vs createrepo_c={cr_updatecoll.module.version!r}"
            assert rpmrepo_updatecoll.module.context == cr_updatecoll.module.context, f"module.context: rpmrepo={rpmrepo_updatecoll.module.context!r} vs createrepo_c={cr_updatecoll.module.context!r}"
            assert rpmrepo_updatecoll.module.arch == cr_updatecoll.module.arch, f"module.arch: rpmrepo={rpmrepo_updatecoll.module.arch!r} vs createrepo_c={cr_updatecoll.module.arch!r}"
        else:
            assert cr_updatecoll.module is None, f"module: rpmrepo={rpmrepo_updatecoll.module!r} vs createrepo_c={cr_updatecoll.module!r}"

        for rpmrepo_updatepkg, cr_updatepkg in zip(rpmrepo_updatecoll.packages, cr_updatecoll.packages):
            assert rpmrepo_updatepkg.name == cr_updatepkg.name, f"name: rpmrepo={rpmrepo_updatepkg.name!r} vs createrepo_c={cr_updatepkg.name!r}"
            assert rpmrepo_updatepkg.version == cr_updatepkg.version, f"version: rpmrepo={rpmrepo_updatepkg.version!r} vs createrepo_c={cr_updatepkg.version!r}"
            assert rpmrepo_updatepkg.release == cr_updatepkg.release, f"release: rpmrepo={rpmrepo_updatepkg.release!r} vs createrepo_c={cr_updatepkg.release!r}"
            assert rpmrepo_updatepkg.epoch == cr_updatepkg.epoch, f"epoch: rpmrepo={rpmrepo_updatepkg.epoch!r} vs createrepo_c={cr_updatepkg.epoch!r}"
            assert rpmrepo_updatepkg.arch == cr_updatepkg.arch, f"arch: rpmrepo={rpmrepo_updatepkg.arch!r} vs createrepo_c={cr_updatepkg.arch!r}"
            assert rpmrepo_updatepkg.src == cr_updatepkg.src, f"src: rpmrepo={rpmrepo_updatepkg.src!r} vs createrepo_c={cr_updatepkg.src!r}"
            assert rpmrepo_updatepkg.filename == cr_updatepkg.filename, f"filename: rpmrepo={rpmrepo_updatepkg.filename!r} vs createrepo_c={cr_updatepkg.filename!r}"
            if rpmrepo_updatepkg.checksum is not None:
                sum_type, sum_value = rpmrepo_updatepkg.checksum
                assert sum_value == cr_updatepkg.sum, f"sum: rpmrepo={sum_value!r} vs createrepo_c={cr_updatepkg.sum!r}"
                cr_sum_type = cr.checksum_name_str(cr_updatepkg.sum_type)
                # Skip checksum type validation when createrepo_c returns "Unknown checksum"
                # This happens for deprecated checksum types like MD5 in old repos
                if cr_sum_type != "Unknown checksum":
                    assert sum_type == cr_sum_type, f"sum_type: rpmrepo={sum_type!r} vs createrepo_c={cr_sum_type!r}"
            else:
                assert cr_updatepkg.sum is None, f"sum: rpmrepo={rpmrepo_updatepkg.checksum!r} vs createrepo_c={cr_updatepkg.sum!r}"
            assert rpmrepo_updatepkg.reboot_suggested == cr_updatepkg.reboot_suggested, f"reboot_suggested: rpmrepo={rpmrepo_updatepkg.reboot_suggested!r} vs createrepo_c={cr_updatepkg.reboot_suggested!r}"
            assert rpmrepo_updatepkg.restart_suggested == cr_updatepkg.restart_suggested, f"restart_suggested: rpmrepo={rpmrepo_updatepkg.restart_suggested!r} vs createrepo_c={cr_updatepkg.restart_suggested!r}"
            assert rpmrepo_updatepkg.relogin_suggested == cr_updatepkg.relogin_suggested, f"relogin_suggested: rpmrepo={rpmrepo_updatepkg.relogin_suggested!r} vs createrepo_c={cr_updatepkg.relogin_suggested!r}"


def compare_pkgs(rpmrepo_pkg, cr_pkg):
    # API DIFFERENCES vs. createrepo_c
    #
    # * pkgid and checksum_type are read-only
    #   * both are set by the "checksum" getter/setter that takes a tuple of (checksum_type, checksum),
    #     which validates that the length of the checksum matches the checksum type
    # * returns epoch as an integer whereas createrepo_c returns string e.g. '0'
    # * fields that are always present in the metadata are non-nullable, return "" when unset
    # * will return "sha1" as checksum type when "sha" was in the metadata
    # * rpm_hreader_range instead of rpm_header_start, rpm_header_end
    # * "files" getter/setter uses a 2-tuple of (type, path) instead of a 3-tuple of (type, base, filename)
    #   * the 3-tuple variant is available as "files_split"
    # * rpm_packager -> packager, since the tag name isn't in the rpm: namespace

    assert rpmrepo_pkg.name == cr_pkg.name, f"name: rpmrepo={rpmrepo_pkg.name!r} vs createrepo_c={cr_pkg.name!r}"
    cr_epoch = int(cr_pkg.epoch or 0)
    assert rpmrepo_pkg.epoch == cr_epoch, f"epoch: rpmrepo={rpmrepo_pkg.epoch!r} vs createrepo_c={cr_epoch!r}"
    assert rpmrepo_pkg.version == cr_pkg.version, f"version: rpmrepo={rpmrepo_pkg.version!r} vs createrepo_c={cr_pkg.version!r}"
    assert rpmrepo_pkg.release == cr_pkg.release, f"release: rpmrepo={rpmrepo_pkg.release!r} vs createrepo_c={cr_pkg.release!r}"
    assert rpmrepo_pkg.arch == cr_pkg.arch, f"arch: rpmrepo={rpmrepo_pkg.arch!r} vs createrepo_c={cr_pkg.arch!r}"
    rpmrepo_nevra = rpmrepo_pkg.nevra()
    cr_nevra = cr_pkg.nevra()
    assert rpmrepo_nevra == cr_nevra, f"nevra: rpmrepo={rpmrepo_nevra!r} vs createrepo_c={cr_nevra!r}"
    rpmrepo_nvra = rpmrepo_pkg.nvra()
    cr_nvra = cr_pkg.nvra()
    assert rpmrepo_nvra == cr_nvra, f"nvra: rpmrepo={rpmrepo_nvra!r} vs createrepo_c={cr_nvra!r}"
    assert rpmrepo_pkg.pkgid == cr_pkg.pkgId, f"pkgid: rpmrepo={rpmrepo_pkg.pkgid!r} vs createrepo_c={cr_pkg.pkgId!r}"
    # assert rpmrepo_pkg.checksum_type == createrepo_pkg.checksum_type
    cr_checksum = (cr_pkg.checksum_type, cr_pkg.pkgId)
    try:
        assert rpmrepo_pkg.checksum == cr_checksum, f"checksum: rpmrepo={rpmrepo_pkg.checksum!r} vs createrepo_c={cr_checksum!r}"
    except AssertionError:
        # rpmrepo will return "sha1" instead of "sha" even when the metadata said "sha"
        if cr_pkg.checksum_type != "sha":
            raise
    cr_summary = cr_pkg.summary or ""
    assert rpmrepo_pkg.summary == cr_summary, f"summary: rpmrepo={rpmrepo_pkg.summary!r} vs createrepo_c={cr_summary!r}"
    cr_description = cr_pkg.description or ""
    assert rpmrepo_pkg.description == cr_description, f"description: rpmrepo={rpmrepo_pkg.description!r} vs createrepo_c={cr_description!r}"
    cr_packager = cr_pkg.rpm_packager or ""
    assert rpmrepo_pkg.packager == cr_packager, f"packager: rpmrepo={rpmrepo_pkg.packager!r} vs createrepo_c={cr_packager!r}"
    cr_url = cr_pkg.url or ""
    assert rpmrepo_pkg.url == cr_url, f"url: rpmrepo={rpmrepo_pkg.url!r} vs createrepo_c={cr_url!r}"
    cr_location_href = cr_pkg.location_href or ""
    assert rpmrepo_pkg.location_href == cr_location_href, f"location_href: rpmrepo={rpmrepo_pkg.location_href!r} vs createrepo_c={cr_location_href!r}"
    assert rpmrepo_pkg.location_base == cr_pkg.location_base, f"location_base: rpmrepo={rpmrepo_pkg.location_base!r} vs createrepo_c={cr_pkg.location_base!r}"
    assert rpmrepo_pkg.time_file == cr_pkg.time_file, f"time_file: rpmrepo={rpmrepo_pkg.time_file!r} vs createrepo_c={cr_pkg.time_file!r}"
    assert rpmrepo_pkg.time_build == cr_pkg.time_build, f"time_build: rpmrepo={rpmrepo_pkg.time_build!r} vs createrepo_c={cr_pkg.time_build!r}"
    assert rpmrepo_pkg.size_package == cr_pkg.size_package, f"size_package: rpmrepo={rpmrepo_pkg.size_package!r} vs createrepo_c={cr_pkg.size_package!r}"
    assert rpmrepo_pkg.size_installed == cr_pkg.size_installed, f"size_installed: rpmrepo={rpmrepo_pkg.size_installed!r} vs createrepo_c={cr_pkg.size_installed!r}"
    assert rpmrepo_pkg.size_archive == cr_pkg.size_archive, f"size_archive: rpmrepo={rpmrepo_pkg.size_archive!r} vs createrepo_c={cr_pkg.size_archive!r}"
    cr_license = cr_pkg.rpm_license or ""
    assert rpmrepo_pkg.rpm_license == cr_license, f"rpm_license: rpmrepo={rpmrepo_pkg.rpm_license!r} vs createrepo_c={cr_license!r}"
    cr_vendor = cr_pkg.rpm_vendor or ""
    assert rpmrepo_pkg.rpm_vendor == cr_vendor, f"rpm_vendor: rpmrepo={rpmrepo_pkg.rpm_vendor!r} vs createrepo_c={cr_vendor!r}"
    cr_group = cr_pkg.rpm_group or ""
    assert rpmrepo_pkg.rpm_group == cr_group, f"rpm_group: rpmrepo={rpmrepo_pkg.rpm_group!r} vs createrepo_c={cr_group!r}"
    cr_buildhost = cr_pkg.rpm_buildhost or ""
    assert rpmrepo_pkg.rpm_buildhost == cr_buildhost, f"rpm_buildhost: rpmrepo={rpmrepo_pkg.rpm_buildhost!r} vs createrepo_c={cr_buildhost!r}"
    cr_sourcerpm = cr_pkg.rpm_sourcerpm or ""
    assert rpmrepo_pkg.rpm_sourcerpm == cr_sourcerpm, f"rpm_sourcerpm: rpmrepo={rpmrepo_pkg.rpm_sourcerpm!r} vs createrepo_c={cr_sourcerpm!r}"
    cr_header_range = (cr_pkg.rpm_header_start, cr_pkg.rpm_header_end)
    assert rpmrepo_pkg.rpm_header_range == cr_header_range, f"rpm_header_range: rpmrepo={rpmrepo_pkg.rpm_header_range!r} vs createrepo_c={cr_header_range!r}"

    assert rpmrepo_pkg.files_split == cr_pkg.files, f"files: rpmrepo={rpmrepo_pkg.files_split!r} vs createrepo_c={cr_pkg.files!r}"
    assert rpmrepo_pkg.changelogs == cr_pkg.changelogs, f"changelogs: rpmrepo={rpmrepo_pkg.changelogs!r} vs createrepo_c={cr_pkg.changelogs!r}"

    assert rpmrepo_pkg.requires == cr_pkg.requires, f"requires: rpmrepo={rpmrepo_pkg.requires!r} vs createrepo_c={cr_pkg.requires!r}"
    assert rpmrepo_pkg.provides == cr_pkg.provides, f"provides: rpmrepo={rpmrepo_pkg.provides!r} vs createrepo_c={cr_pkg.provides!r}"
    assert rpmrepo_pkg.obsoletes == cr_pkg.obsoletes, f"obsoletes: rpmrepo={rpmrepo_pkg.obsoletes!r} vs createrepo_c={cr_pkg.obsoletes!r}"
    assert rpmrepo_pkg.recommends == cr_pkg.recommends, f"recommends: rpmrepo={rpmrepo_pkg.recommends!r} vs createrepo_c={cr_pkg.recommends!r}"
    assert rpmrepo_pkg.suggests == cr_pkg.suggests, f"suggests: rpmrepo={rpmrepo_pkg.suggests!r} vs createrepo_c={cr_pkg.suggests!r}"
    assert rpmrepo_pkg.enhances == cr_pkg.enhances, f"enhances: rpmrepo={rpmrepo_pkg.enhances!r} vs createrepo_c={cr_pkg.enhances!r}"
    assert rpmrepo_pkg.supplements == cr_pkg.supplements, f"supplements: rpmrepo={rpmrepo_pkg.supplements!r} vs createrepo_c={cr_pkg.supplements!r}"
    assert rpmrepo_pkg.conflicts == cr_pkg.conflicts, f"conflicts: rpmrepo={rpmrepo_pkg.conflicts!r} vs createrepo_c={cr_pkg.conflicts!r}"


def validate_rpmrepo(repo_path):
    rpmrepo_reader = rpmmd.RepositoryReader(repo_path)
    cr_reader = cr.RepositoryReader.from_path(repo_path)

    rpmrepo_pkg_parser = rpmrepo_reader.iter_packages()
    cr_pkg_parser = cr_reader.iter_packages()

    for rpmrepo_pkg, createrepo_pkg in zip(rpmrepo_pkg_parser, cr_pkg_parser):
        compare_pkgs(rpmrepo_pkg, createrepo_pkg)

    assert rpmrepo_pkg_parser.remaining_packages == 0

    rpmrepo_updates = rpmrepo_reader.iter_advisories()
    cr_updates = cr_reader.advisories()

    for rpmrepo_updaterecord, createrepo_updaterecord in zip(rpmrepo_updates, cr_updates):
        compare_updaterecord(rpmrepo_updaterecord, createrepo_updaterecord)


def find_repos(directory):
    def ignorable(name):
        return name.startswith(".") or name.endswith(".md")

    repos = []
    for dirpath, dirnames, _filenames in os.walk(directory):
        dirnames[:] = [d for d in dirnames if not ignorable(d) and d != "repodata"]
        if "repodata" in os.listdir(dirpath):
            repos.append(os.path.relpath(dirpath, directory))
    return sorted(repos)


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
