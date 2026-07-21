.PHONY: prepare check test update publish

prepare:
	cargo fmt
	cargo clippy --fix --all-targets --all-features --locked --allow-dirty --allow-staged -- -D warnings
	cargo check --all-features --release --locked

check:
	cargo fmt --check
	cargo clippy --all-targets --all-features --locked -- -D warnings
	cargo check --all-features --release --locked

test:
	cargo test --all-features --locked

update:
	git submodule update --init --recursive
	git submodule foreach 'git fetch --tags && git checkout $$(git describe --tags $$(git rev-list --tags --max-count=1))'
	rm -f src/_icons.rs src/_width.rs
	cargo upgrade -i
	cargo check --all-features --locked

publish:
	cargo publish --locked
