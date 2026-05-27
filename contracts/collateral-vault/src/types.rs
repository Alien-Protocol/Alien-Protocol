use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DataKey {
    Admin,
    Paused,
    SupportedAsset(Address),
    Position(Address, Address), // (user, asset)
    PositionIndex,
}

