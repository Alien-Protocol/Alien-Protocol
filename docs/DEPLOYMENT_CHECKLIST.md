# Alien Gateway — Pre-Deployment Checklist

This checklist must be completed before deploying to testnet or mainnet.
Items marked **[BLOCKER]** must be resolved before any deployment proceeds.

---

## 1. Known Blockers

The following issues **must** be resolved before testnet or mainnet deployment.

| ID | Severity | Description | Tracked In |
|----|----------|-------------|------------|
| B-01 | **Critical** | `zk_verifier.rs` stub always returns `true` — `register_resolver` accepts any proof without verification. | Phase 4 |
| B-02 | **Critical** | `FactoryContract::configure()` has no auth guard — any account can overwrite the auction/core contract addresses post-deploy. | Phase 2 |
| B-03 | **High** | `CoreContract::set_memo()` has no auth guard — any account can overwrite another user's memo ID. | Phase 2 |
| B-04 | **High** | `FactoryContract::configure()` has no idempotency protection — a second call silently overwrites the first configuration. | Phase 2 |
| B-05 | **High** | SMT root must be seeded on-chain before any `register_resolver` call; no privileged initialization entry point exists yet. Calling `register_resolver` on a fresh deployment panics with `RootNotSet`. | Deployment |
| B-06 | **Medium** | Phase 2 ZK circuits (non-inclusion proof, update proof) are incomplete — on-chain root advancement is currently owner-trusted with no cryptographic enforcement. | Phase 2 |
| B-07 | **Medium** | `isValid` is hardcoded to `1` in `zk_verifier.rs` — the verifier is a non-functional stub. | Phase 4 |
| B-08 | **Low** | Trusted setup (`powers_of_tau.ptau`) is local only — no publicly verifiable ceremony has been conducted. | Phase 4 |
| B-09 | **Low** | `AuctionContract` has no `configure_factory` entry point — the factory address must be written directly to instance storage and cannot be set via a permissioned call. | Phase 2 |

---

## 2. Contract Deployment Order

Contracts have the following dependency graph. Deploy in this exact order:

```
1. CoreContract         — no external dependencies
2. AuctionContract      — no external dependencies (factory address wired separately)
3. FactoryContract      — requires CoreContract and AuctionContract addresses
4. EscrowContract       — no external dependencies; vaults created post-deploy
```

### Step-by-step

```bash
# Build all contracts first
cd gateway-contract
stellar contract build

# 1. Deploy CoreContract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/core_contract.wasm \
  --source <DEPLOYER_KEYPAIR> \
  --network testnet
# Save the output as: CORE_CONTRACT_ID

# 2. Deploy AuctionContract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/auction_contract.wasm \
  --source <DEPLOYER_KEYPAIR> \
  --network testnet
# Save the output as: AUCTION_CONTRACT_ID

# 3. Deploy FactoryContract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/factory_contract.wasm \
  --source <DEPLOYER_KEYPAIR> \
  --network testnet
# Save the output as: FACTORY_CONTRACT_ID

# 4. Deploy EscrowContract
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/escrow_contract.wasm \
  --source <DEPLOYER_KEYPAIR> \
  --network testnet
# Save the output as: ESCROW_CONTRACT_ID
```

---

## 3. Post-Deploy Configuration

Run these steps **before** allowing users to interact with the system.

### 3.1 Configure FactoryContract [BLOCKER: B-02 must be fixed first]

```bash
stellar contract invoke \
  --id $FACTORY_CONTRACT_ID \
  --source <ADMIN_KEYPAIR> \
  --network testnet \
  -- configure \
  --auction_contract $AUCTION_CONTRACT_ID \
  --core_contract $CORE_CONTRACT_ID
```

**Verify:**
```bash
# Should return AUCTION_CONTRACT_ID
stellar contract invoke --id $FACTORY_CONTRACT_ID -- get_auction_contract

# Should return CORE_CONTRACT_ID
stellar contract invoke --id $FACTORY_CONTRACT_ID -- get_core_contract
```

### 3.2 Wire AuctionContract → FactoryContract [BLOCKER: B-09 must be fixed first]

Currently requires a direct storage write because `AuctionContract` has no `configure_factory`
entry point. A privileged configuration function must be added before testnet deployment.

### 3.3 Seed the SMT Root [BLOCKER: B-05 must be fixed first]

The `CoreContract` SMT root must be initialized before any `register_resolver` call can succeed.
A privileged `initialize_root(initial_root: BytesN<32>)` entry point needs to be implemented.

