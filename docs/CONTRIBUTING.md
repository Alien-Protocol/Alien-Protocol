# Contributing to Alien Protocol

Thank you for helping build Alien Protocol. Contributions can include contract features, bug fixes, tests, documentation, CI improvements, or thoughtful issue reports.

This guide explains the expected workflow so contributions stay focused, reviewable, and safe for a smart-contract codebase.

## Before you begin

1. Search the [open issues](https://github.com/Alien-Protocol/Alien-Protocol/issues) and pull requests to avoid duplicate work.
2. For a small fix, open or claim an issue and describe your intended change.
3. For a new feature, contract interface change, or storage change, open an issue and agree on the design before writing code.
4. Never report an exploitable vulnerability in a public issue. Follow the security guidance in the [README](../README.md#-security).

## Development setup

You need:

- [Rust](https://www.rust-lang.org/tools/install) with `rustup`
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli)
- Git

Fork the repository on GitHub, then clone your fork:

```bash
git clone https://github.com/<your-username>/Alien-Protocol.git
cd Alien-Protocol
git remote add upstream https://github.com/Alien-Protocol/Alien-Protocol.git
rustup target add wasm32v1-none
```

Confirm that the required tools are available:

```bash
rustc --version
stellar --version
```

Run the existing test suite before making changes:

```bash
cargo test --workspace --all-features
```

## Branch naming

Create every change on a new branch from the latest `main`:

```bash
git checkout main
git pull --ff-only upstream main
git checkout -b feat/add-repayment-flow
```

Use this format:

```text
type/short-kebab-case-description
```

If the change has a GitHub issue, include its number:

```text
type/123-short-kebab-case-description
```

| Type | Use it for | Example |
| --- | --- | --- |
| `feat` | New protocol behavior | `feat/123-add-repayment-flow` |
| `fix` | Bug or security fix | `fix/validate-stale-price` |
| `docs` | Documentation only | `docs/improve-contribution-guide` |
| `test` | New or corrected tests | `test/vault-withdrawal-boundaries` |
| `refactor` | Internal change without new behavior | `refactor/oracle-storage-helpers` |
| `perf` | Performance or resource optimization | `perf/reduce-vault-storage-reads` |
| `ci` | GitHub Actions or automation | `ci/cache-rust-builds` |
| `chore` | Maintenance or dependency work | `chore/update-soroban-sdk` |

Keep branch names lowercase, short, and specific. One branch should address one issue or one closely related change.

## Commit messages

Alien Protocol uses [Conventional Commits](https://www.conventionalcommits.org/):

```text
type(scope): imperative summary
```

The scope is optional, but useful scopes include `vault`, `pool`, `oracle`, `liquidation`, `shared`, `tests`, `docs`, and `ci`.

Good examples:

```text
feat(pool): add repayment accounting
fix(oracle): reject future price timestamps
test(vault): cover partial withdrawal boundaries
docs(contributing): document branch conventions
ci(contracts): build with Stellar CLI
```

Commit-message rules:

- Use an allowed type: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`, or `revert`.
- Write the summary in the imperative mood: `add`, `fix`, `prevent`, or `update`.
- Keep the first line concise, ideally no more than 72 characters.
- Do not end the summary with a period.
- Explain the reason and important tradeoffs in the body when the title is not enough.
- Reference related issues in the footer, for example `Closes #123`.

Mark an intentional breaking change with `!` and explain the impact:

```text
feat(pool)!: change the repayment interface

BREAKING CHANGE: repay now requires the borrower address.
```

Avoid vague messages such as `update code`, `fix issue`, or `changes`.

## What to keep in mind

Smart-contract changes can affect user funds and persistent on-chain state. Review these points while developing:

- **Keep the scope focused.** Avoid unrelated formatting, renaming, or refactoring in the same pull request.
- **Require the correct authorization.** Verify every privileged or user-specific operation with the appropriate Soroban authorization check.
- **Validate every input.** Cover zero and negative amounts, duplicate initialization, unsupported assets, missing prices, and invalid addresses where relevant.
- **Use checked arithmetic.** Consider overflow, underflow, precision, rounding direction, and boundary values in financial calculations.
- **Treat oracle data as untrusted.** Validate freshness, timestamps, supported feeds, signers, and threshold rules.
- **Preserve storage compatibility.** Discuss changes to storage keys, types, contract interfaces, or event schemas before implementation.
- **Emit meaningful events.** State changes that off-chain consumers need to follow should produce stable, testable events.
- **Keep contracts `no_std` compatible.** Do not introduce dependencies or APIs that cannot compile to Soroban WASM.
- **Add tests with the change.** Include success, authorization failure, invalid input, boundary, and regression cases as appropriate.
- **Never commit secrets or generated artifacts.** Keep private keys, local identities, contract IDs, `.env` files, and `target/` output out of Git.

## Build and verify

Format and lint the workspace:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --workspace
```

Run all native contract tests:

```bash
cargo test --workspace --all-features
```

Build and optimize all Soroban contract WASM files with Stellar CLI:

```bash
stellar contract build --locked
```

The build output is written to `target/wasm32v1-none/release/`. All checks should pass without new warnings before you open a pull request.

## Pull requests

Push your branch to your fork:

```bash
git push -u origin feat/add-repayment-flow
```

Open a pull request into `Alien-Protocol/Alien-Protocol:main` and complete the repository's pull-request template.

Before requesting review, confirm that:

- [ ] The pull request addresses one focused issue or feature.
- [ ] The description explains what changed and why.
- [ ] The related issue is linked with `Closes #<issue-number>` when applicable.
- [ ] Tests cover the new behavior and important failure paths.
- [ ] Formatting, Clippy, checks, tests, and the Stellar contract build pass locally.
- [ ] Contract interface, storage, event, and authorization changes are clearly called out.
- [ ] No secrets, local configuration, or build artifacts are included.
- [ ] Evidence is attached when the change has visible or reproducible behavior.

Keep the branch up to date while it is under review:

```bash
git fetch upstream
git rebase upstream/main
git push --force-with-lease
```

Use `--force-with-lease` only for your own contribution branch. Never rewrite shared branches such as `main`.

## Review process

Maintainers may request changes for correctness, security, test coverage, compatibility, or scope. Reply to feedback, push follow-up commits, and resolve conversations only after the concern has been addressed.

Once the required checks pass and the review is approved, a maintainer can merge the pull request.
