#!/usr/bin/bash

set -euo pipefail

# Empty repository
mkdir -p empty_repo/
createrepo_c \
  --outputdir=fixtures/empty_repo/ \
  --revision=1615686706 \
  --checksum=sha256 \
  --repomd-checksum=sha256 \
  --retain-old-md=0 \
  --simple-md-filenames \
  --no-database \
  --pkglist=empty_repo_pkglist.txt \
  packages/

# Complex repository - use all metadata features
mkdir -p complex_repo/
createrepo_c \
  --outputdir=fixtures/complex_repo/ \
  --revision=1615686706 \
  --distro='cpe:/o:fedoraproject:fedora:33,Fedora 33' \
  --content=binary-x86_64 \
  --repo=Fedora \
  --repo=Fedora-Updates \
  --checksum=sha256 \
  --repomd-checksum=sha256 \
  --retain-old-md=0 \
  --simple-md-filenames \
  --no-database \
  --pkglist=complex_repo_pkglist.txt \
  packages/

# assets_dir=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )/fixture_data
# output_dir="$(realpath "$1")"

# # Complex repository - use all metadata features

# mkdir -p "$output_dir"
# for file in $(cat "$assets_dir/complex_repo_pkglist.txt");
# do
#   cp --no-preserve=mode --reflink=auto "$assets_dir/packages/$file" "$output_dir/$file" ;
# done

# while IFS= read -r filename
# do
#   cp --no-preserve=mode --reflink=auto "$assets_dir/packages/$filename" "$output_dir/$filename" ;
# done < "$assets_dir/complex_repo_pkglist.txt"

# createrepo_c \
#   --outputdir="$output_dir" \
#   --revision=1615686706 \
#   --distro='cpe:/o:fedoraproject:fedora:33,Fedora 33' \
#   --content=binary-x86_64 \
#   --repo=Fedora \
#   --repo=Fedora-Updates \
#   --checksum=sha256 \
#   --repomd-checksum=sha256 \
#   --retain-old-md=0 \
#   --simple-md-filenames \
#   --no-database \
#   --pkglist="$assets_dir/complex_repo_pkglist.txt" \
#   "$output_dir"
