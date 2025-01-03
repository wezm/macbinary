<h1 align="center">
  MacBinary for Rust
</h1>

<div align="center">
  <strong>This crate provides utilities for reading MacBinary files and parsing resource forks commonly used on classic Mac OS.</strong>
</div>

<br>

<div align="center">
  <a href="https://cirrus-ci.com/github/wezm/macbinary">
    <img src="https://api.cirrus-ci.com/github/wezm/macbinary.svg" alt="Build Status"></a>
  <a href="https://docs.rs/macbinary">
    <img src="https://docs.rs/macbinary/badge.svg" alt="Documentation"></a>
  <a href="https://crates.io/crates/macbinary">
    <img src="https://img.shields.io/crates/v/macbinary.svg" alt="Version"></a>
  <img src="https://img.shields.io/crates/l/macbinary.svg" alt="License">
</div>

<br>

Features
--------

* Parse Macbinary I, II, and III files
* Extract individual resources by type and id from resource fork data
* Iterate over all resources in resource fork
* Cross-platform (does not rely on a Mac host)
* Includes WebAssembly bindings. Used by my [online MacBinary parser][7bit-macbinary].
* Supports `no_std` environments
* All parsing is done without heap allocation

Building for WebAssembly
------------------------

There is a `Makefile` that automates building for WebAssembly, it requires you have
`wasm-bindgen` installed. Run `make` (or `gmake` on BSD) to build the artefacts.
The output is put into a `wasm` directory.

License & Credits
-----------------

Licensed under Apache License, Version 2.0 ([LICENSE](LICENSE)). The codebase incorporates
binary parsing code from [Allsorts](https://github.com/yeslogic/allsorts) and the
`NumFrom` trait from [ttf-parser].

[7bit-macbinary]: https://7bit.org/macbinary/
[ttf-parser]: https://github.com/RazrFalcon/ttf-parser/blob/eb6823889302cc55d40ae09c583c5f51324bdf44/src/parser.rs#L160
