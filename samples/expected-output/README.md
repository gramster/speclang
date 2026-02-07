# Expected Output

This directory contains the expected output from running the speclang
compiler on the sample files. You can use these to see what the
toolchain produces without building first, or to verify your build
matches.

## Generating fresh output

```bash
cargo run -- compile samples/hello.spl > samples/expected-output/hello.rs
cargo run -- wasm samples/hello.spl > samples/expected-output/hello.wat
cargo run -- compile samples/music.spl > samples/expected-output/music.rs
cargo run -- wasm samples/music.spl > samples/expected-output/music.wat
```
