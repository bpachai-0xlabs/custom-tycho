# Custom Tycho

A Rust application that streams real-time data from Tycho to track Uniswap V2 pool states and calculate swap quotes.

## Features

- Connects to Tycho's Ethereum data stream
- Tracks the USDC/WETH Uniswap V2 pool
- Maintains local state of protocol components, balances, and TVL
- Calculates real-time swap quotes using the constant product formula

## Prerequisites

- Rust (edition 2021 or later)
- Tycho URL (defaults to `tycho-beta.propellerheads.xyz`)
- Tycho authentication token (required)

## Setup

1. Clone the repository:

```bash
git clone https://github.com/bpachai-0xlabs/custom-tycho.git
cd custom-tycho
```

2. Set environment variables (required):

```bash
export TYCHO_URL="tycho-beta.propellerheads.xyz"
export TYCHO_AUTH_TOKEN="your-token-here"
```

3. Build the project:

```bash
cargo build --release
```

## Usage

Run the application:

```bash
cargo run
```

With custom logging:

```bash
RUST_LOG=info cargo run
```

## How It Works

1. Connects to Tycho stream for Uniswap V2 on Ethereum
2. Filters for specific pool (USDC/WETH by default)
3. Receives and processes state updates:
   - Snapshots: Full state of pools
   - Deltas: Incremental updates to balances and TVL
4. Calculates swap quotes for 1 USDC to WETH

## Configuration

Edit `src/main.rs` to customize:

- `filter`: Change the pool address to track different pools
- `amount_in`: Modify the quote calculation amount
- Token addresses: Update for different trading pairs

## Dependencies

- `tycho-client`: Tycho stream client
- `tycho-common`: Common Tycho data types
- `tokio`: Async runtime
- `anyhow`: Error handling
- `tracing-subscriber`: Logging

## License

MIT
