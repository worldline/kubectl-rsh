default: build

build:
	cargo build

pipeline:
	cargo watch -x check -x build -x test

pod_name=shell
debug:
	RUST_LOG=kubectl_rsh=debug cargo run -- $(pod_name)
