"""Shared comparison routines for rpmrepo_metadata objects.

Each function first tries a fast __eq__ check via the Rust PartialEq derive.
On mismatch it falls through to field-by-field assertions so failures pinpoint
exactly which field differs.
"""


def assert_eq(a, b, label):
    assert a == b, f"{label}: {a!r} != {b!r}"


def compare_lists(list1, list2, label, item_compare_fn):
    assert len(list1) == len(list2), f"{label}: length {len(list1)} != {len(list2)}"
    for i, (a, b) in enumerate(zip(list1, list2)):
        if a != b:
            item_compare_fn(a, b)
            assert False, f"{label}[{i}]: items differ but field-by-field found no difference"


# ---------------------------------------------------------------------------
# Package
# ---------------------------------------------------------------------------


def _compare_packages_fields(p1, p2):
    assert_eq(p1.name, p2.name, "name")
    assert_eq(p1.epoch, p2.epoch, "epoch")
    assert_eq(p1.version, p2.version, "version")
    assert_eq(p1.release, p2.release, "release")
    assert_eq(p1.arch, p2.arch, "arch")
    assert_eq(p1.checksum, p2.checksum, "checksum")
    assert_eq(p1.checksum_type, p2.checksum_type, "checksum_type")
    assert_eq(p1.pkgid, p2.pkgid, "pkgid")
    assert_eq(p1.nevra(), p2.nevra(), "nevra")
    assert_eq(p1.nvra(), p2.nvra(), "nvra")
    assert_eq(p1.summary, p2.summary, "summary")
    assert_eq(p1.description, p2.description, "description")
    assert_eq(p1.packager, p2.packager, "packager")
    assert_eq(p1.url, p2.url, "url")
    assert_eq(p1.location_href, p2.location_href, "location_href")
    assert_eq(p1.location_base, p2.location_base, "location_base")
    assert_eq(p1.time_file, p2.time_file, "time_file")
    assert_eq(p1.time_build, p2.time_build, "time_build")
    assert_eq(p1.size_package, p2.size_package, "size_package")
    assert_eq(p1.size_installed, p2.size_installed, "size_installed")
    assert_eq(p1.size_archive, p2.size_archive, "size_archive")
    assert_eq(p1.rpm_license, p2.rpm_license, "rpm_license")
    assert_eq(p1.rpm_vendor, p2.rpm_vendor, "rpm_vendor")
    assert_eq(p1.rpm_group, p2.rpm_group, "rpm_group")
    assert_eq(p1.rpm_buildhost, p2.rpm_buildhost, "rpm_buildhost")
    assert_eq(p1.rpm_sourcerpm, p2.rpm_sourcerpm, "rpm_sourcerpm")
    assert_eq(p1.rpm_header_range, p2.rpm_header_range, "rpm_header_range")
    assert_eq(p1.files, p2.files, "files")
    assert_eq(p1.files_split, p2.files_split, "files_split")
    assert_eq(p1.changelogs, p2.changelogs, "changelogs")
    assert_eq(p1.requires, p2.requires, "requires")
    assert_eq(p1.provides, p2.provides, "provides")
    assert_eq(p1.conflicts, p2.conflicts, "conflicts")
    assert_eq(p1.obsoletes, p2.obsoletes, "obsoletes")
    assert_eq(p1.suggests, p2.suggests, "suggests")
    assert_eq(p1.enhances, p2.enhances, "enhances")
    assert_eq(p1.recommends, p2.recommends, "recommends")
    assert_eq(p1.supplements, p2.supplements, "supplements")


def compare_packages(p1, p2):
    if p1 == p2:
        return
    _compare_packages_fields(p1, p2)


def compare_package_lists(pkgs1, pkgs2):
    compare_lists(pkgs1, pkgs2, "packages", _compare_packages_fields)


# ---------------------------------------------------------------------------
# UpdateInfo / Advisories
# ---------------------------------------------------------------------------


def _compare_update_collection_modules_fields(m1, m2):
    assert_eq(m1.name, m2.name, "module.name")
    assert_eq(m1.stream, m2.stream, "module.stream")
    assert_eq(m1.version, m2.version, "module.version")
    assert_eq(m1.context, m2.context, "module.context")
    assert_eq(m1.arch, m2.arch, "module.arch")


def _compare_update_collection_packages_fields(cp1, cp2):
    assert_eq(cp1.name, cp2.name, "collection_pkg.name")
    assert_eq(cp1.version, cp2.version, "collection_pkg.version")
    assert_eq(cp1.release, cp2.release, "collection_pkg.release")
    assert_eq(cp1.epoch, cp2.epoch, "collection_pkg.epoch")
    assert_eq(cp1.arch, cp2.arch, "collection_pkg.arch")
    assert_eq(cp1.src, cp2.src, "collection_pkg.src")
    assert_eq(cp1.filename, cp2.filename, "collection_pkg.filename")
    assert_eq(cp1.checksum, cp2.checksum, "collection_pkg.checksum")
    assert_eq(cp1.reboot_suggested, cp2.reboot_suggested, "collection_pkg.reboot_suggested")
    assert_eq(cp1.restart_suggested, cp2.restart_suggested, "collection_pkg.restart_suggested")
    assert_eq(cp1.relogin_suggested, cp2.relogin_suggested, "collection_pkg.relogin_suggested")


