import pytest

import rpmrepo_metadata as r


class TestMetadataError:
    def test_metadata_error_raised_on_bad_checksum(self):
        pkg = r.Package()
        with pytest.raises(Exception, match="not a valid checksum"):
            pkg.checksum = ("sha256", "not-a-valid-checksum")

    def test_metadata_error_type(self):
        pkg = r.Package()
        try:
            pkg.checksum = ("sha256", "bad")
        except Exception as e:
            assert type(e).__name__ == "MetadataError"
            assert isinstance(e, Exception)
