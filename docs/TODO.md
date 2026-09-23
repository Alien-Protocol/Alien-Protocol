# Alien Protocol — TODO

Living checklist of work still open. Status reflects the repo as of **9 September 2026**.

Use this with `docs/arch.md` (target design) and `README.md` (honest component status). Check items off in PRs; do not treat unchecked items as “in progress” unless an issue is linked.

**Legend:** `[ ]` todo · `[~]` partial / in progress · `[x]` done enough for current phase

---

## Snapshot

| Area | Status | Notes |
|------|--------|--------|
| Collateral vault | `[~]` | Core custody + risk paths exist; oracle typing / full 4-way integration still open |
| Oracle adapter | `[~]` | Feeder + RedStone paths exist; not fully wired to vault `PriceData` |
| Lending pool | `[~]` | Borrow/repay/supply/accrual implemented; still mock-heavy in places |
| Liquidation engine | `[~]` | Liquidate path exists; tests use mocks, not real vault/pool/oracle |
| Shared crate | `[~]` | Types/math/constants exist; duplicate types remain in feature crates |
| Cross-contract integration | `[~]` | Vault↔pool tests with mock oracle; no full vault↔pool↔oracle↔engine suite |
| Testnet deploy | `[ ]` | No scripts, IDs, or network config in-repo |
| Backend | `[ ]` | No `backend/` tree |
| Frontend | `[ ]` | No `frontend/` tree |
| Production ops | `[ ]` | Contract CI only |

**Suggested focus order:** close contract integration gaps → testnet deploy scripts → thin backend reads → indexer/worker → frontend.

---

## 1. Smart contracts

### 1.1 Cross-contract / system integration

- [ ] End-to-end native test: **real** `collateral-vault` + `lending-pool` + `oracle-adapter` + `liquidation-engine` in one `Env`
- [ ] Unify `PriceData` across vault / oracle / shared (vault still has 2 fields; oracle + shared have `write_timestamp`)
- [ ] Prove vault valuation against the real `oracle-adapter` (not only `MockOracle`)
- [ ] Prove engine `liquidate` against real vault + pool + oracle (today liquidate tests use mock contracts)
- [ ] Document and enforce init order: oracle → vault → pool → engine (addresses, allowlists, borrow asset)
- [ ] Confirm seize path only succeeds when position is actually liquidatable under protocol HF rules
- [ ] TTL / archival strategy for persistent keys (positions, debts, prices) — extend on hot paths or document operator duty

### 1.2 Collateral vault

- [x] `initialize`, deposit / withdraw, pause flags, admin / role setters
- [x] Asset allowlist + per-asset config (decimals, LTV, liquidation threshold)
- [x] Position tracking, `seize_collateral`, upgrade + migrate skeleton
- [x] Health / collateral value helpers using oracle client
- [ ] Fix or remove ignored WASM upgrade test (`test_upgrade` ignored for env host limits on SDK 23)
- [ ] Real storage migrate steps when schema > 2 needs data transforms (today `1 → 2` is effectively a no-op)
- [ ] Harden clawback / freeze / missing-trustline failure modes for RWA SACs
- [ ] Bound or paginate `get_all_positions` / position index growth for production scale
- [ ] Revisit withdrawal safety vs per-asset liquidation threshold (ensure aligned with `shared` HF helpers everywhere)

### 1.3 Lending pool

- [x] Supply / withdraw liquidity, borrow / repay / repay_for
- [x] Debt storage + linear APR accrual helpers
- [x] Pause, admin, vault/oracle wiring
- [~] Vault composition tests (`test_pool_integration`) — still mock oracle
- [ ] Replace remaining `unimplemented!()` mock vault stubs in repay tests with real vault or thin complete mocks
- [ ] Utilization-driven behavior beyond reads (V1 may stay fixed APR; document clearly)
- [ ] Dust / minimum remaining debt enforcement vs `shared::MIN_REMAINING_DEBT` on all repay paths
- [ ] Interest-first repayment edge cases (full repay, dust leftover, pause mid-accrual)
- [ ] Explicit borrow-asset allowlist / USDC SAC assumptions documented for deploy

### 1.4 Liquidation engine

- [x] `liquidate`, `is_liquidatable`, bonus / partial repayment helpers
- [x] Admin + vault/pool/oracle setters
- [ ] Integration tests with **real** vault, pool, and oracle-adapter
- [ ] Full vs partial liquidation fallback when leftover would be unsafe or dust
- [ ] Permissionless liquidator UX path verified (auth trees, token approvals, events)
- [ ] Align HF / close-factor / bonus constants with vault withdraw safety in one shared source of truth
- [ ] Pause / circuit-breaker behavior if vault or pool is paused mid-liquidation

### 1.5 Oracle adapter

