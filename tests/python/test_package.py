import os

import pytest
from rpm_rs import PackageBuilder

import rpmrepo_metadata as r

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "..", "assets")
RPM_FIXTURE = os.path.join(FIXTURES_DIR, "packages", "complex-package-2.3.4-5.el8.x86_64.rpm")
RPM_EMPTY = os.path.join(FIXTURES_DIR, "packages", "rpm-empty-0-0.x86_64.rpm")


class TestPackage:
    def test_new_default(self):
        pkg = r.Package()
        assert pkg.name == ""
        assert pkg.version == ""
        assert pkg.release == ""
        assert pkg.arch == ""

    def test_set_name(self):
        pkg = r.Package()
        pkg.name = "foo"
        assert pkg.name == "foo"

    def test_set_epoch(self):
        pkg = r.Package()
        pkg.epoch = 1
        assert pkg.epoch == 1

    def test_set_version(self):
        pkg = r.Package()
        pkg.version = "1.2.3"
        assert pkg.version == "1.2.3"

    def test_set_release(self):
        pkg = r.Package()
        pkg.release = "4.el9"
        assert pkg.release == "4.el9"

    def test_set_arch(self):
        pkg = r.Package()
        pkg.arch = "x86_64"
        assert pkg.arch == "x86_64"

    def test_evr(self):
        pkg = r.Package()
        pkg.epoch = 1
        pkg.version = "2.3"
        pkg.release = "4"
        evr = pkg.as_evr()
        assert evr.epoch == "1"
        assert evr.version == "2.3"
        assert evr.release == "4"

    def test_checksum(self):
        pkg = r.Package()
        digest = "a" * 64
        pkg.checksum = ("sha256", digest)
        assert pkg.checksum == ("sha256", digest)
        assert pkg.checksum_type == "sha256"
        assert pkg.pkgid == digest

    def test_location(self):
        pkg = r.Package()
        pkg.location_href = "foo-1.0-1.rpm"
        assert pkg.location_href == "foo-1.0-1.rpm"
        pkg.location_base = "http://example.com"
        assert pkg.location_base == "http://example.com"
        pkg.location_base = None
        assert pkg.location_base is None

    def test_summary_description(self):
        pkg = r.Package()
        pkg.summary = "A summary"
        pkg.description = "A description"
        assert pkg.summary == "A summary"
        assert pkg.description == "A description"

    def test_packager_url(self):
        pkg = r.Package()
        pkg.packager = "Test User"
        pkg.url = "http://example.com"
        assert pkg.packager == "Test User"
        assert pkg.url == "http://example.com"

    def test_times(self):
        pkg = r.Package()
        pkg.time_file = 1000
        pkg.time_build = 2000
        assert pkg.time_file == 1000
        assert pkg.time_build == 2000

    def test_sizes(self):
        pkg = r.Package()
        pkg.size_package = 100
        pkg.size_installed = 200
        pkg.size_archive = 300
        assert pkg.size_package == 100
        assert pkg.size_installed == 200
        assert pkg.size_archive == 300

    def test_rpm_metadata(self):
        pkg = r.Package()
        pkg.rpm_license = "MIT"
        pkg.rpm_vendor = "Test"
        pkg.rpm_group = "Development"
        pkg.rpm_buildhost = "builder.example.com"
        pkg.rpm_sourcerpm = "foo-1.0-1.src.rpm"
        assert pkg.rpm_license == "MIT"
        assert pkg.rpm_vendor == "Test"
        assert pkg.rpm_group == "Development"
        assert pkg.rpm_buildhost == "builder.example.com"
        assert pkg.rpm_sourcerpm == "foo-1.0-1.src.rpm"

    def test_header_range(self):
        pkg = r.Package()
        pkg.rpm_header_range = (100, 200)
        assert pkg.rpm_header_range == (100, 200)

    def test_requires(self):
        pkg = r.Package()
        reqs = [("libc.so.6", "EQ", "0", "2.17", "", False)]
        pkg.requires = reqs
        assert len(pkg.requires) == 1
        assert pkg.requires[0][0] == "libc.so.6"

    def test_provides(self):
        pkg = r.Package()
        pkg.provides = [("foo", None, None, None, None, False)]
        assert len(pkg.provides) == 1

    def test_conflicts(self):
        pkg = r.Package()
        pkg.conflicts = [("bar", None, None, None, None, False)]
        assert len(pkg.conflicts) == 1

    def test_obsoletes(self):
        pkg = r.Package()
        pkg.obsoletes = [("old-foo", None, None, None, None, False)]
        assert len(pkg.obsoletes) == 1

    def test_suggests(self):
        pkg = r.Package()
        pkg.suggests = [("optional-dep", None, None, None, None, False)]
        assert len(pkg.suggests) == 1

    def test_enhances(self):
        pkg = r.Package()
        pkg.enhances = [("enh", None, None, None, None, False)]
        assert len(pkg.enhances) == 1

    def test_recommends(self):
        pkg = r.Package()
        pkg.recommends = [("rec", None, None, None, None, False)]
        assert len(pkg.recommends) == 1

    def test_supplements(self):
        pkg = r.Package()
        pkg.supplements = [("sup", None, None, None, None, False)]
        assert len(pkg.supplements) == 1

    def test_files(self):
        pkg = r.Package()
        pkg.files = [(None, "/usr/bin/foo"), ("dir", "/usr/share/foo")]
        assert len(pkg.files) == 2
        assert pkg.files[0] == (None, "/usr/bin/foo")
        assert pkg.files[1] == ("dir", "/usr/share/foo")

    def test_files_split(self):
        pkg = r.Package()
        pkg.files = [(None, "/usr/bin/foo")]
        split = pkg.files_split
        assert len(split) == 1
        assert split[0][2] == "foo"  # filename portion

    def test_files_split_interns_directories(self):
        pkg = r.Package()
        pkg.files = [(None, "/usr/bin/foo"), (None, "/usr/bin/bar")]
        split = pkg.files_split
        assert split[0][1] is split[1][1]  # shared directory object

    def test_files_split_interns_empty_directories(self):
        pkg = r.Package()
        pkg.files = [(None, "foo"), (None, "bar")]
        split = pkg.files_split
        assert split[0][1] == ""
        assert split[1][1] == ""
        assert split[0][1] is split[1][1]  # shared empty directory object

    def test_files_invalid_type(self):
        pkg = r.Package()
        with pytest.raises(ValueError):
            pkg.files = [("badtype", "/usr/bin/foo")]

    def test_changelogs(self):
        pkg = r.Package()
        pkg.changelogs = [("Author Name", 1000000, "- Fixed a bug")]
        assert len(pkg.changelogs) == 1
        assert pkg.changelogs[0] == ("Author Name", 1000000, "- Fixed a bug")

    def test_str_repr(self):
        pkg = r.Package()
        pkg.name = "foo"
        pkg.version = "1.0"
        pkg.release = "1"
        pkg.arch = "x86_64"
        s = str(pkg)
        assert "Package" in s

    def test_nevra_methods(self):
        pkg = r.Package()
        pkg.name = "foo"
        pkg.epoch = 1
        pkg.version = "2.0"
        pkg.release = "3.el9"
        pkg.arch = "x86_64"
        assert "foo" in pkg.nevra()
        assert "foo" in pkg.nvra()


