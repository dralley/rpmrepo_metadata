# TODO


## Use cases

* download repository by providing URL
  * download metadata only
  * download metadata + packages
  * sync optimization, repomd revision
  * allowlist/blocklist packages
* downloading many repos, packing into ISO
* create a repository from a directory of packages
  * configure metadata types, checksum types, signing, tags
* sign repository in-place
* verify checksums / signature for repository in-place
* add / remove packages for repository in-place
  * configurable retain old packages
  * move old packages to /old_packages/?

## Tasks

### repomd.xml

* error handling
* parse customization callbacks?
* location_base
  * https://github.com/rpm-software-management/createrepo_c/blob/master/src/xml_dump_repomd.c#L85-L88
  * https://github.com/rpm-software-management/createrepo_c/blob/master/src/xml_parser_repomd.c#L197-L203

### updateinfo.xml

* serialize + basic tests
* deserialize + basic tests

### distribution trees?

### modules?

### general

* download needs to download to tempdir and then move, for purpose of errors
* error reporting back through the CLI
* fancy allocation strategies

### testing

* compression types
  * none
  * gz
  * bzip2
  * xz
  * zchunk
  * zstd
* metadata features
  * repomd.xml - metadata types
  * filelists.xml - ghost files
  * primary.xml - all fields
  * updateinfo.xml - all fields, SUSE fields, differences in required fields
* weird data
  * empty files
  * containing non-ascii
  * containing non-utf8?
  * invalid dates
