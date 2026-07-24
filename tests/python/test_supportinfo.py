import os

import pytest

import rpmrepo_metadata as r

from conftest import FIXTURES_DIR

SUPPORTINFO_FIXTURES = os.path.join(
    FIXTURES_DIR, "supportinfo_fixtures"
)

EMPTY_V1_XML = """\
<?xml version="1.0" encoding="UTF-8"?>
<package_support schema_version="1.0" current_as="2024-01-01T00:00:00">
  <lifecycles/>
  <support_milestones/>
  <support_levels/>
  <packages/>
  <package_classes/>
  <package_origins/>
  <notes/>
</package_support>
"""

COMPLEX_V1_XML = """\
<?xml version="1.0" encoding="UTF-8"?>
<package_support schema_version="1.0" current_as="2024-01-17T00:00:00">
  <lifecycles>
    <lifecycle name="default_lc" display_name="Default Lifecycle" description="Main support timeline">
      <phase name="supported" support_level="standard" start_milestone="ga"/>
      <phase name="unsupported" support_level="eos" start_milestone="eol"/>
    </lifecycle>
  </lifecycles>
  <support_milestones>
    <milestone name="ga" date="2023-03-15" display_name="GA" description="General availability"/>
    <milestone name="eol" date="2028-03-15" display_name="EOL" description="End of life"/>
  </support_milestones>
  <support_levels>
    <support_level name="standard" severities="Low,Medium,Important,Critical" description="Full support"/>
    <support_level name="eos" severities="" description="End of support"/>
  </support_levels>
  <packages>
    <package name="test-glibc" lifecycle="default_lc" package_class="core" origin="mylinux"/>
  </packages>
  <package_classes>
    <package_class name="core">
      <summary>Core packages</summary>
      <text>Essential system packages</text>
    </package_class>
  </package_classes>
  <package_origins>
    <package_origin name="mylinux" repo_id="mylinux" dist="ml2023" vendor="MyVendor"/>
  </package_origins>
  <notes>
    <note name="note1">A note about support</note>
  </notes>
</package_support>
"""

class TestSupportInfoTypes:
    def test_v1_data_type_exists(self):
        assert hasattr(r, "SupportInfoV1Data")

    def test_lifecycle_type_exists(self):
        assert hasattr(r, "SupportInfoLifecycle")

    def test_phase_type_exists(self):
        assert hasattr(r, "SupportInfoPhase")

    def test_milestone_type_exists(self):
        assert hasattr(r, "SupportInfoMilestone")

    def test_level_type_exists(self):
        assert hasattr(r, "SupportInfoLevel")

    def test_package_type_exists(self):
        assert hasattr(r, "SupportInfoPackage")

    def test_package_class_type_exists(self):
        assert hasattr(r, "SupportInfoPackageClass")

    def test_package_origin_type_exists(self):
        assert hasattr(r, "SupportInfoPackageOrigin")

    def test_note_type_exists(self):
        assert hasattr(r, "SupportInfoNote")

    def test_parse_support_info_function_exists(self):
        assert hasattr(r, "parse_support_info")


