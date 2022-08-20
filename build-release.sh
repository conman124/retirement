#!/bin/bash

rustup run nightly wasm-pack build --release . -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target wasm32-unknown-unknown

# add {"type": "module"} to the generated package.json
# this makes it so that node can import it
jq ". + {type: \"module\"}" pkg/package.json > pkg/package.json.tmp
mv pkg/package.json.tmp pkg/package.json
