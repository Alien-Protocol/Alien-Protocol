# Caatinga deployment workflow

The root `caatinga.config.ts` registers the five deployable Soroban contracts and
uses the Rust workspace as one build root. Caatinga records testnet contract IDs
and WASM hashes in the tracked `caatinga.artifacts.json`. It does not replace the
contracts' initialization or risk configuration.

The current testnet addresses and WASM hashes are in the [deployment record](deployment.md).

## Prepare

Install Node.js 22+, the Stellar CLI, and the Rust toolchain and WASM target
required by [Caatinga](https://github.com/Dione-b/caatinga/blob/main/docs/getting-started.md).
The repository pins a Rust version supported by Caatinga; install its
`wasm32v1-none` target before building.

```bash
npm ci
stellar keys generate alice --fund --network testnet
npm run contracts:doctor -- --source alice
npm run contracts:build
```

Replace `alice` with the local Stellar CLI identity that will administer the
contracts. Review the network and identity before submitting any transaction.

## Deploy

Import the administrator key into a local Stellar CLI identity using
`stellar keys add alien-testnet-deployer --secret-key`. The CLI prompts for the
secret; do not put it in a shell command or repository file. Confirm that
`stellar keys address alien-testnet-deployer` matches the expected public
address. The repeatable script verifies this match, builds, deploys changed
WASMs, performs read checks, generates bindings, and refreshes the deployment
record:

```bash
scripts/deploy-testnet.sh alien-testnet-deployer GCJGOM2HPJGHEAGVFGIFN7SVTGWOZIH27E7JL7Q4D4VVIC4NZ7UK2VRK
```

Commit the updated `caatinga.artifacts.json`. The deploy command creates all
five contract instances. The factory is available for the separate atomic
deployment path described in [contract operations](contracts.md); the Caatinga
path deploys protocol instances directly. Their constructors bind the admin
identity, but they are not initialized until the calls below succeed.

Obtain the four protocol IDs from `caatinga.artifacts.json`:

```bash
ORACLE=$(jq -r '.networks.testnet.contracts["oracle-adapter"].contractId' caatinga.artifacts.json)
POOL=$(jq -r '.networks.testnet.contracts["lending-pool"].contractId' caatinga.artifacts.json)
VAULT=$(jq -r '.networks.testnet.contracts["collateral-vault"].contractId' caatinga.artifacts.json)
ENGINE=$(jq -r '.networks.testnet.contracts["liquidation-engine"].contractId' caatinga.artifacts.json)
ADMIN=$(stellar keys address alice)
```

Set `BORROW_ASSET` to the verified Stellar token contract ID for the seven-decimal
quote asset. Choose an oracle freshness window and interest rate for the target
deployment. These example values are for testnet setup only.

```bash
BORROW_ASSET=C... # replace with the verified token contract ID
ctg invoke oracle-adapter.initialize --network testnet --source alice --admin "$ADMIN" --staleness_threshold 3600
ctg invoke collateral-vault.initialize --network testnet --source alice --admin "$ADMIN" --lending_pool "$POOL" --oracle "$ORACLE" --liquidation_engine "$ENGINE"
ctg invoke lending-pool.initialize --network testnet --source alice --admin "$ADMIN" --vault "$VAULT" --oracle "$ORACLE" --borrow_asset "$BORROW_ASSET" --interest_rate_bps 500
ctg invoke liquidation-engine.initialize --network testnet --source alice --admin "$ADMIN" --vault "$VAULT" --pool "$POOL" --oracle "$ORACLE"
```

The initialization calls use the deployed contract IDs stored by Caatinga.
Check `get_admin` and each contract's dependency getters, then configure price
feeds, authorized publishers, collateral assets, and their risk parameters as
described in [contract operations](contracts.md) before accepting deposits.

```bash
npm run contracts:smoke -- --source alice
npm run contracts:generate
```

Caatinga's smoke checks are read-only reachability checks; they do not prove
the contracts are initialized or safe to fund. Run `make check` for the native
and compiled-contract tests. Generated TypeScript bindings are written to
`generated/contracts/` and are ignored by Git until an application consumes
them. No browser client is configured in this Rust-only repository.
