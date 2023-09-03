#!/bin/bash

wasm-pack build --dev

# add {"type": "module"} to the generated package.json
# this makes it so that node can import it
jq ". + {type: \"module\", main: \"retirement.js\"}" pkg/package.json > pkg/package.json.tmp
mv pkg/package.json.tmp pkg/package.json
cp .npmrc pkg
