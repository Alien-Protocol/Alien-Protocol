/**
 * Shared helpers for the Alien Protocol <-> RedStone Stellar integration scripts.
 *
 * These mirror the RedStone `stellar-connector` reference scripts
 * (https://docs.redstone.finance/docs/dapps/non-evm/stellar/typescript-tutorial/)
 * but are trimmed to exactly what Alien Protocol needs: a single RedStone
 * `redstone_adapter` contract whose id is persisted to / read from disk.
 */
import "dotenv/config";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "fs";
import path from "path";
import { z } from "zod";

/** Stellar networks supported by the RedStone connector. */
export type StellarNetwork = "mainnet" | "testnet" | "custom";

/** Canonical RedStone adapter contract name (matches the compiled WASM file). */
export const PRICE_ADAPTER = "redstone_adapter";

/** Read a required environment variable, failing loudly when it is missing. */
function requireEnv(name: string): string {
  const value = process.env[name];
  if (value === undefined || value.trim() === "") {
    throw new Error(
      `Missing required environment variable "${name}". ` +
        `Copy .env.example to .env and fill it in.`
    );
  }
  return value.trim();
}

/** The Stellar network selected via the NETWORK env var. */
export function readNetwork(): StellarNetwork {
  return z
    .enum(["testnet", "mainnet", "custom"])
    .parse(process.env.NETWORK ?? "testnet");
}

/** The Soroban RPC URL, validated and returned as a string (as the SDK expects). */
export function readUrl(): string {
  const raw =
    process.env.RPC_URL ??
    (readNetwork() === "testnet"
      ? "https://soroban-testnet.stellar.org"
      : "");
  // Validate it parses as a URL, but hand back the original string.
  return z.string().url().parse(raw);
}

/** Secret key (S...) used to sign transactions. */
export function readPrivateKey(): string {
  return requireEnv("PRIVATE_KEY");
}

/** Directory used to persist deployed contract ids. Defaults to ./stellar. */
export function readDeployDir(): string {
  return process.env.DEPLOY_DIR?.trim() || "./stellar";
}

/** Path to the compiled RedStone adapter WASM that the deploy script uploads. */
export function adapterWasmPath(): string {
  return (
    process.env.ADAPTER_WASM_PATH?.trim() ||
    path.join(
      readDeployDir(),
      "target/wasm32v1-none/release",
      `${PRICE_ADAPTER}.wasm`
    )
  );
}

function idFilePath(): string {
  const network = readNetwork();
  return path.join(readDeployDir(), `${PRICE_ADAPTER}-id.${network}`);
}

/** Persist the deployed adapter contract id next to the network it lives on. */
export function saveAdapterId(contractId: string): string {
  const dir = readDeployDir();
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  const filepath = idFilePath();
  writeFileSync(filepath, contractId);
  console.log(`✅ Adapter contract id saved to ${filepath}`);
  return filepath;
}

/**
 * Load the deployed adapter contract id.
 *
 * Precedence: an explicit `ADAPTER_CONTRACT_ID` env override, otherwise the
 * id file written by the deploy script.
 */
export function loadAdapterId(): string {
  const override = process.env.ADAPTER_CONTRACT_ID?.trim();
  if (override) {
    return override;
  }
  const filepath = idFilePath();
  if (!existsSync(filepath)) {
    throw new Error(
      `No deployed contract id found at "${filepath}". ` +
        `Run \`yarn sample-deploy\` first (or set ADAPTER_CONTRACT_ID in .env).`
    );
  }
  return readFileSync(filepath, "utf-8").trim();
}
