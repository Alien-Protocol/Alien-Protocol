# Implementation status — historical snapshot

> **Historical snapshot dated 15 August 2026.** This document records the review as it stood on that date and is not a report of current implementation status. For the current status, see [docs/TODO.md](../docs/TODO.md).

## Review notes from 15 August 2026

- The lending pool and liquidation engine were described as `hello` scaffolds.
- The shared crate was described as a "broken stub".
- `PriceData` was described as mismatched between the vault and oracle.

These notes are retained as historical context; they should not be read as describing the current code.

## What changed since this snapshot

The three findings above were all superseded after 15 August 2026. The table records what each one described then, and what the code actually looks like now — as history, not as a status report.

| Aug 2026 review finding | State of the code as of the `drip` branch (20 October 2026) |
|---|---|
| Lending pool and liquidation engine were `hello` scaffolds | Both are implemented. `contracts/lending-pool/src/lib.rs` exposes the full supply / borrow / repay / repay_for surface with linear APR accrual against real storage; `contracts/liquidation-engine/src/liquidate.rs` implements `liquidate`, `is_liquidatable`, the partial-repay solver, the 8% bonus and the 50% close factor. |
| The shared crate was a "broken stub" | `contracts/shared/src/types.rs` exposes `PriceData`, `Debt`, `AssetConfig`, `CollateralAsset` and `Position`, plus the helpers `accrue_linear_interest`, `health_factor_bps`, `borrow_limit_from_collateral` and `ceil_div`. `shared` is a declared dependency of `collateral-vault`, `lending-pool` and `liquidation-engine` in their `Cargo.toml`, so all three consume it. `contracts/shared/src/events.rs` is still empty. |
| `PriceData` was mismatched between the vault and oracle | The vault no longer defines its own: `contracts/collateral-vault/src/types.rs:133` re-exports it with `pub use shared::PriceData;`, and the engine reads `shared::types::PriceData`. The oracle adapter still declares a structurally identical `PriceData` of its own (`contracts/oracle-adapter/src/types.rs:6`) and is the one contract that does not depend on `shared` — that remaining wiring is tracked in `docs/TODO.md` §1.6. |

For the live, per-area status — including the open cross-contract integration gaps and the still-unwired oracle — see [docs/TODO.md](../docs/TODO.md).
