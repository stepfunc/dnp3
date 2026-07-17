#!/usr/bin/env bash
#
# Bump the crate version across every file that hard-codes it, then refresh
# Cargo.lock. The current version is read from dnp3/Cargo.toml, so you only
# pass the new one.
#
#   ./bump-version.sh 1.7.0-RC2
#
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: $0 <new-version>   (e.g. 1.7.0-RC2)" >&2
  exit 1
fi

NEW="$1"
cd "$(dirname "$0")"

# The files that hard-code the version. This list is the source of truth.
FILES=(
  dnp3/Cargo.toml
  ffi/dnp3-ffi/Cargo.toml
  ffi/dnp3-ffi-java/Cargo.toml
  ffi/dnp3-schema/Cargo.toml
  ffi/dnp3-bindings/Cargo.toml
  dnp3/codegen/pom.xml
  conformance/pom.xml
  ffi/bindings/java/pom.xml
  ffi/bindings/java/examples/pom.xml
  ffi/bindings/dotnet/examples/master/Master.csproj
  ffi/bindings/dotnet/examples/outstation/Outstation.csproj
  ffi/bindings/c/CMakeLists.txt
  guide/sitedata.json
)

OLD="$(sed -n 's/^version = "\(.*\)"/\1/p' dnp3/Cargo.toml | head -n1)"
if [ -z "$OLD" ]; then
  echo "could not determine current version from dnp3/Cargo.toml" >&2
  exit 1
fi

if [ "$OLD" = "$NEW" ]; then
  echo "current version is already $NEW; nothing to do" >&2
  exit 1
fi

echo "bumping $OLD -> $NEW"
for f in "${FILES[@]}"; do
  if ! grep -q -- "$OLD" "$f"; then
    echo "  warning: $OLD not found in $f" >&2
    continue
  fi
  sed -i "s/${OLD}/${NEW}/g" "$f"
  echo "  updated $f"
done

echo "refreshing Cargo.lock"
cargo update -p dnp3 -p dnp3-ffi -p dnp3-ffi-java -p dnp3-schema -p dnp3-bindings --precise "$NEW"

echo "verifying no '$OLD' references remain in the bumped files"
stray=()
for f in "${FILES[@]}"; do
  if grep -q -- "$OLD" "$f"; then
    stray+=("$f")
  fi
done
if [ ${#stray[@]} -ne 0 ]; then
  echo "  version string still present in: ${stray[*]}" >&2
  exit 1
fi
echo "done. review 'git diff', then update CHANGELOG.md."
