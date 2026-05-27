[![Contract CI](https://github.com/Alien-Protocol/Alien-Protocol/actions/workflows/contract.yml/badge.svg)](https://github.com/Alien-Protocol/Alien-Protocol/actions/workflows/contract.yml)

# Alien Protocol

RWA lending infrastructure on Stellar

## What We Are Building

Alien Protocol is a real-world asset lending protocol being built on Stellar with Soroban. The goal is to let users lock tokenized assets as collateral and borrow against them in a transparent, on-chain system.

This project is planned as more than a single smart contract. It is being designed as a full lending stack with protocol contracts, pricing/oracle support, liquidation and risk monitoring, backend indexing services, and developer-friendly infrastructure around the core lending flow.

In short, Alien Protocol aims to become a reliable credit layer for tokenized assets on Stellar, making RWA-backed borrowing safer, easier to track, and easier to integrate into future apps.

## Planned Core Components

- `collateral-vault`: stores and manages deposited collateral
- `lending-pool`: handles borrowing, repayment, and liquidity logic
- `oracle-adapter`: provides asset pricing data to the protocol
- `liquidation-engine`: monitors unhealthy positions and executes liquidation rules

Together, these components are intended to support a complete RWA-backed lending lifecycle, from deposit and valuation to borrowing, repayment, and risk management.

## Why This Matters

The Stellar ecosystem has strong payment rails, but RWA lending infrastructure is still early. Alien Protocol is being built to help close that gap by creating a programmable credit layer for tokenized assets, with clear risk controls and protocol-level transparency.

## Current Status

This repository is currently in the early contract development stage. The workspace and protocol modules have been scaffolded, and the next phase is to replace the placeholder contract logic with production lending, collateral, oracle, and liquidation flows.

## Repository Structure

```text
contracts/
  collateral-vault/
  lending-pool/
  oracle-adapter/
  liquidation-engine/
```

## Build Direction

The long-term goal is to evolve this repository into the on-chain foundation of Alien Protocol, with smart contracts that can later connect to off-chain services such as indexers, risk monitors, and application interfaces built around the protocol.
