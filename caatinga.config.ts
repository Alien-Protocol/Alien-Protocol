import { defineConfig } from "@caatinga/core";

const wasmRoot = "./target/wasm32v1-none/release";

export default defineConfig({
  project: "alien-protocol",
  defaultNetwork: "testnet",
  buildRoot: ".",
  contracts: {
    "deployment-factory": {
      path: "./contracts/deployment-factory",
      wasm: `${wasmRoot}/deployment_factory.wasm`,
    },
    "oracle-adapter": {
      path: "./contracts/oracle-adapter",
      wasm: `${wasmRoot}/oracle_adapter.wasm`,
      deployArgs: { admin: "${source.address}" },
    },
    "lending-pool": {
      path: "./contracts/lending-pool",
      wasm: `${wasmRoot}/lending_pool.wasm`,
      deployArgs: { admin: "${source.address}" },
    },
    "collateral-vault": {
      path: "./contracts/collateral-vault",
      wasm: `${wasmRoot}/collateral_vault.wasm`,
      deployArgs: { admin: "${source.address}" },
    },
    "liquidation-engine": {
      path: "./contracts/liquidation-engine",
      wasm: `${wasmRoot}/liquidation_engine.wasm`,
      deployArgs: { admin: "${source.address}" },
    },
  },
  networks: {
    testnet: {
      rpcUrl: "https://soroban-testnet.stellar.org",
      networkPassphrase: "Test SDF Network ; September 2015",
    },
  },
  frontend: {
    bindingsOutput: "./generated/contracts",
  },
  smoke: {
    reads: [
      { contract: "oracle-adapter", method: "get_admin", expect: { matcher: "reachable" } },
      { contract: "lending-pool", method: "get_admin", expect: { matcher: "reachable" } },
      { contract: "collateral-vault", method: "get_contract_version", expect: { matcher: "reachable" } },
      { contract: "liquidation-engine", method: "get_admin", expect: { matcher: "reachable" } },
    ],
  },
});