- [x] Admin/feeder `set_price`, staleness, pause, RedStone push/pull
- [ ] Single price model consumed by vault and engine (address-keyed vs RedStone symbol-keyed mapping)
- [ ] Map RWA / collateral asset addresses ↔ RedStone feed IDs for production assets
- [ ] Multi-feeder / deviation bounds (architecture Phase 3 aggregation — not required for V1 demo)
- [ ] Fail-closed policy documented for every consumer (`get_price_or_fail` only)

### 1.6 Shared crate

- [x] `PriceData`, `Debt`, `AssetConfig`, interest / HF / LTV helpers, constants
- [ ] Deduplicate vault-local `PriceData` / `AssetConfig` in favor of `shared` where ABI-safe
- [ ] Shared event topics or docs so indexers have one schema
- [ ] Keep error codes stable; never renumber after testnet

### 1.7 Contract quality / security

- [ ] Scout / static analysis pass before testnet
- [ ] Auth-tree review on all wrappers that call `token.transfer` / `repay_for` / `seize_collateral`
- [ ] Fuzz or property tests for deposit/withdraw/borrow/repay/liquidate value paths
- [ ] Threat model doc (`docs/security.md` or `SECURITY.md`)
- [ ] Pause runbook (who pauses what, in which order)
- [ ] No mainnet funds until external review plan exists (README already warns unaudited)

---

## 2. Deploy & tooling

- [ ] `rust-toolchain.toml` pinned to CI toolchain
- [ ] Deploy scripts: build → upload → deploy → initialize (testnet first)
- [ ] Network config (RPC URL, passphrase, identity names) without committing secrets
- [ ] Record contract IDs, WASM hashes, commit SHA after each deploy
- [ ] Align CONTRIBUTING / CI: `stellar contract build --locked` vs `cargo build --target wasm32v1-none`
- [ ] Makefile or `justfile` for common verify commands
- [ ] Friendbot / key generation docs for contributors

---

## 3. Backend (Phase 2 — not started)

Target layout from `docs/arch.md`:

```text
backend/
  api/
  indexer/
  liquidation-worker/
  oracle-service/
  migrations/
```

- [ ] Scaffold Axum workspace + config (network, contract IDs, DB, Redis)
- [ ] PostgreSQL schema for positions, debts, liquidations, events
- [ ] Redis caching for prices / read models
- [ ] Stellar RPC client (reads first: position, debt, price, HF)
- [ ] Event indexer for vault/pool/oracle/engine `#[contractevent]`s
- [ ] Oracle feeder service (push prices / RedStone) with auth’d keys out of git
- [ ] Liquidation worker (backstop; protocol remains permissionless on-chain)
- [ ] Analytics / health APIs
- [ ] Auth challenge endpoint (SEP-10 style) when wallet login is required
- [ ] Docker Compose for API + Postgres + Redis

**Do not** build the full worker/indexer until contract IDs exist on testnet and 4-contract integration tests pass.

---

## 4. Frontend (Phase 3 — not started)

- [ ] App scaffold (Next.js / React per stack choice)
- [ ] Wallet connect (Freighter / Stellar Wallets Kit)
- [ ] Dashboard: positions, HF, prices
- [ ] Deposit / withdraw flows
- [ ] Borrow / repay flows
- [ ] Liquidation history / analytics views
- [ ] Clear pending / success / failed tx UX
- [ ] Keep blockchain logic out of UI components (dedicated modules)

---

## 5. Docs & process

- [x] `docs/arch.md` product architecture
- [x] `docs/CONTRIBUTING.md`
- [ ] Keep README component status table in sync with this TODO
- [ ] `docs/api.md` when backend routes exist
- [ ] `docs/risk-model.md` (per-asset LTV table from arch, as implemented)
- [ ] `SECURITY.md` + private vulnerability reporting path
- [ ] Refresh or archive `private/implementation-status.md` (Aug 2026; pool/engine no longer hello stubs)
- [ ] Deployment / ops runbook

---

## 6. Production engineering (Phase 4)

- [ ] Tracing / metrics / health checks
- [ ] Monitoring (oracle freshness, pause events, large transfers)
- [ ] Testnet launch checklist (`stellar-deploy` rule)
- [ ] Expand CI beyond contracts when backend/frontend land
- [ ] Grant packaging only after contracts + backend + demoable liquidation (see arch §17)

---

## 7. Explicitly out of scope for V1 (track, do not start early)

From arch V2–V4 — leave unchecked until V1 ships:

- [ ] Utilization-based variable rates
- [ ] Decentralized oracle aggregation
- [ ] Governance / insurance fund / liquidation marketplace
- [ ] Cross-chain collateral
- [ ] Institutional permissioned pools

---

## How to use this file

1. Open or claim a GitHub issue for any non-trivial checkbox.
2. Prefer one PR per checkbox cluster (e.g. “unify PriceData” separate from “Axum scaffold”).
3. When closing a contract gap, add or extend a test that would have failed before the fix.
4. Update the **Snapshot** table when a whole row changes status.
