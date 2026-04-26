use soroban_sdk::{contractevent, Address, BytesN, Env};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultCrtEvent {
    #[topic]
    pub commitment: BytesN<32>,
    pub token: Address,
    pub owner: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchedulePayEvent {
    #[topic]
    pub payment_id: u32,
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub amount: i128,
    pub release_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayExecEvent {
    #[topic]
    pub payment_id: u32,
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoSetEvent {
    #[topic]
    pub auto_pay_id: u32,
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub amount: i128,
    pub interval: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoPayEvent {
    #[topic]
    pub auto_pay_id: u32,
    pub from: BytesN<32>,
    pub to: BytesN<32>,
    pub amount: i128,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VaultCancelEvent {
    #[topic]
    pub commitment: BytesN<32>,
    pub refunded_amount: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DepositEvent {
    #[topic]
    pub commitment: BytesN<32>,
    pub amount: i128,
    pub new_balance: i128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WithdrawEvent {
    #[topic]
    pub commitment: BytesN<32>,
    pub amount: i128,
    pub new_balance: i128,
}

#[contractevent]
pub struct AutoCancelEvent {
    #[topic]
    pub rule_id: u32,
    pub from: BytesN<32>,
}

pub struct Events;

impl Events {
    pub fn schedule_pay(
        env: &Env,
        payment_id: u32,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        release_at: u64,
    ) {
        SchedulePayEvent {
            payment_id,
            from,
            to,
            amount,
            release_at,
        }
        .publish(env);
    }

    pub fn pay_exec(env: &Env, payment_id: u32, from: BytesN<32>, to: BytesN<32>, amount: i128) {
        PayExecEvent {
            payment_id,
            from,
            to,
            amount,
        }
        .publish(env);
    }

    pub fn vault_crt(env: &Env, commitment: BytesN<32>, token: Address, owner: Address) {
        VaultCrtEvent {
            commitment,
            token,
            owner,
        }
        .publish(env);
    }

    pub fn auto_set(
        env: &Env,
        auto_pay_id: u32,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        interval: u64,
    ) {
        AutoSetEvent {
            auto_pay_id,
            from,
            to,
            amount,
            interval,
        }
        .publish(env);
    }

    pub fn auto_pay(
        env: &Env,
        auto_pay_id: u32,
        from: BytesN<32>,
        to: BytesN<32>,
        amount: i128,
        timestamp: u64,
    ) {
        AutoPayEvent {
            auto_pay_id,
            from,
            to,
            amount,
            timestamp,
        }
        .publish(env);
    }

    pub fn vault_cancel(env: &Env, commitment: BytesN<32>, refunded_amount: i128) {
        VaultCancelEvent {
            commitment,
            refunded_amount,
        }
        .publish(env);
    }

    pub fn deposit(env: &Env, commitment: BytesN<32>, amount: i128, new_balance: i128) {
        DepositEvent {
            commitment,
            amount,
            new_balance,
        }
        .publish(env);
    }

    pub fn withdraw(env: &Env, commitment: BytesN<32>, amount: i128, new_balance: i128) {
        WithdrawEvent {
            commitment,
            amount,
            new_balance,
        }
        .publish(env);
    }

    pub fn auto_cancel(env: &Env, from: BytesN<32>, rule_id: u32) {
        AutoCancelEvent { rule_id, from }.publish(env);
    }
}