class TestSupportInfoV1Construction:
    def test_create_empty(self):
        data = r.SupportInfoV1Data(current_as="2024-01-01T00:00:00")
        assert data.current_as == "2024-01-01T00:00:00"
        assert data.lifecycles == []
        assert data.milestones == []
        assert data.support_levels == []
        assert data.packages == []
        assert data.package_classes == []
        assert data.package_origins == []
        assert data.notes == []

    def test_create_lifecycle(self):
        lc = r.SupportInfoLifecycle(
            name="default_lc",
            display_name="Default Lifecycle",
            description="Main support timeline",
        )
        assert lc.name == "default_lc"
        assert lc.display_name == "Default Lifecycle"
        assert lc.description == "Main support timeline"
        assert lc.note is None
        assert lc.phases == []

    def test_create_phase(self):
        phase = r.SupportInfoPhase(
            name="supported",
            support_level="standard",
            start_milestone="ga",
        )
        assert phase.name == "supported"
        assert phase.support_level == "standard"
        assert phase.start_milestone == "ga"
        assert phase.start_date is None

    def test_create_milestone(self):
        ms = r.SupportInfoMilestone(
            name="ga",
            date="2023-03-15",
            display_name="GA",
            description="General availability",
        )
        assert ms.name == "ga"
        assert ms.date == "2023-03-15"
        assert ms.display_name == "GA"
        assert ms.description == "General availability"

    def test_create_level(self):
        level = r.SupportInfoLevel(
            name="standard",
            severities="Low,Medium,Important,Critical",
            description="Full support",
        )
        assert level.name == "standard"
        assert level.severities == "Low,Medium,Important,Critical"
        assert level.description == "Full support"

    def test_create_package(self):
        pkg = r.SupportInfoPackage(
            name="test-glibc",
            lifecycle="default_lc",
            origin="mylinux",
            package_class="core",
        )
        assert pkg.name == "test-glibc"
        assert pkg.lifecycle == "default_lc"
        assert pkg.origin == "mylinux"
        assert pkg.package_class == "core"

    def test_create_package_class(self):
        cls = r.SupportInfoPackageClass(
            name="core",
            summary="Core packages",
            text="Essential system packages",
        )
        assert cls.name == "core"
        assert cls.summary == "Core packages"
        assert cls.text == "Essential system packages"

    def test_create_package_origin(self):
        origin = r.SupportInfoPackageOrigin(
            name="mylinux",
            repo_id="mylinux",
            dist="ml2023",
            vendor="MyVendor",
        )
        assert origin.name == "mylinux"
        assert origin.repo_id == "mylinux"
        assert origin.dist == "ml2023"
        assert origin.vendor == "MyVendor"
        assert origin.signing_key is None

    def test_create_note(self):
        note = r.SupportInfoNote(name="note1", content="A note about support")
        assert note.name == "note1"
        assert note.content == "A note about support"

    def test_set_lifecycle_phases(self):
        lc = r.SupportInfoLifecycle(name="test_lc")
        phase1 = r.SupportInfoPhase(
            name="supported", support_level="standard", start_milestone="ga"
        )
        phase2 = r.SupportInfoPhase(
            name="unsupported", support_level="eos", start_date="2028-03-15"
        )
        lc.phases = [phase1, phase2]
        assert len(lc.phases) == 2
        assert lc.phases[0].name == "supported"
        assert lc.phases[1].start_date == "2028-03-15"

    def test_set_data_collections(self):
        data = r.SupportInfoV1Data(current_as="2024-01-01T00:00:00")

        lc = r.SupportInfoLifecycle(name="lc1")
        data.lifecycles = [lc]
        assert len(data.lifecycles) == 1

        ms = r.SupportInfoMilestone(name="ga", date="2024-01-01")
        data.milestones = [ms]
        assert len(data.milestones) == 1

        level = r.SupportInfoLevel(name="standard", severities="Low,Medium")
        data.support_levels = [level]
        assert len(data.support_levels) == 1

        pkg = r.SupportInfoPackage(
            name="foo", lifecycle="lc1", origin="mylinux"
        )
        data.packages = [pkg]
        assert len(data.packages) == 1

        cls = r.SupportInfoPackageClass(
            name="core", summary="Core", text="Essential"
        )
        data.package_classes = [cls]
        assert len(data.package_classes) == 1

        origin = r.SupportInfoPackageOrigin(
            name="mylinux", repo_id="mylinux", dist="ml", vendor="V"
        )
        data.package_origins = [origin]
        assert len(data.package_origins) == 1

        note = r.SupportInfoNote(name="n1", content="Note")
        data.notes = [note]
        assert len(data.notes) == 1


