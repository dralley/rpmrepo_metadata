#!/usr/bin/bash

set -euo pipefail

### Download external repositories

# CentOS Stream 9 - BaseOS
rpmrepo download --only-metadata external_repos/cs9-baseos http://mirror.stream.centos.org/9-stream/BaseOS/x86_64/os/

# # CentOS Stream 9 - BaseOS - aarch64
# rpmrepo download --only-metadata external_repos/cs9-baseos-aarch64 http://mirror.stream.centos.org/9-stream/BaseOS/aarch64/os/

# # CentOS Stream 9 - BaseOS - ppc64le
# rpmrepo download --only-metadata external_repos/cs9-baseos-ppc64le http://mirror.stream.centos.org/9-stream/BaseOS/ppc64le/os/

# # CentOS Stream 9 - BaseOS - s390x
# rpmrepo download --only-metadata external_repos/cs9-baseos-s390x http://mirror.stream.centos.org/9-stream/BaseOS/s390x/os/

# # CentOS Stream 9 - BaseOS - source
# rpmrepo download --only-metadata external_repos/cs9-baseos-src http://mirror.stream.centos.org/9-stream/BaseOS/source/tree/

# CentOS Stream 9 - Appstream
rpmrepo download --only-metadata external_repos/cs9-appstream http://mirror.stream.centos.org/9-stream/AppStream/x86_64/os/

# Fedora 35
rpmrepo download --only-metadata external_repos/fedora35 https://dl.fedoraproject.org/pub/fedora/linux/releases/35/Everything/x86_64/os/

# Fedora 35 Updates
rpmrepo download --only-metadata external_repos/fedora35-updates https://dl.fedoraproject.org/pub/fedora/linux/updates/35/Everything/x86_64/

# Fedora 35 Modular Updates
rpmrepo download --only-metadata external_repos/fedora35-modular-updates https://dl.fedoraproject.org/pub/fedora/linux/updates/35/Modular/x86_64/

# OpenSUSE Tumbleweed
rpmrepo download --only-metadata external_repos/opensuse-tumbleweed https://download.opensuse.org/tumbleweed/repo/oss/

# # Alma Linux 8 - BaseOS
# rpmrepo download --only-metadata external_repos/alma8-baseos https://repo.almalinux.org/almalinux/8/BaseOS/x86_64/os/

# # Alma Linux 8 - Appstream
# rpmrepo download --only-metadata external_repos/alma8-appstream https://repo.almalinux.org/almalinux/8/AppStream/x86_64/os/

# # Oracle Linux 7
# rpmrepo download --only-metadata external_repos/ol7 https://yum.oracle.com/repo/OracleLinux/OL7/latest/x86_64/

# # CentOS 6
# rpmrepo download --only-metadata external_repos/centos6 https://vault.centos.org/centos/6/os/x86_64/

# # RPMFusion - Fedora 35
# rpmrepo download --only-metadata external_repos/rpmfusion-f35 https://download1.rpmfusion.org/free/fedora/releases/35/Everything/x86_64/os/

