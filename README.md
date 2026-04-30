# SafeHatid

**Trustless escrow for Filipino online buyers and sellers — no more COD anxiety.**

---

## Problem
Filipino online buyers refuse to pay upfront to unknown sellers out of fear of scams,
while sellers won't ship without payment — killing countless transactions every day
on Facebook Marketplace, Carousell, and Instagram shops.

## Solution
Buyers lock USDC into a Soroban escrow contract the moment they confirm an order.
Sellers see the funds are locked on-chain and ship with confidence. When the buyer
confirms delivery (or after 5 days with no response), the contract automatically
releases payment to the seller. No bank. No GCash dispute. No ghosting.

---

## MVP Timeline

| Day | Milestone |
|-----|-----------|
| 1   | Contract compiled and deployed to testnet |
| 2   | lock_funds + confirm_delivery flow working end-to-end |
| 3   | Frontend: buyer deposit screen + seller shipment screen |
| 4   | Auto-release timer wired up, dispute button added |
| 5   | Demo polish, testnet walkthrough recorded |

---

## Stellar Features Used
- **Soroban smart contracts** — conditional escrow state machine
- **USDC transfers** — real stablecoin, not a fictional token
- **XLM** — pays network fees (near-zero cost, buyer never notices)
- **Trustlines** — buyer holds USDC before locking funds

---

## Prerequisites

- Rust toolchain: `rustup target add wasm32-unknown-unknown`
- Soroban CLI v22+: `cargo install --locked soroban-cli`
- A funded Stellar testnet account (use [Stellar Lab](https://laboratory.stellar.org))

---

## Build

````bash
soroban contract build
````

Output: `target/wasm32-unknown-unknown/release/safehatid.wasm`

---

## Test

````bash
cargo test
````

---

## Deploy to Testnet

````bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/safehatid.wasm \
  --source YOUR_SECRET_KEY \
  --network testnet
````

---

## Sample CLI Invocations

**Lock funds (buyer deposits 500 USDC):**
````bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source BUYER_SECRET_KEY \
  --network testnet \
  -- lock_funds \
  --buyer GBUYER... \
  --seller GSELLER... \
  --amount 500 \
  --token USDC_CONTRACT_ID
````

**Confirm delivery (buyer taps "Item Received"):**
````bash
soroban contract invoke \
  --id CONTRACT_ID \
  --source BUYER_SECRET_KEY \
  --network testnet \
  -- confirm_delivery \
  --buyer GBUYER... \
  --order_id 1
````

**Read order state:**
````bash
soroban contract invoke \
  --id CONTRACT_ID \
  --network testnet \
  -- get_order \
  --order_id 1
````

---

## Deployed Contract Link

🔗 https://stellar.expert/explorer/testnet/tx/825cd546829c89f894f7db1a1d4eadc50e09dba3fb39e2b3c9e502591e3e0a8f
🔗 https://lab.stellar.org/r/testnet/contract/CDP6SBLJSWTZQJQABOWONE4QTNCNX2RRGZJNNP7UBAJV24F5PXSZ6ZXT

## License

MIT