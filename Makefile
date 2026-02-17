all:
	cargo build --release
	mkdir -p bin
	cp target/release/rholang-lsp bin/

clean:
	cargo clean
	rm -rf bin

.PHONY: all clean
