#!/bin/bash

rustup run nightly wasm-pack build --release . -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort --target wasm32-unknown-unknown