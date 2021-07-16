# TODO


## Use cases

* Killer feature?  streaming API
  * pull-based package parsing API, return one package at a time while streaming over metadata
  * (probably) better alternative to the callback-based API of createrepo_c
  * drawback: relies on the order of packages in primary.xml, filelists.xml, other.xml being the same
  * upside: way less memory consumption, user has total control over how fast data is being pulled and when it is freed
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

### filelists.xml

* tests

### primary.xml

* serialize tests
* deserialize tests

### other.xml

* tests

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
