# 🚀🚀🚀 Jito Shredstream Pumpfun Sniper 🚀🚀🚀

This project is a sniping bot built on top of the open-source Jito Shredstream Client. It leverages Jito Shredstream as a real-time data source to identify and snipe profitable trading opportunities on Solana by analyzing low-tip transactions and executing automated swaps.

## 🚧 Test Examples
https://solscan.io/tx/4K8m2yAa2Ya2xUn3Shu3sUQWosDbG8HwPmRtnGkVxeRbxbyjBpyFUxjvsfzs2Njr17JhSkyBxZkLrhJNWe3fqJ4Q
https://solscan.io/tx/4NKZaSeSM4Uwkq3EFMpc1v9bskCuZaLLmThPGg3NDmSHQHxoQk5754BYmPex8bPudQx2xBxTBJXK9NWcJHuw2za5
https://solscan.io/tx/2A4P5r3cHBZtNFr1q9v3c3oTEwtHFCxeviDqGhFraRrZB3k4wBfQ8tKcFVFXGH5ztmKdUBXtQ38bYMN4TvKfAV93
https://solscan.io/tx/3KU1LEQ5QKdzrmEASVwm5iBxjzcDQBVLFW2fGzj1MMLUWEmqETMjZjdEL7VztDymhMKCN5WgUbiZGG9bNZYpiqzg

## 🚀 Features
✔ <b>Real-time transaction monitoring via Jito Shredstream</b>
<br>
✔ <b>Low-tip transaction detection (configurable threshold)</b>
<br>
✔ <b>Automated sniping with buy & sell strategies</b>
<br>
✔ <b>Redis caching for performance optimization</b>
<br>
✔ <b>Customizable parameters (price range, tip limits, delays)</b>
<br>

## ⚙️ Setup
Prerequisites
Rust (v1.70+ recommended)

Redis (for caching)

Solana CLI (optional, for wallet management)

## Installation

### Clone the repo:

```bash
git clone https://github.com/hodlwarden/jito-shredstream-pumpfun-sniper.git
cd jito-shredstream-pumpfun-sniper
```

### Build the project:

```bash
cargo build --release
```

## 🔧 Configuration
Create a .env file in the root directory with the following variables:

```
# Jito Shredstream Endpoint (default: local Jito validator)
SERVER_URL="http://127.0.0.1:9999"

# Solana RPC (use a private RPC for lower latency)
RPC_URL="https://api.mainnet-beta.solana.com"

# Wallet Private Key (Base58)
PRIVATE_KEY="your_private_key_here"

# Redis Cache (improves performance)
REDIS_URL="redis://127.0.0.1:6379"

# Trading Parameters
MIN_SOL_PRICE="0.5"    # Min token price (SOL) to snipe
MAX_SOL_PRICE="3.0"    # Max token price (SOL) to snipe
BUY_SOL_AMOUNT="0.1"   # SOL amount per snipe
SELL_DELAY_MS="5000"    # Delay before selling (ms)
MAX_TIP_LAMPORTS="10000" # Max tip allowed (lamports)
```

## 🚀 Usage

### Run the Sniper
```
cargo run --release
```
## Custom Strategies
Modify src/utils/auto_trader.rs to adjust:

Buy conditions (e.g., liquidity thresholds)

Sell timing (dynamic delays, trailing stops)

Tip filtering (aggressive vs. conservative)

## ⚡ How It Works
Listens to Jito Shredstream for new transactions.

Detects low-tip transactions (below MAX_TIP_LAMPORTS).

Analyzes swaps for tokens in the configured price range (MIN_SOL_PRICE to MAX_SOL_PRICE).

Executes buys with BUY_SOL_AMOUNT.

Automatically sells after SELL_DELAY_MS.

## 📌 Notes & Warnings
⚠ High-risk trading: Sniping can lead to losses (slippage, rugs, failed TXs).
⚠ Use a dedicated wallet: Avoid using your main wallet.
⚠ Optimize RPC: A private RPC reduces latency (essential for sniping).

## 🔄 Advanced Optimizations
Run on a low-latency server (e.g., AWS/GCP in us-west-1).

Preload token metadata in Redis to speed up lookups.

Adjust gas fees dynamically based on network congestion.

## 🤝 Contributing
Contributions are welcome! Please open an issue or pull request for any improvements.
Feel free to reach out me for any suggestions and questions, you're always welcome.
<br>
Telegram - [Hodlwarden](https://t.me/hodlwarden)
