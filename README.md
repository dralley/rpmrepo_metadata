# rpmrepo_metadata

rpmrepo_metadata is a library for manipulating, reading and writing RPM repositories.

## Installation

```
pip install rpmrepo_metadata
```

Note: requires Python >= 3.7.

## Example

```
In [1]: from rpmrepo_metadata import RepositoryReader

In [2]: reader = RepositoryReader("tests/assets/external_repos/centos7/")

In [3]: packages = reader.iter_packages()

In [4]: packages.total_packages
Out[4]: 10072

In [5]: next(packages)
Out[5]: <Package at 0x5613b8983cb0>

In [6]: packages.remaining_packages
Out[6]: 10071

In [7]: for pkg in packages:
   ...:     print(pkg.nevra())
389-ds-base-0:1.3.10.2-6.el7.x86_64
389-ds-base-devel-0:1.3.10.2-6.el7.x86_64
389-ds-base-libs-0:1.3.10.2-6.el7.x86_64
389-ds-base-snmp-0:1.3.10.2-6.el7.x86_64
Cython-0:0.19-5.el7.x86_64
ElectricFence-0:2.2.2-39.el7.i686
ElectricFence-0:2.2.2-39.el7.x86_64
...
...

```
