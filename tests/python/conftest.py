import os

import pytest

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "..", "assets")
COMPLEX_REPO = os.path.join(FIXTURES_DIR, "fixture_repos", "complex_repo")
EMPTY_REPO = os.path.join(FIXTURES_DIR, "fixture_repos", "empty_repo")
COMPS_FIXTURE = os.path.join(FIXTURES_DIR, "comps_fixture.xml")
