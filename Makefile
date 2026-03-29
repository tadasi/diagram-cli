.PHONY: install build

# ~/.cargo/bin に dg を入れる（PATH に ~/.cargo/bin が含まれていること）
install:
	cargo install --path . --locked

build:
	cargo build --release
