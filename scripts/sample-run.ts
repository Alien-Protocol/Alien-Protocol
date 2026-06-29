/**
 * Alien Protocol — RedStone adapter price-read script (Stellar Testnet).
 *
 * Reads the saved adapter contract id, connects to the deployed contract, and
 * uses the RedStone Pull Model to fetch live signed prices for XLM, USDC and
 * BTC. The signed payload is passed to the on-chain `get_prices` function
 * (via `getPricesFromPayload`), and each feed's human-readable price and data
 * timestamp is logged.
 *
 * Exits with a non-zero code if any feed fails or returns a non-positive price.
 *
 * Run with: `yarn sample-run`
 *
 * Reference:
 *   https://docs.redstone.finance/docs/dapps/non-evm/stellar/typescript-tutorial/
 */
import {
  ContractParamsProvider,
  getSignersForDataServiceId,
} from "@redstone-finance/sdk";
import {
  makeKeypair,
  PriceAdapterStellarContractConnector,
  StellarClientBuilder,
} from "@redstone-finance/stellar-connector";
import {
  loadAdapterId,
  readNetwork,
  readPrivateKey,
  readUrl,
} from "./utils";

/** Feeds requested from the RedStone data network. */
const FEEDS = ["XLM", "USDC", "BTC"] as const;

/** RedStone production data service. */
const DATA_SERVICE_ID = "redstone-primary-prod";

/** Number of unique signers required for a valid aggregated price. */
const UNIQUE_SIGNERS_COUNT = 3;

/**
 * RedStone prices are returned scaled by 1e8 (8 decimals). Convert a raw
 * on-chain bigint into a human-readable number.
 */
const REDSTONE_DECIMALS = 8;
function toHumanReadable(rawValue: bigint): number {
  return Number(rawValue) / 10 ** REDSTONE_DECIMALS;
}

async function main(): Promise<void> {
  const network = readNetwork();
  const adapterId = loadAdapterId();
  const keypair = makeKeypair(readPrivateKey());

  const client = new StellarClientBuilder()
    .withStellarNetwork(network)
    .withRpcUrl(readUrl())
    .build();

  console.log(`🌐 Network:  ${network}`);
  console.log(`📄 Adapter:  ${adapterId}`);
  console.log(`📡 Feeds:    ${FEEDS.join(", ")}`);
  console.log("⏳ Fetching live signed prices via the RedStone Pull Model...\n");

  const paramsProvider = new ContractParamsProvider({
    dataServiceId: DATA_SERVICE_ID,
    uniqueSignersCount: UNIQUE_SIGNERS_COUNT,
    dataPackagesIds: [...FEEDS],
    authorizedSigners: getSignersForDataServiceId(DATA_SERVICE_ID),
  });

  const adapter = await new PriceAdapterStellarContractConnector(
    client,
    adapterId,
    keypair
  ).getAdapter();

  // Fetch the live signed data packages once so we can report the per-feed
  // data timestamp carried in the signed payload (Pull Model).
  const dataPackages = await paramsProvider.requestDataPackages();

  // Pull Model: fetch the live signed payload and feed it to the on-chain
  // `get_prices` function. Returns aggregated prices in feed order.
  const prices = await adapter.getPricesFromPayload(paramsProvider);
  const feedIds = paramsProvider.getDataFeedIds();

  let hadFailure = false;

  for (let i = 0; i < feedIds.length; i++) {
    const feedId = feedIds[i];
    const rawPrice = prices[i];

    if (rawPrice === undefined || rawPrice <= 0n) {
      console.error(
        `❌ ${feedId}: invalid price (${rawPrice ?? "missing"})`
      );
      hadFailure = true;
      continue;
    }

    // Data timestamp from the signed package (falls back to on-chain storage).
    let timestamp = "n/a";
    const tsMs = dataPackages[feedId]?.[0]?.dataPackage.timestampMilliseconds;
    if (tsMs !== undefined && tsMs > 0) {
      timestamp = new Date(tsMs).toISOString();
    } else {
      try {
        const ts = await adapter.readTimestampFromContract(feedId);
        if (Number.isFinite(ts) && ts > 0) {
          timestamp = new Date(ts).toISOString();
        }
      } catch {
        // Timestamp is best-effort; a price was still successfully read.
      }
    }

    console.log(
      `✅ ${feedId.padEnd(5)} | price: ${toHumanReadable(rawPrice)} USD` +
        ` (raw: ${rawPrice.toString()}) | timestamp: ${timestamp}`
    );
  }

  if (hadFailure) {
    throw new Error("One or more feeds failed to return a valid price.");
  }

  console.log("\n🎉 All feeds returned non-zero prices.");
}

void main().catch((err) => {
  console.error("\n❌ Price read failed:");
  console.error(err);
  process.exit(1);
});
