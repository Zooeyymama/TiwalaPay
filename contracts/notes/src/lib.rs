#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, contractevent,
    symbol_short,
    Address, Env, token,
};

// ── Events (replaces deprecated env.events().publish()) ──────────────────────

#[contractevent]
pub struct FundsLocked {
    pub order_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
}

#[contractevent]
pub struct ItemShipped {
    pub order_id: u64,
    pub seller: Address,
}

#[contractevent]
pub struct FundsReleased {
    pub order_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
}

#[contractevent]
pub struct AutoReleased {
    pub order_id: u64,
    pub seller: Address,
    pub amount: i128,
}

#[contractevent]
pub struct DisputeRaised {
    pub order_id: u64,
    pub buyer: Address,
    pub amount: i128,
}

#[contractevent]
pub struct DisputeResolved {
    pub order_id: u64,
    pub released_to_seller: bool,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Escrow(u64),
    OrderCount,
}

// ── Data types ────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub struct EscrowOrder {
    pub order_id: u64,
    pub buyer: Address,
    pub seller: Address,
    pub amount: i128,
    pub token: Address,
    pub status: EscrowStatus,
    pub created_at: u64,
    pub release_after: u64,
}

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum EscrowStatus {
    Locked,
    Shipped,
    Released,
    Disputed,
    Refunded,
}

// ── Constants ─────────────────────────────────────────────────────────────────

// ~5 days at roughly 10 seconds per ledger
const AUTO_RELEASE_LEDGERS: u64 = 720;

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct SafeHatidContract;

#[contractimpl]
impl SafeHatidContract {

    /// Buyer locks USDC into escrow. Transfers funds from buyer to this
    /// contract and emits a FundsLocked event the seller's UI can react to.
    pub fn lock_funds(
        env: Env,
        buyer: Address,
        seller: Address,
        amount: i128,
        token: Address,
    ) -> u64 {
        buyer.require_auth();

        let order_id: u64 = env.storage().instance()
            .get(&DataKey::OrderCount)
            .unwrap_or(0u64) + 1;
        env.storage().instance().set(&DataKey::OrderCount, &order_id);

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&buyer, &env.current_contract_address(), &amount);

        // Wrap the cast so `<` is never parsed as a generic opener
        let release_after = (env.ledger().sequence() as u64) + AUTO_RELEASE_LEDGERS;

        let order = EscrowOrder {
            order_id,
            buyer: buyer.clone(),
            seller: seller.clone(),
            amount,
            token: token.clone(),
            status: EscrowStatus::Locked,
            created_at: env.ledger().sequence() as u64,
            release_after,
        };
        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&FundsLocked {
            order_id,
            buyer,
            seller,
            amount,
        });

        order_id
    }

    /// Seller confirms they have shipped the item.
    pub fn confirm_shipment(env: Env, seller: Address, order_id: u64) {
        seller.require_auth();

        let mut order: EscrowOrder = env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found");

        assert!(order.seller == seller, "unauthorized");
        assert!(order.status == EscrowStatus::Locked, "wrong status");

        order.status = EscrowStatus::Shipped;
        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&ItemShipped { order_id, seller });
    }

    /// Buyer confirms they received the item. Releases USDC to the seller.
    pub fn confirm_delivery(env: Env, buyer: Address, order_id: u64) {
        buyer.require_auth();

        let mut order: EscrowOrder = env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found");

        assert!(order.buyer == buyer, "unauthorized");
        assert!(
            order.status == EscrowStatus::Shipped
                || order.status == EscrowStatus::Locked,
            "wrong status"
        );

        let token_client = token::Client::new(&env, &order.token);
        token_client.transfer(
            &env.current_contract_address(),
            &order.seller,
            &order.amount,
        );

        order.status = EscrowStatus::Released;
        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&FundsReleased {
            order_id,
            buyer: order.buyer.clone(),
            seller: order.seller.clone(),
            amount: order.amount,
        });
    }

    /// Trustless auto-release after the 5-day window. Anyone can call this —
    /// the ledger sequence check makes it safe.
    pub fn auto_release(env: Env, order_id: u64) {
        let mut order: EscrowOrder = env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found");

        assert!(
            order.status == EscrowStatus::Shipped
                || order.status == EscrowStatus::Locked,
            "wrong status"
        );
        // Parentheses around the cast prevent the generic-argument parse error
        assert!(
            (env.ledger().sequence() as u64) >= order.release_after,
            "too early to auto-release"
        );

        let token_client = token::Client::new(&env, &order.token);
        token_client.transfer(
            &env.current_contract_address(),
            &order.seller,
            &order.amount,
        );

        order.status = EscrowStatus::Released;
        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&AutoReleased {
            order_id,
            seller: order.seller.clone(),
            amount: order.amount,
        });
    }

    /// Buyer raises a dispute before the auto-release window closes.
    pub fn raise_dispute(env: Env, buyer: Address, order_id: u64) {
        buyer.require_auth();

        let mut order: EscrowOrder = env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found");

        assert!(order.buyer == buyer, "unauthorized");
        assert!(
            order.status == EscrowStatus::Shipped
                || order.status == EscrowStatus::Locked,
            "wrong status"
        );
        // Parentheses fix the same generic-argument ambiguity
        assert!(
            (env.ledger().sequence() as u64) < order.release_after,
            "dispute window closed"
        );

        order.status = EscrowStatus::Disputed;
        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&DisputeRaised {
            order_id,
            buyer,
            amount: order.amount,
        });
    }

    /// Admin or DAO resolves a disputed order.
    /// Pass release_to_seller = true to pay the seller, false to refund the buyer.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        order_id: u64,
        release_to_seller: bool,
    ) {
        admin.require_auth();

        let mut order: EscrowOrder = env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found");

        assert!(order.status == EscrowStatus::Disputed, "not in dispute");

        let token_client = token::Client::new(&env, &order.token);

        if release_to_seller {
            token_client.transfer(
                &env.current_contract_address(),
                &order.seller,
                &order.amount,
            );
            order.status = EscrowStatus::Released;
        } else {
            token_client.transfer(
                &env.current_contract_address(),
                &order.buyer,
                &order.amount,
            );
            order.status = EscrowStatus::Refunded;
        }

        env.storage().persistent().set(&DataKey::Escrow(order_id), &order);

        env.events().publish_event(&DisputeResolved {
            order_id,
            released_to_seller: release_to_seller,
        });
    }

    /// Read-only: returns the current state of an order.
    pub fn get_order(env: Env, order_id: u64) -> EscrowOrder {
        env.storage().persistent()
            .get(&DataKey::Escrow(order_id))
            .expect("order not found")
    }
}

mod test;