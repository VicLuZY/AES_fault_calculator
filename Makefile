SHELL := /bin/sh
WASM_TARGET := wasm32-unknown-unknown
WASM_OUT := target/$(WASM_TARGET)/release/faultcalc_wasm.wasm

.PHONY: help verify native wasm standalone sample release clean bundle

help:
	@echo "Targets:"
	@echo "  make verify      Run Rust tests for all crates"
	@echo "  make native      Build native CLI at bin/faultcalc"
	@echo "  make wasm        Build Rust WASM and copy it to web/faultcalc.wasm"
	@echo "  make standalone  Embed web/faultcalc.wasm into web/faultcalc_workstation.html"
	@echo "  make sample      Regenerate cases/sample.json using the Rust builder"
	@echo "  make bundle      Build native, WASM, standalone, and sample"
	@echo "  make release     Build dist/faultcalc-rust-wasm.zip"
	@echo "  make clean       Remove generated build outputs"

verify:
	cargo test --workspace

native:
	cargo build -p faultcalc-cli --release
	mkdir -p bin
	cp target/release/faultcalc bin/faultcalc

wasm:
	rustup target add $(WASM_TARGET)
	cargo build -p faultcalc-wasm --release --target $(WASM_TARGET)
	cp $(WASM_OUT) web/faultcalc.wasm

standalone: wasm native
	bin/faultcalc embed-wasm web/index.template.html web/faultcalc.wasm web/faultcalc_workstation.html

sample: native
	bin/faultcalc sample > cases/sample.json

bundle: verify native wasm standalone sample

release: bundle
	rm -rf dist/faultcalc dist/faultcalc-rust-wasm.zip
	mkdir -p dist/faultcalc
	cp bin/faultcalc dist/faultcalc/
	cp -R web cases docs crates examples Cargo.toml Makefile README.md CODEX_GOAL.txt dist/faultcalc/
	cd dist && zip -qr faultcalc-rust-wasm.zip faultcalc

clean:
	rm -rf bin out target web/faultcalc.wasm web/faultcalc_workstation.html
