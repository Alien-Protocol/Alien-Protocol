# Testnet deployment

- Network: Stellar testnet (`Test SDF Network ; September 2015`)
- Administrator: `GCJGOM2HPJGHEAGVFGIFN7SVTGWOZIH27E7JL7Q4D4VVIC4NZ7UK2VRK`
- Deployment source: [Caatinga artifacts](../caatinga.artifacts.json)

| Contract | Address | WASM SHA-256 | Deployed at (UTC) |
| --- | --- | --- | --- |
| deployment-factory | `CBPT6ZGCIXA2L2H6VWVFELVWJOAAKSQ7S6Q5JL2QOUNUZQEADPPPEG33` | `641a4af2f6d367c8018c06bc12703892040d7233296443117a3afdeeb1eb9dca` | 2026-11-28T09:34:25.679Z |
| oracle-adapter | `CA3A7ZB7YXWWMNROIWAM6GX74HQPKSST6RQHAESRWI37KVOZG7SDJ5AU` | `ec58bdb3c90fcee2154e3e620ca34012718ddd72b3cdced5f65d92533570d973` | 2026-11-28T09:34:41.039Z |
| lending-pool | `CB7QKTIZLPENLBHHHBGDNPSWRQYZ4TJ2CNCCX3LZVDZGW4FZSWZZQERN` | `8dd3cfde4e13160edf51e3983a6e3a5e1263bf8ac4cac443d36f24abbb9903ce` | 2026-11-28T09:34:50.148Z |
| collateral-vault | `CAS6NDY4D5QTCRWLT3AUJIIEJTTJKXQU4WZ7OYQ6N2WQPJZPR3RVOLOY` | `bca0b33cd16eab333255a7294035645676b8929ef6da545d5f80ecfba3c9e321` | 2026-11-28T09:35:06.225Z |
| liquidation-engine | `CC2FQTPDJLZS3SWS73EHPXVCPJOHRANT24GRYH4MZDH727MT6YXXXL6A` | `cd8783fdfb8b34e872cd5525e4ab04a3398a45b77809f8eac90b961ba2de5e74` | 2026-11-28T09:35:21.583Z |

These addresses identify deployed contract instances. Constructor admin binding is
complete, but the protocol contracts still require their `initialize` calls and
asset, oracle, and risk configuration before accepting deposits or loans. See
the [Caatinga workflow](caatinga.md) for initialization order.

The private key is never stored in this repository. The deployment script
checks the supplied Stellar CLI identity against the administrator address.
