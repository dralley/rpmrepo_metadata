# Empty repository
mkdir -p empty_repo/
createrepo_c \
  --outputdir=empty_repo/ \
  --revision=1615686706 \
  --checksum=sha256 \
  --repomd-checksum=sha256 \
  --retain-old-md=0 \
  --simple-md-filenames \
  --no-database \
  --pkglist=fixture_data/empty_repo_pkglist.txt \
  fixture_data/packages/

# Complex repository - use all metadata features
mkdir -p complex_repo/
createrepo_c \
  --outputdir=complex_repo/ \
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
  --pkglist=fixture_data/complex_repo_pkglist.txt \
  fixture_data/packages/
