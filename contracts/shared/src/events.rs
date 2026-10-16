//! Shared event topics and documentation for indexers.
///
/// This module defines the canonical event topic names that off-chain indexers and
/// consumers should rely on when filtering or decoding contract events. Event
/// topics are exposed as string literals so they can be used both in on-chain
/// contracts (via compile-time constants) and by off-chain tooling without adding
/// a dependency on the contract crates themselves.

/// Emitted when a new contract is initialized.
public const INITIALIZED: &str = "initialized";

/// Emitted when a contract is upgraded to a new implementation.
public const UPGRADED: &str = "upgraded";

/// Emitted when a contract is paused.
public const PAUSED: &str = "paused";

/// Emitted when a contract is unpaused.
public const UNPAUSED: &str = "unpaused";

/// Emitted when a contract is permanently deactivated.
public const DEACTIVATED: &str = "deactivated";

/// Emitted when a proposal is created.
public const PROPOSAL_CREATED: &str = "proposal_created";

/// Emitted when a proposal is executed.
public const PROPOSAL_EXECUTED: &str = "proposal_executed";

/// Emitted when a proposal is cancelled.
public const PROPOSAL_CANCELLED: &str = "proposal_cancelled";

/// Emitted when a vote is cast on a proposal.
public const VOTE_CAST: &str = "vote_cast";

/// Emitted when a vote is withdrawn from a proposal.
public const VOTE_WITHDRAWN: &str = "vote_withdrawn";

/// Emitted when a token transfer occurs.
public const TRANSFER: &str = "transfer";

/// Emitted when a token approval is granted.
public const APPROVAL: &str = "approval";

/// Emitted when a token mint occurs.
public const MINT: &str = "mint";

/// Emitted when a token burn occurs.
public const BURN: &str = "burn";