```bash
# Once B-05 is resolved:
stellar contract invoke \
  --id $CORE_CONTRACT_ID \
  --source <ADMIN_KEYPAIR> \
  --network testnet \
  -- initialize_root \
  --initial_root <GENESIS_SMT_ROOT_HEX>
```

**Verify:**
```bash
stellar contract invoke --id $CORE_CONTRACT_ID -- get_smt_root
# Should return the genesis root, not panic with RootNotSet
```

---

## 4. Role & Access Matrix

| Contract | Function | Required Caller | Auth Status |
|----------|----------|----------------|-------------|
| `FactoryContract` | `configure()` | Admin / Deployer | **No auth guard [B-02]** |
| `FactoryContract` | `deploy_username()` | `AuctionContract` only | Enforced via `auction_contract.require_auth()` |
| `CoreContract` | `register_resolver()` | Any authenticated address | Enforced via `caller.require_auth()` |
| `CoreContract` | `set_memo()` | Registered commitment owner | **No auth guard [B-03]** |
| `CoreContract` | `add_stellar_address()` | Registered commitment owner | Enforced — owner check + `require_auth` |
| `CoreContract` | `add_chain_address()` | Registered commitment owner | Enforced via `AddressManager` |
| `CoreContract` | `remove_chain_address()` | Registered commitment owner | Enforced via `AddressManager` |
| `EscrowContract` | `schedule_payment()` | Vault owner | Enforced via `config.owner.require_auth()` |
| `EscrowContract` | `setup_auto_pay()` | Vault owner | Enforced via `config.owner.require_auth()` |
| `EscrowContract` | `trigger_auto_pay()` | Anyone (keeper / bot) | No auth required by design |
| `EscrowContract` | `execute_scheduled()` | Anyone | No auth required by design |

---

## 5. Post-Deploy Verification

Run these checks after all configuration steps complete.

### 5.1 Factory Configuration
- [ ] `get_auction_contract()` returns the correct `AuctionContract` address
- [ ] `get_core_contract()` returns the correct `CoreContract` address
- [ ] Calling `configure()` a second time is blocked (requires B-02/B-04 fix)

### 5.2 Core Contract
- [ ] `get_smt_root()` succeeds without panicking (requires B-05 fix)
- [ ] `register_resolver()` succeeds with a valid proof and matching SMT roots
- [ ] `register_resolver()` rejects a stale root with `StaleRoot` error

### 5.3 Auction → Factory Wiring
- [ ] `claim_username()` on a closed auction triggers `FactoryContract.deploy_username()`
- [ ] `get_username_record()` on the factory returns the correct `core_contract` reference

### 5.4 Escrow
- [ ] Vault creation entry point is implemented and tested
- [ ] `schedule_payment()` reserves funds correctly
- [ ] `execute_scheduled()` transfers tokens at or after `release_at`

### 5.5 Automated Test Suite
```bash
cd gateway-contract
cargo test --verbose    # must pass with zero failures
cargo clippy -- -D warnings  # must pass with zero warnings
```

---

## 6. Security Pre-flight Checklist

- [ ] **[B-01]** ZK verifier stub replaced with real Groth16 implementation (Phase 4)
- [ ] **[B-02]** Auth guard added to `FactoryContract::configure()`
- [ ] **[B-03]** Auth guard added to `CoreContract::set_memo()`
- [ ] **[B-04]** Idempotency guard (or one-time initialization flag) added to `configure()`
- [ ] **[B-05]** SMT root seeding entry point implemented and callable post-deploy
- [ ] **[B-06]** Phase 2 ZK circuits complete and integrated with `register_resolver`
- [ ] **[B-07]** `isValid` stub replaced by real circuit output validation
- [ ] **[B-08]** Trusted setup ceremony conducted publicly and verifiably
- [ ] **[B-09]** `configure_factory` entry point added to `AuctionContract`
- [ ] External security audit completed (Phase 6)
- [ ] Threat model reviewed against final contract surface

---

## 7. Testnet Readiness Gate

| Gate | Requirement | Status |
|------|-------------|--------|
| G-01 | Blockers B-01 through B-05 resolved | 🔴 Not ready |
| G-02 | `cargo test` passes with zero failures | ✅ Ready |
| G-03 | `cargo clippy` passes with zero warnings | ✅ Ready |
| G-04 | All contracts deployed and cross-configured | 🔴 Not started |
| G-05 | SMT root seeded on testnet | 🔴 Blocked by B-05 |
| G-06 | Auction → Factory cross-contract flow verified on testnet | 🔴 Not started |
| G-07 | Security review and threat model sign-off | 🔴 Pending Phase 6 |

---

*See also: [docs/threat-model.md](threat-model.md) for the full threat analysis.*

*Last updated: 2026-04-23*
