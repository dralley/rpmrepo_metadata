#!/usr/bin/bash

set -euo pipefail

### Download external repositories

# CentOS 6
rpmrepo download --only-metadata assets/external_repos/centos6 https://vault.centos.org/centos/6/os/x86_64/

# CentOS 7
rpmrepo download --only-metadata assets/external_repos/centos7 http://mirror.centos.org/centos/7/os/x86_64/

# CentOS Stream 8 - BaseOS
rpmrepo download --only-metadata assets/external_repos/cs8-baseos http://mirror.centos.org/centos/8-stream/BaseOS/x86_64/os/

# CentOS Stream 8 - Appstream
rpmrepo download --only-metadata assets/external_repos/cs8-appstream http://mirror.centos.org/centos/8-stream/AppStream/x86_64/os/

# CentOS Stream 9 - BaseOS
rpmrepo download --only-metadata external_repos/cs9-baseos http://mirror.stream.centos.org/9-stream/BaseOS/x86_64/os/

# CentOS Stream 9 - BaseOS - aarch64
rpmrepo download --only-metadata external_repos/cs9-baseos-aarch64 http://mirror.stream.centos.org/9-stream/BaseOS/aarch64/os/

# CentOS Stream 9 - BaseOS - ppc64le
rpmrepo download --only-metadata external_repos/cs9-baseos-ppc64le http://mirror.stream.centos.org/9-stream/BaseOS/ppc64le/os/

# CentOS Stream 9 - BaseOS - s390x
rpmrepo download --only-metadata external_repos/cs9-baseos-s390x http://mirror.stream.centos.org/9-stream/BaseOS/s390x/os/

# CentOS Stream 9 - BaseOS - source
rpmrepo download --only-metadata external_repos/cs9-baseos-src http://mirror.stream.centos.org/9-stream/BaseOS/source/tree/

# CentOS Stream 9 - Appstream
rpmrepo download --only-metadata external_repos/cs9-appstream http://mirror.stream.centos.org/9-stream/AppStream/x86_64/os/

# CentOS Stream 10 - BaseOS - source
rpmrepo download --only-metadata external_repos/cs10-baseos-src http://mirror.stream.centos.org/10-stream/BaseOS/source/tree/

# CentOS Stream 10 - Appstream
rpmrepo download --only-metadata external_repos/cs10-appstream http://mirror.stream.centos.org/10-stream/BaseOS/x86_64/os/

# CentOS Stream 10 - Appstream
rpmrepo download --only-metadata external_repos/cs10-appstream http://mirror.stream.centos.org/10-stream/AppStream/x86_64/os/

# Fedora 42
rpmrepo download --only-metadata external_repos/fedora42 https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Everything/x86_64/os/

# Fedora 42 Updates
rpmrepo download --only-metadata external_repos/fedora42-updates https://dl.fedoraproject.org/pub/fedora/linux/updates/42/Everything/x86_64/

# EPEL 7
rpmrepo download --only-metadata external_repos/epel7 https://download.fedoraproject.org/pub/epel/7/x86_64/

# EPEL 8 - Everything
rpmrepo download --only-metadata external_repos/epel8 https://download.fedoraproject.org/pub/epel/8/Everything/x86_64/

# EPEL 9 - Everything
rpmrepo download --only-metadata external_repos/epel9 https://download.fedoraproject.org/pub/epel/9/Everything/x86_64/

# EPEL 9 - Modular
rpmrepo download --only-metadata external_repos/epel9-modular https://download.fedoraproject.org/pub/epel/9/Modular/x86_64/

# RPMFusion - Fedora 35
rpmrepo download --only-metadata external_repos/rpmfusion-f36 https://download1.rpmfusion.org/free/fedora/releases/36/Everything/x86_64/os/

# OpenSUSE Tumbleweed
rpmrepo download --only-metadata external_repos/opensuse-tumbleweed https://download.opensuse.org/tumbleweed/repo/oss/

# Alma Linux 8 - BaseOS
rpmrepo download --only-metadata external_repos/alma8-baseos https://repo.almalinux.org/almalinux/8/BaseOS/x86_64/os/

# Alma Linux 8 - Appstream
rpmrepo download --only-metadata external_repos/alma8-appstream https://repo.almalinux.org/almalinux/8/AppStream/x86_64/os/

# Oracle Linux 7
rpmrepo download --only-metadata external_repos/ol7 https://yum.oracle.com/repo/OracleLinux/OL7/latest/x86_64/

# Microsoft Azure RHEL8 additions
rpmrepo download --only-metadata external_repos/ms-rhel8-additions https://packages.microsoft.com/rhel/8/prod/

# Nvidia CUDA tools
rpmrepo download --only-metadata external_repos/nvidia-cuda-el8 https://developer.download.nvidia.com/compute/cuda/repos/rhel8/x86_64/

# Convert2RHEL
rpmrepo download --only-metadata external_repos/convert2rhel https://ftp.redhat.com/redhat/convert2rhel/7/os/

# Harbottle - has an externally-pointing location base
# rpmrepo download --only-metadata external_repos/harbottle-location-base http://harbottle.gitlab.io/harbottle-main/8-stream/x86_64/

# Rundeck - has a backwards-pointing location href and multiple checksums in repomd.xml
rpmrepo download --only-metadata external_repos/rundeck-location-href https://packages.rundeck.com/pagerduty/rundeck/rpm_any/rpm_any/x86_64/

# Puppet 7 - sha checksum
rpmrepo download --only-metadata external_repos/puppetlabs-puppet7-el8 https://yum.puppetlabs.com/puppet7/el/8/x86_64/
