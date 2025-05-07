# 🧾 Technical Specification – DIAMOND Token (Anchor Smart Contract)
- reduce stack usage,
- handle all checked math operations,
- log events with `emit!`,
- respect multisig-based authority verification.


Perform modularization and avoid code duplication. PDA constraints and seeds must be respected.

Main goal compile project to local without a errors

---

## 🎯 Goal

Create a token contract with:

- Limited emission
- Fixed mint price
- Built-in control logic (`pause`, `multisig`, `blacklist`)
- Premint capability
- Admin token burn (with refund)
- On-chain purchases via website
- Proof-of-Reserve support
- Optimized stack usage

---

## 🔧 Functions (Each must be implemented separately)

### 1. `mint_by_user`

- User pays USDT, USDC, SOL, or another token via TokenAccount (on the website).
- Fixed price: **0.8 USDT per token**
- Checks: amount >= MIN_PURCHASE_USDT
- Payment is converted to USDT and transferred to a **PDA vault** (on-chain reserve).
- Must verify: `decimals == 6` for USDT/USDC and for the token.

---

### 2. `admin_burn` *(⚠️ Heavy stack — must be optimized!)*

- Admin can burn tokens from premint or PDA vault.
- Returns equivalent value in USDT.
- Executable **only via SPL multisig (3 of 5)**.
- User does NOT call this directly (they sell via DEX or use `purchase_item`).

---

### 3. `pause / unpause`

- `pause()` blocks minting.
- `unpause()` can only be called **15 minutes after** the last `pause()`.
- Last pause timestamp is stored in `TokenState`.
- Only callable via SPL multisig (3 of 5).

---

### 4. `update_max_supply`

- Allows **decreasing** `MAX_SUPPLY` (never increase).
- Only via SPL multisig (3 of 5).

---

### 5. `add_to_blacklist / remove_from_blacklist`

- Only via SPL multisig (3 of 5).
- Adds/removes addresses from blacklist.
- Blacklisted users **cannot mint**.
- Can be used in `transfer_hook`.

---

### 6. `on_transfer_hook`

- Based on **SPL Token-2022** standard.
- Prevents token transfers **between blacklisted addresses**.

---

### 7. `purchase_item`

- User sends tokens (via website) to PDA vault.
- Used to purchase goods (e.g. jewelry).
- Admin can later burn these tokens and refund USDT via `admin_burn`.

---

## 📦 Token Supply

- `INITIAL_SUPPLY`: 8,000,000 tokens → assigned to admin or vault at init.
- `MAX_SUPPLY`: 100,000,000 tokens → hard limit, cannot be increased.
- `TOTAL_SUPPLY`: 
  - Increases via `mint_by_user`
  - Decreases via `admin_burn`
- Stored in `TokenState`

---

## 🔍 Proof-of-Reserve

All payments go to **on-chain PDA vault**.
Anyone can verify USDT balance on-chain.

---

## 💰 Fees

- Buy fee: **0%**
- Sell fee: **0%**

---

## 🛡 Security

- All calculations must use:
  - `checked_add`, `checked_sub`, `checked_mul`, `saturating_sub`
- Use `require!`, `require_keys_eq!` for validations
- Always validate: `decimals == 6`

---

## 📢 Logging

- Emit events (`emit!`) for: mint, burn, pause, unpause, blacklist changes, purchases.
- Use `msg!` for internal validation status messages.