class TestSupportInfoV1Parsing:
    def test_from_xml_empty(self):
        data = r.SupportInfoV1Data.from_xml(EMPTY_V1_XML)
        assert data.current_as == "2024-01-01T00:00:00"
        assert data.lifecycles == []
        assert data.milestones == []
        assert data.support_levels == []
        assert data.packages == []

    def test_from_xml_complex(self):
        data = r.SupportInfoV1Data.from_xml(COMPLEX_V1_XML)
        assert data.current_as == "2024-01-17T00:00:00"

        assert len(data.lifecycles) == 1
        lc = data.lifecycles[0]
        assert lc.name == "default_lc"
        assert lc.display_name == "Default Lifecycle"
        assert len(lc.phases) == 2
        assert lc.phases[0].name == "supported"
        assert lc.phases[0].support_level == "standard"
        assert lc.phases[0].start_milestone == "ga"
        assert lc.phases[1].name == "unsupported"
        assert lc.phases[1].start_milestone == "eol"

        assert len(data.milestones) == 2
        assert data.milestones[0].name == "ga"
        assert data.milestones[0].date == "2023-03-15"
        assert data.milestones[1].name == "eol"

        assert len(data.support_levels) == 2
        assert data.support_levels[0].name == "standard"
        assert data.support_levels[0].severities == "Low,Medium,Important,Critical"
        assert data.support_levels[1].name == "eos"
        assert data.support_levels[1].severities == ""

        assert len(data.packages) == 1
        assert data.packages[0].name == "test-glibc"
        assert data.packages[0].lifecycle == "default_lc"
        assert data.packages[0].package_class == "core"
        assert data.packages[0].origin == "mylinux"

        assert len(data.package_classes) == 1
        assert data.package_classes[0].name == "core"
        assert data.package_classes[0].summary == "Core packages"
        assert data.package_classes[0].text == "Essential system packages"

        assert len(data.package_origins) == 1
        assert data.package_origins[0].name == "mylinux"
        assert data.package_origins[0].vendor == "MyVendor"

        assert len(data.notes) == 1
        assert data.notes[0].name == "note1"
        assert data.notes[0].content == "A note about support"


class TestParseSupportInfoAutoDetect:
    def test_detects_v1(self):
        result = r.parse_support_info(COMPLEX_V1_XML)
        assert isinstance(result, r.SupportInfoV1Data)
        assert result.current_as == "2024-01-17T00:00:00"

class TestSupportInfoV1Roundtrip:
    def test_roundtrip(self):
        data = r.SupportInfoV1Data.from_xml(COMPLEX_V1_XML)
        xml_out = data.to_xml()
        data2 = r.SupportInfoV1Data.from_xml(xml_out)
        assert data == data2

    def test_construct_and_roundtrip(self):
        data = r.SupportInfoV1Data(current_as="2024-06-01T00:00:00")

        phase = r.SupportInfoPhase(
            name="supported", support_level="full", start_milestone="ga"
        )
        lc = r.SupportInfoLifecycle(
            name="my_lc", display_name="My Lifecycle"
        )
        lc.phases = [phase]
        data.lifecycles = [lc]

        ms = r.SupportInfoMilestone(name="ga", date="2024-01-01")
        data.milestones = [ms]

        level = r.SupportInfoLevel(
            name="full", severities="Low,Medium,High"
        )
        data.support_levels = [level]

        pkg = r.SupportInfoPackage(
            name="mypackage", lifecycle="my_lc", origin="myrepo"
        )
        data.packages = [pkg]

        origin = r.SupportInfoPackageOrigin(
            name="myrepo", repo_id="myrepo", dist="fc40", vendor="Fedora"
        )
        data.package_origins = [origin]

        note = r.SupportInfoNote(name="n1", content="A note")
        data.notes = [note]

        xml_out = data.to_xml()
        data2 = r.SupportInfoV1Data.from_xml(xml_out)
        assert data2.current_as == "2024-06-01T00:00:00"
        assert len(data2.lifecycles) == 1
        assert data2.lifecycles[0].name == "my_lc"
        assert len(data2.lifecycles[0].phases) == 1
        assert data2.milestones[0].name == "ga"
        assert data2.support_levels[0].severities == "Low,Medium,High"
        assert data2.packages[0].name == "mypackage"
        assert data2.package_origins[0].vendor == "Fedora"
        assert data2.notes[0].content == "A note"

class TestSupportInfoFixtures:
    def test_parse_v1_fixture(self):
        fixture = os.path.join(
            SUPPORTINFO_FIXTURES, "test_support_info_v1.xml"
        )
        if not os.path.isfile(fixture):
            pytest.skip("V1 fixture file not available")
        with open(fixture, "r") as f:
            xml = f.read()
        result = r.parse_support_info(xml)
        assert isinstance(result, r.SupportInfoV1Data)
        assert len(result.lifecycles) > 0
        assert len(result.packages) > 0

    def test_v1_data_str(self):
        data = r.SupportInfoV1Data(current_as="2024-01-01T00:00:00")
        assert "SupportInfoV1Data" in str(data)
        assert "2024-01-01" in str(data)

    def test_lifecycle_str(self):
        lc = r.SupportInfoLifecycle(name="my_lc")
        assert "my_lc" in str(lc)
