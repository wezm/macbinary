all: wasm/macbinary.js

wasm/macbinary.js: target/wasm32-unknown-unknown/release/macbinary.wasm
	wasm-bindgen target/wasm32-unknown-unknown/release/macbinary.wasm --target web --out-dir wasm

target/wasm32-unknown-unknown/release/macbinary.wasm:
	cargo build --lib --target wasm32-unknown-unknown --release

.PHONY: target/wasm32-unknown-unknown/release/macbinary.wasm
