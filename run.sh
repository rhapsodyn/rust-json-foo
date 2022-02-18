#!/bin/zsh

cargo build --release && ./target/release/rust-json-foo
echo -------------
node v8-json.js
