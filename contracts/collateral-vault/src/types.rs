use soroban_sdk::contracttype;

// Datakey's

#[contracttype]
pub enum Datakey {
    Admin,
    LendingPool,
}
