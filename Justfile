all:
	cargo clippy --locked -- -D warnings
	cargo fmt -- --check