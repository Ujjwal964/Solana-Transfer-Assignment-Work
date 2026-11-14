# Solana Transfer Extractor

A Rust-based [Substreams](https://substreams.streamingfast.io/) project that extracts native SOL and SPL token transfers from Solana Mainnet blocks and produces structured JSON Lines (JSONL) output.

## 1. Project Overview

The objective of this project is to identify and extract token transfer instructions from Solana blockchain transactions.

The extractor processes the specified Solana Mainnet block range:

- **Start Block:** `385870151`
- **End Block:** `385870156`
- **Network:** Solana Mainnet Beta

For each detected transfer, the output contains:

- `from`: Sender account address.
- `to`: Recipient account address.
- `amount`: Transfer amount represented as an integer.

### Expected Output Format

Each transfer is represented as an independent JSON object on a separate line.

```json
{"from":"SenderAddress","to":"RecipientAddress","amount":325253}
{"from":"SenderAddress","to":"RecipientAddress","amount":1000000}
```

## 2. Supported Transfer Types

The implementation currently handles the following instruction types:

| Program | Instruction | Description |
|---|---|---|
| System Program | Transfer | Native SOL transfer |
| SPL Token | Transfer | SPL token transfer |
| SPL Token | TransferChecked | Checked SPL token transfer |
| Token-2022 | Transfer | Token-2022 transfer |
| Token-2022 | TransferChecked | Checked Token-2022 transfer |

The extractor also processes inner instructions (CPI instructions) associated with top-level transaction instructions.

## 3. Technology Stack

- **Rust** – Core extraction logic and transfer instruction decoding.
- **Substreams** – Blockchain data processing framework.
- **WebAssembly (WASM)** – Compilation target for executing the Rust Substreams module.
- **Pinax** – Free API access to Solana blockchain data through the Substreams endpoint.
- **Solana Mainnet** – Source blockchain.
- **Protocol Buffers** – Structured output definition.
- **Base58** – Solana account address encoding.
- **Buf** – Protocol Buffer code generation.
- **Substreams CLI** – Build and execution.
- **jq** – JSON output transformation.

## 4. Project Structure

```text
solanatransferextractor/
│
├── proto/
│   └── mydata.proto
│
├── src/
│   ├── lib.rs
│   └── pb/
│       └── ...
│
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── README.md
├── buf.gen.yaml
└── substreams.yaml
```

## 5. Protocol Buffer Definition

File: `proto/mydata.proto`

```protobuf
syntax = "proto3";

package mydata.v1;

message MyData {
  repeated Transfer transfers = 1;
}

message Transfer {
  string from = 1;
  string to = 2;
  uint64 amount = 3;
}
```

The `MyData` message contains a collection of transfers.

Each `Transfer` contains sender, recipient, and amount fields.

The amount uses `uint64` to preserve the full unsigned 64-bit integer range in the Protocol Buffer representation.

## 6. Substreams Configuration

File: `substreams.yaml`

The manifest defines the package, Solana import, protobuf configuration, WASM binary, modules, network, and program filters.

Important configuration:

```yaml
modules:
  - name: map_filtered_transactions
    use: solana:transactions_by_programid_without_votes
    initialBlock: 385870151

  - name: map_my_data
    kind: map
    inputs:
      - map: map_filtered_transactions
    output:
      type: proto:mydata.v1.MyData

network: solana-mainnet-beta
```

### Program Filtering

The imported module is configured to filter transactions associated with:

| Program | Program ID |
|---|---|
| System Program | `11111111111111111111111111111111` |
| SPL Token | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| Token-2022 | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |

The program IDs are also used in the Rust extraction logic to identify and decode transfer instructions.

## 7. Extraction Logic

File: `src/lib.rs`

The extraction process follows these steps:

1. Receive filtered Solana transactions.
2. Access the transaction message and metadata.
3. Ignore failed transactions.
4. Construct the complete account-key list, including address lookup table accounts.
5. Iterate through top-level instructions.
6. Decode supported transfer instructions.
7. Process associated inner instructions.
8. Resolve sender and recipient account addresses.
9. Store extracted transfers in the `MyData` protobuf message.

### Native SOL Transfer

The System Program transfer instruction uses:

- A four-byte little-endian instruction discriminator.
- An eight-byte little-endian amount in lamports.

The implementation recognizes discriminator `2`.

### SPL Token Transfer

The SPL Token instruction data is decoded using:

- Instruction type `3` for Transfer.
- Instruction type `12` for TransferChecked.

The amount is extracted as an unsigned 64-bit little-endian integer.

### Account Address Encoding

Solana account public keys are stored as byte arrays in transaction data.

The `bs58` crate converts these byte arrays into Base58-encoded Solana addresses.

### Inner Instructions

The extractor processes inner instructions associated with their corresponding top-level instruction index.

This allows transfers invoked through cross-program invocations to be included.

## 8. Prerequisites

Install the following tools:

- Rust and Cargo
- Substreams CLI
- Buf CLI
- jq
- A Pinax account and Substreams API key

The project targets the `wasm32-unknown-unknown` compilation target.

The Substreams endpoint used is:

```text
solana.substreams.pinax.network:443
```

### API Key Configuration

Set the API key as an environment variable:

```bash
export SUBSTREAMS_API_KEY="YOUR_API_KEY"
```

Do not commit API keys or other credentials to GitHub.

## 9. Build Instructions

Navigate to the project directory:

```bash
cd solanatransferextractor
```

Build the Substreams package:

```bash
substreams build
```

The build generates:

```text
solanatransferextractor-v0.1.0.spkg
```

## 10. Run Instructions

The manifest defines the initial block as `385870151` already configured in yaml.

Execute the extractor for six blocks:

```bash
substreams run -e solana.substreams.pinax.network:443 solanatransferextractor-v0.1.0.spkg map_my_data -t +6 -o jsonl | jq -c '.. | objects | select(has("from") and has("to") and has("amount")) | {from, to, amount: (.amount | tonumber)}'

```

## 11. Output Example

```json
{"from":"3nnVbsCfN1mwUk2XSLCnjzY3bDDTdzpnKjvRmd8nESS2","to":"5QayzJL4uyauHMaKsgdiGZ7hxVZ1KmzLBbEpbYDBqi4S","amount":1000300019}
```

The output is JSON Lines (JSONL), meaning each transfer is represented on an individual line.

## 12. Important Implementation Notes

- Failed transactions are excluded.
- Static account keys and loaded address lookup table accounts are combined.
- Top-level and inner instructions are inspected.
- Native SOL amounts are expressed in lamports.
- SPL token amounts are currently expressed in raw token base units, not decimal-adjusted display amounts.
- SPL sender and recipient fields currently contain token-account addresses, not necessarily the wallet-owner addresses.
- Only the transfer instruction variants explicitly implemented in the Rust code are currently extracted.

## 13. Security and Configuration

- Keep the Substreams API key in an environment variable.
- Do not commit `.env` files or credentials.
- Keep generated build directories out of version control.
- The Rust program IDs and YAML filters must remain consistent.

## 14. Current Project Status

The project currently has:

- [x] Rust Substreams module.
- [x] Solana Mainnet configuration.
- [x] Configured initial block.
- [x] Native SOL transfer extraction.
- [x] SPL Token transfer extraction.
- [x] Token-2022 transfer extraction.
- [x] Top-level instruction processing.
- [x] Inner/CPI instruction processing.
- [x] Base58 account address encoding.
- [x] Protobuf output structure.
- [x] WASM build.
- [x] Substreams package generation.
- [x] Six-block extraction run.

---

## 15. Project Information

| Detail | Information |
|---|---|
| **Project** | Solana Transfer Extractor |
| **Language** | Rust |
| **Framework** | Substreams |
| **Network** | Solana Mainnet Beta |
| **Block Range** | 385870151–385870156 |
| **Developed By** | Ujjwal Bansal |
|**GitHub Repository** | [Link](https://github.com/Ujjwal964/Solana-Transfer-Assignment-Work) |
| **GitHub** | [Ujjwal964](https://github.com/Ujjwal964) |
| **LinkedIn** | [linkedin.com/in/ujjwalbansall](https://www.linkedin.com/in/ujjwalbansall) |
