import { readFileSync, writeFileSync } from "node:fs";

const admin = process.argv[2];
if (!/^G[A-Z2-7]{55}$/.test(admin ?? "")) {
  throw new Error("Expected a Stellar public address");
}

const artifacts = JSON.parse(readFileSync("caatinga.artifacts.json", "utf8"));
const names = [
  "deployment-factory",
  "oracle-adapter",
  "lending-pool",
  "collateral-vault",
  "liquidation-engine",
];
const contracts = artifacts.networks?.testnet?.contracts;
if (artifacts.project !== "alien-protocol" || !contracts) {
  throw new Error("Missing Alien Protocol testnet artifacts");
}

const rows = names.map((name) => {
  const artifact = contracts[name];
  if (!artifact?.contractId || !artifact?.wasmHash || !artifact?.deployedAt) {
    throw new Error(`Missing deployment data for ${name}`);
  }
  return `| ${name} | \`${artifact.contractId}\` | \`${artifact.wasmHash}\` | ${artifact.deployedAt} |`;
});

const markdown = `# Testnet deployment

- Network: Stellar testnet (\`Test SDF Network ; September 2015\`)
- Administrator: \`${admin}\`
- Deployment source: [Caatinga artifacts](../caatinga.artifacts.json)

| Contract | Address | WASM SHA-256 | Deployed at (UTC) |
| --- | --- | --- | --- |
${rows.join("\n")}

These addresses identify deployed contract instances. Constructor admin binding is
complete, but the protocol contracts still require their \`initialize\` calls and
asset, oracle, and risk configuration before accepting deposits or loans. See
the [Caatinga workflow](caatinga.md) for initialization order.

The private key is never stored in this repository. The deployment script
checks the supplied Stellar CLI identity against the administrator address.
`;

writeFileSync("docs/deployment.md", markdown);
