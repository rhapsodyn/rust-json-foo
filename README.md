# Rust-json-foo

Simple `JSON.parse` in rust, as 1/3 speed of v8's [`JSON.parse`](https://github.com/v8/v8/blob/master/src/json/json-parser.cc).

## Just To Learn Rust

Which means you should also use this project for learning purpose. 

## Some detail

1. Parsing json in `iteration way` (NOT recursion)
1. Boost by `smallvec` and `smartstring` (memory effective)