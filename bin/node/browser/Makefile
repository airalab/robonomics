DIR=$(dir $(realpath $(firstword $(MAKEFILE_LIST))))

all: pkg
	python -m http.server 8000

pkg: ${DIR}/../cli
	wasm-pack build --target web --out-dir ${DIR}$@ --no-typescript --release $^ -- --no-default-features --features "browser"

clean:
	rm -rf ${DIR}pkg
