# lst_lending_amm

## 📖 **Overview**
LST Lending AMM is a Solana-based decentralized lending and borrowing program(protocol) built with the Anchor framework. It allows users to:

- Deposit Liquid Staking Tokens (LST) as collateral.
- Borrow assets against LST collateral.
- Perform partial liquidations on under-collateralized positions.
- Utilize oracle price feeds from Pyth and Switchboard for accurate asset pricing.

This program(protocol) is designed to ensure efficiency, scalability, and security while maintaining capital efficiency for lenders and borrowers.

## License
This project is under the **MIT LICENSE**


## 🚀 **Features**

1. **Deposit Collateral:**
   - Users can deposit their LST tokens as collateral into the protocol.
   - Tracks user collateral balance and protocol-level stats.

2. **Borrow Assets:**
   - Borrowers can take out loans based on their LST collateral.
   - Supports dynamic interest rates based on pool utilization.

3. **Liquidate Under-Collateralized Positions:**
   - Partial liquidations are supported to minimize protocol risk.
   - Liquidators earn bonuses for successful liquidation.

4. **Oracle Integration:**
   - Real-time asset pricing through Pyth and Switchboard price feeds.

5. **Dynamic Interest Rates:**
   - Interest rates adjust dynamically based on pool utilization.

---

## 🛠️ **Technology Stack**

- **Blockchain:** Solana
- **Smart Contract Framework:** Anchor
- **Oracles:** Pyth, Switchboard
- **Languages:** Rust and Typescript
- **Token Standard:** SPL Tokens
- **Solana Playground**

## 📑 **Program Instructions**

### 🏦 **1. Deposit Collateral**
```rust
pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()>
```
- Transfers LST tokens from the user account to the collateral vault.
- Updates user collateral balance and protocol stats.

**Event:** `CollateralDeposited`

---

### 💳 **2. Borrow Assets**
```rust
pub fn borrow(ctx: Context<Borrow>, borrow_amount: u64) -> Result<()>
```
- Verifies sufficient collateral value based on oracle price feeds.
- Calculates interest dynamically based on utilization.
- Transfers borrowed tokens to the user.

**Event:** `AssetBorrowed`

---

### ⚠️ **3. Liquidate Under-Collateralized Positions**
```rust
pub fn liquidate(ctx: Context<Liquidate>, repay_amount: u64) -> Result<()>
```
- Checks if the position is under-collateralized.
- Swaps assets and reduces borrower debt.
- Liquidator receives a liquidation bonus.

**Event:** `PositionLiquidated`

---

### 📊 **4. Fetch Oracle Price**
```rust
pub fn get_price_instruction(ctx: Context<GetPrice>) -> Result<u64>
```
- Fetches real-time price data from Pyth and Switchboard.
- Ensures accurate pricing for borrow and liquidation logic.

---

## 📦 **Accounts**

### **User Accounts:**
- `CollateralAccount`: Tracks user collateral deposits.
- `DebtAccount`: Tracks user borrow amounts.

### **Protocol Accounts:**
- `ProtocolStats`: Aggregates global collateral and borrow stats.
- `GlobalState`: Manages protocol pause and admin state.

---

## 📡 **Error Handling**

- `InsufficientCollateral`: Not enough collateral to borrow.
- `PositionStillSafe`: Position is not liquidatable.
- `ProtocolPaused`: Protocol is in paused state.
- `InvalidOracle`: Oracle feed issue.

---

## 📈 **Dynamic Interest Rate Model**
- **Utilization < 80%:** Base interest rate = 5%
- **Utilization ≥ 80%:** Interest increases dynamically

**Formula:**
let interest_rate = if utilization < 80 { 5 } else { 10 + (utilization - 80) };
```

---

## 📢 **Events**
- **CollateralDeposited:** Logs user collateral deposits.
- **AssetBorrowed:** Logs borrowing activity.
- **PositionLiquidated:** Logs liquidation events.

---


---