def _compare_update_collections_fields(c1, c2):
    assert_eq(c1.name, c2.name, "collection.name")
    assert_eq(c1.shortname, c2.shortname, "collection.shortname")
    if c1.module is not None:
        assert c2.module is not None, "collection.module: expected non-None"
        if c1.module != c2.module:
            _compare_update_collection_modules_fields(c1.module, c2.module)
    else:
        assert c2.module is None, "collection.module: expected None"
    compare_lists(
        c1.packages,
        c2.packages,
        "collection.packages",
        _compare_update_collection_packages_fields,
    )


def _compare_update_references_fields(r1, r2):
    assert_eq(r1.href, r2.href, "reference.href")
    assert_eq(r1.id, r2.id, "reference.id")
    assert_eq(r1.title, r2.title, "reference.title")
    assert_eq(r1.reftype, r2.reftype, "reference.reftype")


def _compare_update_records_fields(rec1, rec2):
    assert_eq(rec1.id, rec2.id, "advisory.id")
    assert_eq(rec1.fromstr, rec2.fromstr, "advisory.fromstr")
    assert_eq(rec1.status, rec2.status, "advisory.status")
    assert_eq(rec1.update_type, rec2.update_type, "advisory.update_type")
    assert_eq(rec1.version, rec2.version, "advisory.version")
    assert_eq(rec1.title, rec2.title, "advisory.title")
    assert_eq(rec1.issued_date, rec2.issued_date, "advisory.issued_date")
    assert_eq(rec1.updated_date, rec2.updated_date, "advisory.updated_date")
    assert_eq(rec1.rights, rec2.rights, "advisory.rights")
    assert_eq(rec1.release, rec2.release, "advisory.release")
    assert_eq(rec1.pushcount, rec2.pushcount, "advisory.pushcount")
    assert_eq(rec1.severity, rec2.severity, "advisory.severity")
    assert_eq(rec1.summary, rec2.summary, "advisory.summary")
    assert_eq(rec1.description, rec2.description, "advisory.description")
    assert_eq(rec1.solution, rec2.solution, "advisory.solution")
    assert_eq(rec1.message, rec2.message, "advisory.message")
    assert_eq(rec1.reboot_suggested, rec2.reboot_suggested, "advisory.reboot_suggested")
    compare_lists(
        rec1.references,
        rec2.references,
        "advisory.references",
        _compare_update_references_fields,
    )
    compare_lists(
        rec1.pkglist,
        rec2.pkglist,
        "advisory.pkglist",
        _compare_update_collections_fields,
    )


def compare_update_records(rec1, rec2):
    if rec1 == rec2:
        return
    _compare_update_records_fields(rec1, rec2)


def compare_advisory_lists(recs1, recs2):
    compare_lists(recs1, recs2, "advisories", _compare_update_records_fields)


# ---------------------------------------------------------------------------
# Comps
# ---------------------------------------------------------------------------


# Every comps type exposes canonicalize() (deterministic ordering of nested
# collections and localized strings) and to_dict() (a plain, comparable form).
# Comparing canonicalized dicts gives a single readable diff on failure and
# tracks the field set automatically, with no per-field boilerplate to maintain.
# Element ordering is not semantically significant and is not preserved across a
# write/read round-trip, so canonicalizing both sides is exactly the right
# comparison here. Per-field accessor coverage lives in test_comps.py.


def _canon_dict(obj):
    """Canonicalize a comps object (if supported) and return its dict form."""
    if hasattr(obj, "canonicalize"):
        obj.canonicalize()
    return obj.to_dict()


def _sorted_canon(items, key):
    """Return canonicalized dicts for a comps collection, sorted by ``key``."""
    return sorted((_canon_dict(item) for item in items), key=lambda d: d[key])


def compare_comps_groups(g1, g2):
    assert _canon_dict(g1) == _canon_dict(g2)


def compare_comps_categories(c1, c2):
    assert _canon_dict(c1) == _canon_dict(c2)


def compare_comps_environments(e1, e2):
    assert _canon_dict(e1) == _canon_dict(e2)


def compare_comps_langpacks(l1, l2):
    assert _canon_dict(l1) == _canon_dict(l2)


def compare_comps(comps1, comps2):
    """Compare the comps content of two Repository or CompsData objects."""
    assert _sorted_canon(comps1.groups, "id") == _sorted_canon(comps2.groups, "id"), "groups"
    assert _sorted_canon(comps1.categories, "id") == _sorted_canon(comps2.categories, "id"), "categories"
    assert (
        _sorted_canon(comps1.environments, "id") == _sorted_canon(comps2.environments, "id")
    ), "environments"
    assert (
        _sorted_canon(comps1.langpacks, "name") == _sorted_canon(comps2.langpacks, "name")
    ), "langpacks"
