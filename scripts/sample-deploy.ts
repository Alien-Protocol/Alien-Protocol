/**
 * Alien Protocol — RedStone adapter deploy script (Stellar Testnet).
 *
 * Connects to Stellar using credentials from `.env`, deploys the compiled
 * RedStone `redstone_adapter` WASM, initialises it, and saves the resulting
 * contract id to `stellar/redstone_adapter-id.<network>`.
 *
 * Run with: `yarn sample-deploy`
 *
 * Reference:
 *   https://docs.redstone.finance/docs/dapps/non-evm/stellar/typescript-tutorial/
 */
import { existsSync } from "fs";
import {
  makeKeypair,
  PriceAdapterStellarContractConnector,
  StellarClientBuilder,
  StellarContractDeployer,
} from "@redstone-finance/stellar-connector";
import {
  adapterWasmPath,
  readNetwork,
  readPrivateKey,
  readUrl,
  saveAdapterId,
} from "./utils";

/**
 * Number of times to (re)attempt the on-chain deploy. Public Testnet ledger
 * close + RPC propagation can exceed the connector's internal tx-wait window,
 * surfacing as a transient "timed out" error even when the tx actually lands.
 * Retrying makes a single `yarn sample-deploy` reliably complete. Override
 * with the DEPLOY_RETRIES env var.
 */
const DEPLOY_RETRIES = Number(process.env.DEPLOY_RETRIES ?? 5);

function isTransient(err: unknown): boolean {
  const msg = String((err as Error)?.message ?? err);
  return /timed out|Agreement failed|TRY_AGAIN|got max 0 equal results/i.test(
    msg
  );
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function sampleDeploy(): Promise<void> {
  const network = readNetwork();
  const wasmPath = adapterWasmPath();

  if (!existsSync(wasmPath)) {
    throw new Error(
      `Compiled adapter WASM not found at "${wasmPath}".\n` +
        `Build the RedStone redstone_adapter contract first and either place ` +
        `the WASM at that path or set ADAPTER_WASM_PATH in your .env.`
    );
  }

  const keypair = makeKeypair(readPrivateKey());

  const client = new StellarClientBuilder()
    .withStellarNetwork(network)
    .withRpcUrl(readUrl())
    .build();

  console.log(`🌐 Network:   ${network}`);
  console.log(`🔑 Deployer:  ${keypair.publicKey()}`);
  console.log(`📦 WASM:      ${wasmPath}`);
  console.log("⏳ Deploying RedStone adapter contract...");

  const deployer = new StellarContractDeployer(client, keypair);

  let contractId = "";
  for (let attempt = 1; attempt <= DEPLOY_RETRIES; attempt++) {
    try {
      const result = await deployer.deploy(wasmPath);
      contractId = result.contractId;
      console.log(`🧬 WASM hash: ${result.wasmHash}`);
      console.log(`🚀 Adapter contract deployed at: ${contractId}`);

      // Initialise the freshly deployed adapter with the deployer as admin.
      const adapter = await new PriceAdapterStellarContractConnector(
        client,
        contractId,
        keypair
      ).getAdapter();
      await adapter.init(keypair.publicKey());
      console.log(`🛠️  Adapter initialised (admin: ${keypair.publicKey()})`);
      break;
    } catch (err) {
      if (attempt < DEPLOY_RETRIES && isTransient(err)) {
        console.warn(
          `⚠️  Attempt ${attempt}/${DEPLOY_RETRIES} hit a transient network ` +
            `timeout; retrying...`
        );
        await sleep(4000);
        continue;
      }
      throw err;
    }
  }

  saveAdapterId(contractId);

  // Make the contract id easy to scrape from logs / CI.
  console.log(`CONTRACT_ID=${contractId}`);
}

void sampleDeploy().catch((err) => {
  console.error("❌ Deploy failed:");
  console.error(err);
  process.exit(1);
});
