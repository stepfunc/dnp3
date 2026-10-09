#!/usr/bin/env bash
# Build a product and record the evidence bom-tools needs: the cargo build log and the full and
# runtime `cargo tree` outputs, all from the same arguments in this environment.
# See https://github.com/stepfunc/bom-tools#producing-the-build-evidence
#
# usage: build-evidence.sh <evidence dir> <cargo|cross> <build arguments (-p, --target, features)...>
set -euo pipefail

out="$1"
build="$2"
shift 2
mkdir -p "$out"
tree_format=(--prefix depth --color never --format '{p}|{f}')

# a normal build first, so compiler errors stay readable in the CI log
"$build" build --release --locked "$@"
# then the same build as JSON; everything is fresh, so this only replays the artifacts
"$build" build --release --locked "$@" --message-format json > "$out/build.json"
cargo tree --locked "$@" -e normal,build "${tree_format[@]}" > "$out/tree.txt"
cargo tree --locked "$@" -e normal,no-proc-macro "${tree_format[@]}" > "$out/runtime-tree.txt"