class TestPackageFromHeader:
    def test_from_file(self):
        pkg = r.Package.from_file(RPM_FIXTURE)
        assert pkg.name == "complex-package"
        assert pkg.version == "2.3.4"
        assert pkg.release == "5.el8"
        assert pkg.arch == "x86_64"

    def test_from_buffer(self):
        """Parses RPM headers with caller-provided file metadata."""
        expected = r.Package.from_file(RPM_FIXTURE)
        with open(RPM_FIXTURE, "rb") as rpm_file:
            data = rpm_file.read()[: expected.rpm_header_range[1]]

        parsed = r.Package.from_buffer(
            data,
            time_file=expected.time_file,
            location_href=expected.location_href,
            checksum=expected.checksum,
            size_package=expected.size_package,
            location_base=expected.location_base,
        )

        assert parsed == expected

    def test_package_from_buffer_derives_file_values(self):
        """Derives SHA-256 and package size from a complete RPM buffer."""
        expected = r.Package.from_file(RPM_FIXTURE)
        with open(RPM_FIXTURE, "rb") as rpm_file:
            data = rpm_file.read()

        parsed = r.Package.from_buffer(
            data,
            time_file=expected.time_file,
            location_href=expected.location_href,
            location_base=expected.location_base,
        )

        assert parsed == expected

    def test_package_from_buffer_rejects_partial_file_values(self):
        """Rejects a checksum when the matching full-file size is absent."""
        expected = r.Package.from_file(RPM_FIXTURE)
        with open(RPM_FIXTURE, "rb") as rpm_file:
            data = rpm_file.read()

        with pytest.raises(r.MetadataError, match="must be supplied together"):
            r.Package.from_buffer(
                data,
                time_file=expected.time_file,
                location_href=expected.location_href,
                checksum=expected.checksum,
            )

    def test_package_from_buffer_rejects_size_without_checksum(self):
        """Rejects a full-file size when the matching checksum is absent."""
        expected = r.Package.from_file(RPM_FIXTURE)
        with open(RPM_FIXTURE, "rb") as rpm_file:
            data = rpm_file.read()

        with pytest.raises(r.MetadataError, match="must be supplied together"):
            r.Package.from_buffer(
                data,
                time_file=expected.time_file,
                location_href=expected.location_href,
                size_package=expected.size_package,
            )

    def test_package_from_buffer_uses_configured_checksum_type(self):
        """Uses the requested checksum type when parsing a complete RPM buffer."""
        expected = r.Package.from_file(RPM_FIXTURE)
        with open(RPM_FIXTURE, "rb") as rpm_file:
            data = rpm_file.read()

        parsed = r.Package.from_buffer(
            data,
            time_file=expected.time_file,
            location_href=expected.location_href,
            checksum_type=r.ChecksumType.Sha512,
        )

        assert parsed.checksum_type == "sha512"

    # TODO: apply the same stripping to things other than changelogs
    def test_package_parse_strips_xml_forbidden_characters(self):
        """Strips XML-forbidden characters from changelogs in an rpm-rs-built RPM."""
        builder = PackageBuilder("changelog-control-chars", "1.0", "MIT", "noarch")
        builder.add_changelog_entry("AuthorXYName", "DescriptionABCDE", 1_700_000_000)
        rpm_bytes = builder.build().to_bytes()

        replacements = [
            (b"AuthorXYName", b"Author\x1b\tName"),
            (b"DescriptionABCDE", b"Description\x1f\n\xef\xbf\xbe"),
        ]

        # replace the values in the binary with "corrupted" versions
        for original, replacement in replacements:
            assert len(original) == len(replacement)
            assert rpm_bytes.count(original) == 1
            rpm_bytes = rpm_bytes.replace(original, replacement, 1)

        parsed = r.Package.from_buffer(
            rpm_bytes,
            time_file=0,
            location_href="changelog-control-chars-1.0-1.noarch.rpm",
        )

        assert parsed.changelogs == [("Author\tName", 1_700_000_000, "Description\n")]

    def test_package_parse_lossy_decodes_non_utf8_changelog_characters(self):
        """Parses non-UTF-8 data in changelogs (so long as RPMTAG_ENCODING does not claim UTF-8)"""
        builder = PackageBuilder("invalid-utf8-changelog", "1.0", "MIT", "noarch")
        builder.add_changelog_entry("AuthorXYName", "DescriptionABCDE", 1_700_000_000)
        rpm_bytes = builder.build().to_bytes()

        replacements = [
            # There is an RPMTAG_ENCODING tag which declares that the package uses UTF-8 for strings.
            # rpm-rs always declares this, and treats decoding issues as errors during parsing when it is
            # declared, so we have to corrupt that value to bypass the check for testing purposes.
            (b"utf-8\x00", b"ascii\x00"),
            (b"AuthorXYName", b"Author\xffYName"),
            (b"DescriptionABCDE", b"Description\xffBCDE"),
        ]
        for original, replacement in replacements:
            assert len(original) == len(replacement)
            assert rpm_bytes.count(original) == 1
            rpm_bytes = rpm_bytes.replace(original, replacement, 1)

        parsed = r.Package.from_buffer(
            rpm_bytes,
            time_file=0,
            location_href="invalid-utf8-changelog-1.0-1.noarch.rpm",
        )

        assert parsed.changelogs == [("Author�YName", 1_700_000_000, "Description�BCDE")]

    def test_from_file_populates_metadata(self):
        pkg = r.Package.from_file(RPM_FIXTURE)
        assert pkg.checksum_type == "sha256"
        assert len(pkg.checksum[1]) == 64
        assert pkg.size_package > 0

    def test_from_file_nonexistent(self):
        with pytest.raises(OSError):
            r.Package.from_file("/nonexistent/path.rpm")

    def test_from_file_defaults(self):
        pkg = r.Package.from_file(RPM_FIXTURE)
        assert pkg.name == "complex-package"
        assert pkg.checksum_type == "sha256"

    def test_from_file_checksum_type(self):
        pkg = r.Package.from_file(RPM_FIXTURE, checksum_type=r.ChecksumType.Sha512)
        assert pkg.checksum_type == "sha512"
        assert len(pkg.checksum[1]) == 128

    def test_from_file_location_href(self):
        pkg = r.Package.from_file(RPM_FIXTURE, location_href="custom/path.rpm")
        assert pkg.location_href == "custom/path.rpm"

    def test_from_file_location_base(self):
        pkg = r.Package.from_file(RPM_FIXTURE, location_base="http://example.com")
        assert pkg.location_base == "http://example.com"

    def test_from_file_changelog_limit(self):
        pkg_default = r.Package.from_file(RPM_FIXTURE)
        pkg_limited = r.Package.from_file(RPM_FIXTURE, changelog_limit=1)
        assert len(pkg_limited.changelogs) <= 1
        assert len(pkg_limited.changelogs) <= len(pkg_default.changelogs)

    def test_from_file_simple_rpm(self):
        pkg = r.Package.from_file(RPM_EMPTY)
        assert pkg.name == "rpm-empty"
        assert pkg.arch == "x86_64"


