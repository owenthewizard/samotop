#!/bin/bash

# This scipt takes care of fixing things that can be fixed automatically.
# - formatting
# - README generation

set -e

cargo fmt --all

for f in `find */ -name Cargo.toml`; do
    pushd `dirname $f` > /dev/null
    pwd
    cargo readme > README.md
    popd > /dev/null
done

git diff --exit-code --color