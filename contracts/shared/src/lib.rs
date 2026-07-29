//! Shared protocol primitives for Alien Protocol.
//!
//! Import this crate to access canonical cross-contract types, errors, and
//! interface definitions without pulling in individual contract crates.
//!
//! ```text
//! shared::types::PriceData
//! shared::errors::ProtocolError
//! shared::interfaces::LendingPoolClient
//! ```

#![no_std]

pub mod constant;
pub mod errors;
pub mod events;
pub mod interfaces;
pub mod types;

pub use errors::ProtocolError;
