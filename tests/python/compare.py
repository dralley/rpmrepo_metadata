"""Shared comparison routines for rpmrepo_metadata objects.

Each function first tries a fast __eq__ check via the Rust PartialEq derive.
On mismatch it falls through to field-by-field assertions so failures pinpoint
exactly which field differs.
"""


def assert_eq(a, b, label):
    assert a == b, f"{label}: {a!r} != {b!r}"


def compare_lists(list1, list2, label, item_compare_fn):
    assert len(list1) == len(list2), (
        f"{label}: length {len(list1)} != {len(list2)}"
    )
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
        c1.packages, c2.packages,
        "collection.packages", _compare_update_collection_packages_fields,
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
    compare_lists(
        rec1.references, rec2.references,
        "advisory.references", _compare_update_references_fields,
    )
    compare_lists(
        rec1.pkglist, rec2.pkglist,
        "advisory.pkglist", _compare_update_collections_fields,
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

def _compare_comps_package_reqs_fields(p1, p2):
    assert_eq(p1.name, p2.name, "package_req.name")
    assert_eq(p1.reqtype, p2.reqtype, "package_req.reqtype")
    assert_eq(p1.requires, p2.requires, "package_req.requires")
    assert_eq(p1.basearchonly, p2.basearchonly, "package_req.basearchonly")


def _compare_comps_groups_fields(g1, g2):
    assert_eq(g1.id, g2.id, "group.id")
    assert_eq(g1.name, g2.name, "group.name")
    assert_eq(g1.description, g2.description, "group.description")
    assert_eq(g1.default, g2.default, "group.default")
    assert_eq(g1.uservisible, g2.uservisible, "group.uservisible")
    assert_eq(g1.biarchonly, g2.biarchonly, "group.biarchonly")
    assert_eq(g1.langonly, g2.langonly, "group.langonly")
    assert_eq(g1.display_order, g2.display_order, "group.display_order")
    assert_eq(g1.name_by_lang, g2.name_by_lang, "group.name_by_lang")
    assert_eq(g1.desc_by_lang, g2.desc_by_lang, "group.desc_by_lang")
    compare_lists(
        g1.packages, g2.packages,
        "group.packages", _compare_comps_package_reqs_fields,
    )


def _compare_comps_categories_fields(c1, c2):
    assert_eq(c1.id, c2.id, "category.id")
    assert_eq(c1.name, c2.name, "category.name")
    assert_eq(c1.description, c2.description, "category.description")
    assert_eq(c1.display_order, c2.display_order, "category.display_order")
    assert_eq(c1.name_by_lang, c2.name_by_lang, "category.name_by_lang")
    assert_eq(c1.desc_by_lang, c2.desc_by_lang, "category.desc_by_lang")
    assert_eq(c1.group_ids, c2.group_ids, "category.group_ids")


def _compare_comps_environment_options_fields(o1, o2):
    assert_eq(o1.group_id, o2.group_id, "env_option.group_id")
    assert_eq(o1.default, o2.default, "env_option.default")


def _compare_comps_environments_fields(e1, e2):
    assert_eq(e1.id, e2.id, "environment.id")
    assert_eq(e1.name, e2.name, "environment.name")
    assert_eq(e1.description, e2.description, "environment.description")
    assert_eq(e1.display_order, e2.display_order, "environment.display_order")
    assert_eq(e1.name_by_lang, e2.name_by_lang, "environment.name_by_lang")
    assert_eq(e1.desc_by_lang, e2.desc_by_lang, "environment.desc_by_lang")
    assert_eq(e1.group_ids, e2.group_ids, "environment.group_ids")
    compare_lists(
        e1.option_ids, e2.option_ids,
        "environment.option_ids", _compare_comps_environment_options_fields,
    )


def _compare_comps_langpacks_fields(l1, l2):
    assert_eq(l1.name, l2.name, "langpack.name")
    assert_eq(l1.install, l2.install, "langpack.install")


def compare_comps_groups(g1, g2):
    if g1 == g2:
        return
    _compare_comps_groups_fields(g1, g2)


def compare_comps_categories(c1, c2):
    if c1 == c2:
        return
    _compare_comps_categories_fields(c1, c2)


def compare_comps_environments(e1, e2):
    if e1 == e2:
        return
    _compare_comps_environments_fields(e1, e2)


def compare_comps_langpacks(l1, l2):
    if l1 == l2:
        return
    _compare_comps_langpacks_fields(l1, l2)


def compare_comps(repo1, repo2):
    compare_lists(repo1.groups, repo2.groups, "groups", _compare_comps_groups_fields)
    compare_lists(repo1.categories, repo2.categories, "categories", _compare_comps_categories_fields)
    compare_lists(repo1.environments, repo2.environments, "environments", _compare_comps_environments_fields)
    compare_lists(repo1.langpacks, repo2.langpacks, "langpacks", _compare_comps_langpacks_fields)
