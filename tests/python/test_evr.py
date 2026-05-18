import rpmrepo_metadata as r


class TestEVR:
    def test_new(self):
        evr = r.EVR("1", "2.3.4", "5.el9")
        assert evr.epoch == "1"
        assert evr.version == "2.3.4"
        assert evr.release == "5.el9"

    def test_components(self):
        evr = r.EVR("0", "1.0", "1.fc39")
        assert evr.components() == ("0", "1.0", "1.fc39")

    def test_parse(self):
        evr = r.EVR.parse("1:2.3.4-5.el9")
        assert evr.epoch == "1"
        assert evr.version == "2.3.4"
        assert evr.release == "5.el9"

    def test_parse_no_epoch(self):
        evr = r.EVR.parse("2.3.4-5")
        assert evr.epoch == ""
        assert evr.version == "2.3.4"
        assert evr.release == "5"

    def test_str(self):
        evr = r.EVR("1", "2.3", "4")
        assert "EVR" in str(evr)

    def test_repr(self):
        evr = r.EVR("0", "1.0", "1")
        assert "EVR" in repr(evr)

    def test_comparison_equal(self):
        assert r.EVR("0", "1.0", "1") == r.EVR("", "1.0", "1")

    def test_comparison_less(self):
        assert r.EVR("", "1.0", "1") < r.EVR("1", "1.0", "1")

    def test_comparison_greater(self):
        assert r.EVR("", "1.1", "1") > r.EVR("", "1.0", "1")
