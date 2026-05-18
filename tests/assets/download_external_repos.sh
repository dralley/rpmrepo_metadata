#!/usr/bin/bash

set -euo pipefail

DEST="${1:-external_repos2}"

### Download external repositories

CentOS 7
rpmrepo download --only-metadata "$DEST"/centos-stream/centos7 http://vault.centos.org/centos/7/os/x86_64/

# CentOS Stream 8 - BaseOS
rpmrepo download --only-metadata "$DEST"/centos-stream/cs8-baseos http://vault.centos.org/centos/8-stream/BaseOS/x86_64/os/

# CentOS Stream 8 - Appstream
rpmrepo download --only-metadata "$DEST"/centos-stream/cs8-appstream http://vault.centos.org/centos/8-stream/AppStream/x86_64/os/

# CentOS Stream 9 - BaseOS
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-baseos http://mirror.stream.centos.org/9-stream/BaseOS/x86_64/os/

# CentOS Stream 9 - BaseOS - aarch64
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-baseos-aarch64 http://mirror.stream.centos.org/9-stream/BaseOS/aarch64/os/

# CentOS Stream 9 - BaseOS - ppc64le
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-baseos-ppc64le http://mirror.stream.centos.org/9-stream/BaseOS/ppc64le/os/

# CentOS Stream 9 - BaseOS - s390x
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-baseos-s390x http://mirror.stream.centos.org/9-stream/BaseOS/s390x/os/

# CentOS Stream 9 - BaseOS - source
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-baseos-src http://mirror.stream.centos.org/9-stream/BaseOS/source/tree/

# CentOS Stream 9 - Appstream
rpmrepo download --only-metadata "$DEST"/centos-stream/cs9-appstream http://mirror.stream.centos.org/9-stream/AppStream/x86_64/os/

# CentOS Stream 10 - BaseOS - source
rpmrepo download --only-metadata "$DEST"centos-stream//cs10-baseos-src http://mirror.stream.centos.org/10-stream/BaseOS/source/tree/

# CentOS Stream 10 - Appstream
rpmrepo download --only-metadata "$DEST"/centos-stream/cs10-baseos http://mirror.stream.centos.org/10-stream/BaseOS/x86_64/os/

# CentOS Stream 10 - Appstream
rpmrepo download --only-metadata "$DEST"/centos-stream/cs10-appstream http://mirror.stream.centos.org/10-stream/AppStream/x86_64/os/

# Fedora 42
rpmrepo download --only-metadata "$DEST"/fedora/fedora42 https://dl.fedoraproject.org/pub/fedora/linux/releases/42/Everything/x86_64/os/

# Fedora 42 Updates
rpmrepo download --only-metadata "$DEST"/fedora/fedora42-updates https://dl.fedoraproject.org/pub/fedora/linux/updates/42/Everything/x86_64/

# EPEL 10 - Everything
rpmrepo download --only-metadata "$DEST"/fedora/epel9 https://download.fedoraproject.org/pub/epel/10/Everything/x86_64/

# RPMFusion - Fedora 42
rpmrepo download --only-metadata "$DEST"/fedora/rpmfusion-f42 https://download1.rpmfusion.org/free/fedora/releases/42/Everything/x86_64/os/

# OpenSUSE Tumbleweed
rpmrepo download --only-metadata "$DEST"/other/opensuse-tumbleweed https://download.opensuse.org/tumbleweed/repo/oss/

# Alma Linux 8 - BaseOS
rpmrepo download --only-metadata "$DEST"/other/alma8-baseos https://repo.almalinux.org/almalinux/8/BaseOS/x86_64/os/

# Alma Linux 8 - Appstream
rpmrepo download --only-metadata "$DEST"/other/alma8-appstream https://repo.almalinux.org/almalinux/8/AppStream/x86_64/os/

# Oracle Linux 9
rpmrepo download --only-metadata "$DEST"/other/ol9 https://yum.oracle.com/repo/OracleLinux/OL9/developer/x86_64/

# Microsoft Azure RHEL9 additions
rpmrepo download --only-metadata "$DEST"/vendor/ms-rhel9-additions https://packages.microsoft.com/rhel/9/prod/

# Nvidia CUDA tools
rpmrepo download --only-metadata "$DEST"/vendor/nvidia-cuda-el9 https://developer.download.nvidia.com/compute/cuda/repos/rhel9/x86_64/

# Puppet 7 - sha checksum
rpmrepo download --only-metadata "$DEST"/vendor/puppetlabs-puppet7-el8 https://yum.puppetlabs.com/puppet7/el/8/x86_64/

# Grafana - lots of files in filelists
rpmrepo download --only-metadata "$DEST"/vendor/grafana https://packages.grafana.com/oss/rpm/

# Google Cloud SDK EL9 - lots of files in filelists, relatively evenly distributed
rpmrepo download --only-metadata "$DEST"/vendor/google-cloud-skd-el9 https://packages.cloud.google.com/yum/repos/cloud-sdk-el9-x86_64/

# Elasticsearch EL9
rpmrepo download --only-metadata "$DEST"/vendor/elasticsearch-el9 https://artifacts.elastic.co/packages/9.x/yum/

# Rundeck - has a backwards-pointing location href and multiple checksums in repomd.xml
rpmrepo download --only-metadata "$DEST"/weird/rundeck-location-href https://packages.rundeck.com/pagerduty/rundeck/rpm_any/rpm_any/x86_64/
