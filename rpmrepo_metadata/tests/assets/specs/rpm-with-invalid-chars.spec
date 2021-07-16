Name:      rpm-with-invalid-chars
Version:   1
Release:   1%{?dist}
Summary:   An RPM file with invalid characters in its description.
License:   Public Domain
URL:       https://github.com/dralley/rpmrepo_rs/
BuildArch: noarch

%description
This RPM that contains XML-illegal characters such as ampersand & and less-than < greater-than > in its </description>.
These must be escaped in the final XML metadata. The XML spec does not strictly require escaping 'single' or "double" quotes
within text content, and not all XML libraries do so. However, it is generally recommended.

%prep

%build

%install

%files

%changelog

