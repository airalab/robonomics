DIR=$(dir $(realpath $(firstword $(MAKEFILE_LIST))))

all: pkg
	python -m http.server 8000

pkg:
	cargo build --release -p node-cli --target wasm32-unknown-unknown --no-default-features --features browser
	wasm-bindgen ${DIR}../../../../target/wasm32-unknown-unknown/release/node_cli.wasm --out-dir ${DIR}pkg --target web

clean:
	rm -rf ${DIR}pkg
