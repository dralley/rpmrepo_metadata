#!/usr/bin/bash

set -euo pipefail

DEST="${1:-external_repos}"

# Select the package download mode for this fixture set. For example:
# DOWNLOAD_OPTIONS=(--only-package-headers)
DOWNLOAD_OPTIONS=(--no-check-certificate)

### Download external repositories

# CentOS 7
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/centos7 http://vault.centos.org/centos/7/os/x86_64/

# CentOS Stream 8 - BaseOS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs8-baseos http://vault.centos.org/centos/8-stream/BaseOS/x86_64/os/

# CentOS Stream 8 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs8-appstream http://vault.centos.org/centos/8-stream/AppStream/x86_64/os/

# CentOS Stream 9 - BaseOS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-baseos http://mirror.stream.centos.org/9-stream/BaseOS/x86_64/os/

# CentOS Stream 9 - BaseOS - aarch64
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-baseos-aarch64 http://mirror.stream.centos.org/9-stream/BaseOS/aarch64/os/

# CentOS Stream 9 - BaseOS - ppc64le
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-baseos-ppc64le http://mirror.stream.centos.org/9-stream/BaseOS/ppc64le/os/

# CentOS Stream 9 - BaseOS - s390x
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-baseos-s390x http://mirror.stream.centos.org/9-stream/BaseOS/s390x/os/

# CentOS Stream 9 - BaseOS - source
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-baseos-src http://mirror.stream.centos.org/9-stream/BaseOS/source/tree/

# CentOS Stream 9 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs9-appstream http://mirror.stream.centos.org/9-stream/AppStream/x86_64/os/

# CentOS Stream 10 - BaseOS - source
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"centos-stream/cs10-baseos-src http://mirror.stream.centos.org/10-stream/BaseOS/source/tree/

# CentOS Stream 10 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs10-baseos http://mirror.stream.centos.org/10-stream/BaseOS/x86_64/os/

# CentOS Stream 10 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/centos-stream/cs10-appstream http://mirror.stream.centos.org/10-stream/AppStream/x86_64/os/


# Fedora 44
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/fedora/fedora44 https://dl.fedoraproject.org/pub/fedora/linux/releases/44/Everything/x86_64/os/

# Fedora 44 Updates
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/fedora/fedora44-updates https://dl.fedoraproject.org/pub/fedora/linux/updates/44/Everything/x86_64/

# EPEL 9 - Everything
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/fedora/epel9 https://download.fedoraproject.org/pub/epel/9/Everything/x86_64/

# EPEL 10 - Everything
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/fedora/epel10 https://download.fedoraproject.org/pub/epel/10/Everything/x86_64/

# RPMFusion - Fedora 44
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/fedora/rpmfusion-f44 https://download1.rpmfusion.org/free/fedora/releases/44/Everything/x86_64/os/


# Alma Linux 8 - BaseOS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/alma8-baseos https://repo.almalinux.org/almalinux/8/BaseOS/x86_64/os/

# Alma Linux 8 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/alma8-appstream https://repo.almalinux.org/almalinux/8/AppStream/x86_64/os/

# Alma Linux 10 - BaseOS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/alma10-baseos https://repo.almalinux.org/almalinux/10/BaseOS/x86_64/os/

# Alma Linux 10 - Appstream
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/alma10-appstream https://repo.almalinux.org/almalinux/10/AppStream/x86_64/os/

# Oracle Linux 9
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/ol9 https://yum.oracle.com/repo/OracleLinux/OL9/developer/x86_64/

# Oracle Linux 7
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/other/ol7 http://yum.oracle.com/repo/OracleLinux/OL7/latest/x86_64/



# OpenSUSE Tumbleweed
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-tumbleweed https://download.opensuse.org/tumbleweed/repo/oss/

# openSUSE Leap 16.0 - OSS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-leap16-oss https://download.opensuse.org/distribution/leap/16.0/repo/oss/

# openSUSE Leap 16.0 - non-OSS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-leap16-non-oss https://download.opensuse.org/distribution/leap/16.0/repo/non-oss/

# openSUSE Leap 15.6 - OSS
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-leap15.6-oss https://download.opensuse.org/distribution/leap/15.6/repo/oss/

# openSUSE Leap 15.6 - updates
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-leap15.6-updates https://download.opensuse.org/update/leap/15.6/oss/

# openSUSE Leap 42.3 - updates (2019-era repository)
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/suse/opensuse-leap42.3-updates https://download.opensuse.org/update/leap/42.3/oss/



# Microsoft Azure RHEL9 additions
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/ms-rhel9-additions https://packages.microsoft.com/rhel/9/prod/

# Nvidia CUDA tools
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/nvidia-cuda-el9 https://developer.download.nvidia.com/compute/cuda/repos/rhel9/x86_64/

# Puppet 7 - sha checksum
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/puppetlabs-puppet7-el8 https://yum.puppetlabs.com/puppet7/el/8/x86_64/

# Grafana - lots of files in filelists
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/grafana https://packages.grafana.com/oss/rpm/

# Google Cloud SDK EL9 - lots of files in filelists, relatively evenly distributed
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/google-cloud-sdk-el9 https://packages.cloud.google.com/yum/repos/cloud-sdk-el9-x86_64/

# Elasticsearch EL9
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/vendor/elasticsearch-el9 https://artifacts.elastic.co/packages/9.x/yum/



# Rundeck - has a backwards-pointing location href and multiple checksums in repomd.xml
rpmrepo download "${DOWNLOAD_OPTIONS[@]}" "$DEST"/weird/rundeck-location-href https://packages.rundeck.com/pagerduty/rundeck/rpm_any/rpm_any/x86_64/
