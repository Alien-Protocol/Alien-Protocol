#!/usr/bin/env bash
set -euo pipefail

identity="${1:?Usage: scripts/deploy-testnet.sh <stellar-identity> <expected-public-address>}"
expected_address="${2:?Usage: scripts/deploy-testnet.sh <stellar-identity> <expected-public-address>}"

if [[ ! "$expected_address" =~ ^G[A-Z2-7]{55}$ ]]; then
  echo "Expected address must be a Stellar G address" >&2
  exit 1
fi

actual_address="$(stellar keys address "$identity")"
if [[ "$actual_address" != "$expected_address" ]]; then
  echo "The identity does not match the expected admin address" >&2
  exit 1
fi

npm ci --ignore-scripts --no-audit --no-fund
npm run contracts:doctor -- --source "$identity"
npm run contracts:build
npm run contracts:deploy -- --source "$identity" --if-changed --no-generate
npm run contracts:smoke -- --source "$identity"
npm run contracts:generate
npm run contracts:status
node scripts/write-deployment-record.mjs "$expected_address"
