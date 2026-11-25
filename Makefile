CARGO ?= cargo
WASM_TARGET ?= wasm32v1-none
CONTRACTS := collateral-vault lending-pool liquidation-engine oracle-adapter

.PHONY: all check fmt wasm-fixture clippy cargo-check test build

# Run the same checks as CI, in sequence.
all: check

check:
	$(MAKE) fmt
	$(MAKE) wasm-fixture
	$(MAKE) clippy
	$(MAKE) cargo-check
	$(MAKE) test
	$(MAKE) build

fmt:
	$(CARGO) fmt --all --check

# The collateral-vault upgrade tests embed this release WASM at compile time.
wasm-fixture:
	$(CARGO) build --locked -p collateral-vault --target $(WASM_TARGET) --release

clippy: wasm-fixture
	$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

cargo-check:
	$(CARGO) check --workspace

test: wasm-fixture
	$(CARGO) test --locked --workspace --all-features

build:
	$(CARGO) build --locked $(foreach contract,$(CONTRACTS),-p $(contract)) --target $(WASM_TARGET) --release
