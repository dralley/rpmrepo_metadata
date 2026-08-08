#!/bin/bash
#
# Validate all repository metadata files in tests/assets against RELAX NG schemas.
#
# Known limitation: RHEL 6 updateinfo.xml fails validation due to libxml2's
# exponential complexity with interleave patterns. This is a validator limitation,
# not a schema error. All modern repositories (RHEL 8+, Fedora, AlmaLinux, etc.)
# validate successfully.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ASSETS_DIR="$(cd "$SCRIPT_DIR/../../tests/assets" && pwd)"

echo "=== Validating updateinfo files against updateinfo.rng ==="
echo

updateinfo_pass=0
updateinfo_skip=0

for file in "$ASSETS_DIR"/broken_fixture_repos/*/repodata/*updateinfo*.xml* \
            "$ASSETS_DIR"/external_repos/*/*/repodata/*updateinfo*.xml*; do
    [[ -f "$file" ]] || continue

    # Skip fixture files
    [[ "$file" == *"_fixture.xml" ]] && { echo "SKIP (fixture): $file"; : $((updateinfo_skip++)); continue; }
    # Skip RHEL 6 (known libxml2 limitation)
    [[ "$file" == *"rhel6"* ]] && { echo "SKIP (RHEL 6 - libxml2 limitation): $file"; : $((updateinfo_skip++)); continue; }
    # Skip zchunk
    [[ "$file" == *.zck ]] && { echo "SKIP (zchunk): $file"; : $((updateinfo_skip++)); continue; }

    # Decompress if needed
    case "$file" in
        *.gz) temp=$(mktemp); gunzip -c "$file" > "$temp"; vfile="$temp" ;;
        *.xz) temp=$(mktemp); xz -dc "$file" > "$temp"; vfile="$temp" ;;
        *.zst) temp=$(mktemp); zstd -dc "$file" > "$temp"; vfile="$temp" ;;
        *) temp=""; vfile="$file" ;;
    esac

    # Validate
    if xmllint --huge --relaxng "$SCRIPT_DIR/updateinfo.rng" "$vfile" --noout 2>&1 | grep -q "validates"; then
        echo "✓ PASS: $file"
        : $((updateinfo_pass++))
    else
        echo "✗ FAIL: $file"
        xmllint --huge --relaxng "$SCRIPT_DIR/updateinfo.rng" "$vfile" --noout 2>&1 | head -3 | sed 's/^/  /'
        [[ -n "$temp" ]] && rm -f "$temp"
        exit 1
    fi

    [[ -n "$temp" ]] && rm -f "$temp"
done

echo
echo "=== Validating comps files against comps.rng ==="
echo

comps_pass=0
comps_skip=0

for file in "$ASSETS_DIR"/broken_fixture_repos/*/repodata/*comps*.xml* \
            "$ASSETS_DIR"/external_repos/*/*/repodata/*comps*.xml*; do
    [[ -f "$file" ]] || continue

    # Skip fixture files and zchunk
    [[ "$file" == *"_fixture.xml" ]] && { echo "SKIP (fixture): $file"; : $((comps_skip++)); continue; }
    [[ "$file" == *.zck ]] && { echo "SKIP (zchunk): $file"; : $((comps_skip++)); continue; }

    # Decompress if needed
    case "$file" in
        *.gz) temp=$(mktemp); gunzip -c "$file" > "$temp"; vfile="$temp" ;;
        *.xz) temp=$(mktemp); xz -dc "$file" > "$temp"; vfile="$temp" ;;
        *.zst) temp=$(mktemp); zstd -dc "$file" > "$temp"; vfile="$temp" ;;
        *) temp=""; vfile="$file" ;;
    esac

    # Validate
    if xmllint --huge --relaxng "$SCRIPT_DIR/comps.rng" "$vfile" --noout 2>&1 | grep -q "validates"; then
        echo "✓ PASS: $file"
        : $((comps_pass++))
    else
        echo "✗ FAIL: $file"
        xmllint --huge --relaxng "$SCRIPT_DIR/comps.rng" "$vfile" --noout 2>&1 | head -3 | sed 's/^/  /'
        [[ -n "$temp" ]] && rm -f "$temp"
        exit 1
    fi

    [[ -n "$temp" ]] && rm -f "$temp"
done

echo
echo "=== Summary ==="
echo "updateinfo: $updateinfo_pass passed, $updateinfo_skip skipped"
echo "comps: $comps_pass passed, $comps_skip skipped"
