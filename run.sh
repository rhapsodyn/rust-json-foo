#!/bin/zsh

cargo build --release && ./target/release/rust-json-foo
echo "------ compare with local nodejs -------"
node v8-json.js
