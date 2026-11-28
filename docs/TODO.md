# Alien Protocol work queue

Contract status reviewed on 28 November 2026. The implementation and operating assumptions are described in [V1 contract operations](contracts.md).

## 1. Smart contracts

| Stage | Verified deliverable | Remaining work |
| --- | --- | --- |
| 1. Interfaces and custody | Shared price, position and risk-config types. Vault withdrawal requires a successful debt read. Risk metadata is locked for open collateral. | Measure network costs for slot-based indexing and rehearse batched migration on testnet. Review issuer clawback and asset-freeze handling for each proposed collateral token. |
| 2. Lending and liquidation | Fixed APR with fractional carry, interest-first repayment, dust checks, health-factor query without a pool callback, direct liquidator-to-pool settlement, pro-rata distribution of paid interest and collateral-limited recovery. | Validate stressed withdrawal liquidity and token granularity for launch assets. |
| 3. Integration and tests | Four-contract lifecycle, partial and full liquidation, explicit liquidator authorization, stale-price rejection and rollback after failed payment. Vault upgrade and migration tests run in the standard suite. | Exercise issuer restrictions and deployment wiring on testnet. |
| 4. Operations and review | Local build, tests, risk model, pause sequence and operator TTL duties documented. | Independent contract review, dependency review and resource measurements before a funded release. |

Future schema changes require migration tests against existing data. Keep published error codes stable. The oracle supports manual address prices and signed RedStone feed mappings. Deployment-specific addresses, feed IDs, decimals, signers and thresholds must be verified for the chosen network.

## 2. Deploy & tooling

- [x] Local Rust toolchain pinned to 1.94.1.
- [ ] Deploy scripts: build → upload → deploy → initialize (testnet first)
- [ ] Network config (RPC URL, passphrase, identity names) without committing secrets
- [ ] Record contract IDs, WASM hashes, commit SHA after each deploy
- [ ] Align CONTRIBUTING / CI: `stellar contract build --locked` vs `cargo build --target wasm32v1-none`
- [x] Makefile or `justfile` for common verify commands
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
- [x] `.github/CONTRIBUTING.md`
- [x] `SECURITY.md` + private vulnerability reporting path (exists at repo root; linked from README)
- [x] `CHANGELOG.md` at repo root
- [x] README links to local verification and the V1 contract operations guide.
- [ ] Document backend routes when the backend exists.
- [x] Document implemented V1 risk parameters and pause controls.
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