class TestPackageSorting:
    def _make_pkg(self, name, epoch, version, release, arch):
        pkg = r.Package()
        pkg.name = name
        pkg.epoch = epoch
        pkg.version = version
        pkg.release = release
        pkg.arch = arch
        return pkg

    def test_sort_by_evr(self):
        packages = [
            self._make_pkg("foo", 0, "3.0", "1.el9", "x86_64"),
            self._make_pkg("foo", 0, "1.0", "1.el9", "x86_64"),
            self._make_pkg("foo", 1, "1.0", "1.el9", "x86_64"),
            self._make_pkg("foo", 0, "2.0", "1.el9", "x86_64"),
            self._make_pkg("foo", 0, "1.0", "2.el9", "x86_64"),
        ]

        packages.sort(key=lambda p: p.as_evr())

        versions = [p.version for p in packages]
        assert versions == ["1.0", "1.0", "2.0", "3.0", "1.0"]

        releases = [p.release for p in packages]
        assert releases == ["1.el9", "2.el9", "1.el9", "1.el9", "1.el9"]

        epochs = [p.epoch for p in packages]
        assert epochs == [0, 0, 0, 0, 1]

    def test_sort_by_nevra(self):
        packages = [
            self._make_pkg("zlib", 0, "1.0", "1.el9", "x86_64"),
            self._make_pkg("bash", 0, "5.0", "1.el9", "x86_64"),
            self._make_pkg("bash", 0, "4.0", "1.el9", "x86_64"),
            self._make_pkg("glibc", 0, "2.0", "1.el9", "i686"),
            self._make_pkg("glibc", 0, "2.0", "1.el9", "x86_64"),
        ]

        packages.sort(key=lambda p: p.as_nevra())

        nevras = [p.nvra() for p in packages]
        assert nevras == [
            "bash-4.0-1.el9.x86_64",
            "bash-5.0-1.el9.x86_64",
            "glibc-2.0-1.el9.i686",
            "glibc-2.0-1.el9.x86_64",
            "zlib-1.0-1.el9.x86_64",
        ]


class TestChecksumType:
    def test_enum_values_exist(self):
        assert r.ChecksumType.Md5 is not None
        assert r.ChecksumType.Sha1 is not None
        assert r.ChecksumType.Sha224 is not None
        assert r.ChecksumType.Sha256 is not None
        assert r.ChecksumType.Sha384 is not None
        assert r.ChecksumType.Sha512 is not None

    def test_enum_equality(self):
        assert r.ChecksumType.Sha256 == r.ChecksumType.Sha256
        assert r.ChecksumType.Sha256 != r.ChecksumType.Sha512
