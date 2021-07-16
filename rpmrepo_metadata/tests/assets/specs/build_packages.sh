#!/bin/sh

for spec in *.spec; do
    rpmbuild -ba $spec
done
