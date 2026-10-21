.PHONY: prepare check test update

prepare:
	cargo +nightly fmt
	cargo clippy --fix --all-targets --all-features --locked --allow-dirty --allow-staged -- -D warnings
	cargo check --all-features --release --locked

check:
	cargo +nightly fmt --check
	cargo clippy --all-targets --locked -- -D warnings
	cargo clippy --all-targets --all-features --locked -- -D warnings
	cargo check --all-features --release --locked

test:
	cargo test --locked
	cargo test --all-features --locked

update:
	cargo upgrade -i
	cargo run -r --example update-icons --locked
	cargo run -r --example update-widths --locked
	cargo +nightly fmt
	cargo check --all-features --locked

.PHONY: examples
examples: # regenerate the example SVGs
	rm -rf examples/*.svg
	cargo run -r --example generate --all-features --locked
