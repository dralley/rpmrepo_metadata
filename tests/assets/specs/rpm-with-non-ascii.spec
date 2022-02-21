Name:      rpm-with-non-ascii
Version:   1
Release:   1%{?dist}
Summary:   An RPM file with non-ascii characters in its metadata.
License:   Public Domain
URL:       https://github.com/dralley/rpmrepo_rs/
BuildArch: noarch

%description
This file contains unicode characters and should be encoded as UTF-8. The
following code points are all outside the "Basic Latin (ASCII)" code point
block:

* U+0080: 
* U+0100: Ā
* U+0180: ƀ
* U+0250: ɐ
* U+02B0: ʰ
* U+0041 0x0300: À
* U+0370: Ͱ

See: http://www.unicode.org/charts/
%prep

%build

%install

%files

%changelog

