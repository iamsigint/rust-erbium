# 🧪 Erbium Node - API Testing Guide

This document contains curl commands to test all Erbium Node APIs, including REST, RPC, and WebSocket.

## 📋 API Configuration

By default, the ports are:
- **REST API**: `http://localhost:8080`
- **RPC API**: `http://localhost:8545`
- **WebSocket**: `ws://localhost:8546`

## 🐧 Linux/Mac Commands

### REST API - Basic Information

```bash
# Node information
curl -X GET "http://localhost:8080/node"

# Blockchain information
curl -X GET "http://localhost:8080/chain"

# Health check
curl -X GET "http://localhost:8080/health"
```

### REST API - Blocks

```bash
# List recent blocks
curl -X GET "http://localhost:8080/blocks"

# Get specific block by hash
curl -X GET "http://localhost:8080/blocks/0xblock1"
```

### REST API - Transactions

```bash
# List recent transactions
curl -X GET "http://localhost:8080/transactions"

# Get specific transaction by hash
curl -X GET "http://localhost:8080/transactions/0xtx1"

# Send simple transaction (transfer)
curl -X POST "http://localhost:8080/transactions" \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_type": "transfer",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "amount": 1000,
    "fee": 10,
    "nonce": 1
  }'

# Send stake transaction
curl -X POST "http://localhost:8080/transactions" \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_type": "stake",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "amount": 10000,
    "fee": 10,
    "nonce": 2
  }'

# Send confidential transaction (with ZK proof)
curl -X POST "http://localhost:8080/transactions" \
  -H "Content-Type: application/json" \
  -d '{
    "transaction_type": "confidential_transfer",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "amount": 500,
    "fee": 10,
    "nonce": 3,
    "zk_proof": "a1b2c3...",
    "input_commitments": ["abc123...", "def456..."],
    "output_commitments": ["ghi789...", "jkl012..."],
    "range_proofs": ["mno345...", "pqr678..."],
    "binding_signature": "stu901..."
  }'
```

### REST API - Validators

```bash
# List validators
curl -X GET "http://localhost:8080/validators"
```

### REST API - Bridges (Cross-chain)

```bash
# List available bridges
curl -X GET "http://localhost:8080/bridges"

# List bridge transfers
curl -X GET "http://localhost:8080/bridges/transfers"

# Get specific transfer
curl -X GET "http://localhost:8080/bridges/transfers/transfer_001"

# Initiate bridge transfer
curl -X POST "http://localhost:8080/bridges/transfers" \
  -H "Content-Type: application/json" \
  -d '{
    "source_chain": "bitcoin-mainnet",
    "target_chain": "erbium-mainnet",
    "amount": 1000000,
    "asset_id": "BTC",
    "sender": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "recipient": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f"
  }'
```

### RPC API - Ethereum-compatible Methods

```bash
# Get latest block number
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Get block by hash
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBlockByHash","params":["0xblock1", false],"id":2}'

# Get block by number
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBlockByNumber","params":["latest", false],"id":3}'

# Get transaction by hash
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getTransactionByHash","params":["0xtx1"],"id":4}'

# Get account balance
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0x742d35Cc6634C0532925a3b844Bc454e4438f44e", "latest"],"id":5}'

# Send transaction
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc":"2.0",
    "method":"eth_sendTransaction",
    "params":[{
      "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
      "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
      "value": "0x3e8",
      "gas": "0x5208",
      "gasPrice": "0x4a817c800"
    }],
    "id":6
  }'

# Get network version
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"net_version","params":[],"id":7}'

# Get client version
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"web3_clientVersion","params":[],"id":8}'

# Get chain information (Erbium specific)
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"erb_chainInfo","params":[],"id":9}'

# Get chain ID
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":10}'

# Get gas price
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_gasPrice","params":[],"id":11}'

# Estimate gas for transaction
curl -X POST "http://localhost:8545" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_estimateGas","params":[{"to":"0x742d35Cc6634C0532925a3b844Bc454e4438f44f","value":"0x3e8"}],"id":12}'
```

### WebSocket - Tests

```bash
# Install websocat (Linux/Mac)
curl -L https://github.com/vi/websocat/releases/latest/download/websocat.x86_64-unknown-linux-musl -o websocat
chmod +x websocat

# Connect to WebSocket
websocat ws://localhost:8546/ws

# After connecting, send JSON messages:
{
  "type": "subscribe",
  "data": {
    "topics": ["blocks", "transactions", "validators"]
  }
}

{
  "type": "ping",
  "data": {}
}

{
  "type": "unsubscribe",
  "data": {
    "topics": ["blocks"]
  }
}
```

## 🪟 Windows Commands (PowerShell)

### REST API - Basic Information

```powershell
# Node information
curl.exe -X GET "http://localhost:8080/node"

# Blockchain information
curl.exe -X GET "http://localhost:8080/chain"

# Health check
curl.exe -X GET "http://localhost:8080/health"
```

### REST API - Blocks

```powershell
# List recent blocks
curl.exe -X GET "http://localhost:8080/blocks"

# Get specific block by hash
curl.exe -X GET "http://localhost:8080/blocks/0xblock1"
```

### REST API - Transactions

```powershell
# List recent transactions
curl.exe -X GET "http://localhost:8080/transactions"

# Get specific transaction by hash
curl.exe -X GET "http://localhost:8080/transactions/0xtx1"

# Send simple transaction (transfer)
curl.exe -X POST "http://localhost:8080/transactions" `
  -H "Content-Type: application/json" `
  -d '{
    "transaction_type": "transfer",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "amount": 1000,
    "fee": 10,
    "nonce": 1
  }'

# Send stake transaction
curl.exe -X POST "http://localhost:8080/transactions" `
  -H "Content-Type: application/json" `
  -d '{
    "transaction_type": "stake",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "amount": 10000,
    "fee": 10,
    "nonce": 2
  }'

# Send confidential transaction (with ZK proof)
curl.exe -X POST "http://localhost:8080/transactions" `
  -H "Content-Type: application/json" `
  -d '{
    "transaction_type": "confidential_transfer",
    "from": "0x742d35Cc6634C0532925a3b844Bc454e4438f44e",
    "to": "0x742d35Cc6634C0532925a3b844Bc454e4438f44f",
    "amount": 500,
    "fee": 10,
    "nonce": 3,
    "zk_proof": "a1b2c3...",
    "input_commitments": ["abc123...", "def456..."],
    "output_commitments": ["ghi789...", "jkl012..."],
    "range_proofs": ["mno345...", "pqr678..."],
    "binding_signature": "stu901..."
  }'
```

### REST API - Validators

```powershell
# List validators
curl.exe -X GET "http://localhost:8080/validators"
```

### REST API - Bridges (Cross-chain)

```powershell
# List available bridges
curl.exe -X GET "http://localhost:8080/bridges"

# List bridge transfers
curl.exe -X GET "http://localhost:8080/bridges/transfers"

# Get specific transfer
curl.exe -X GET "http://localhost:8080/bridges/transfers/transfer_001"

# Initiate bridge transfer
curl.exe -X POST "http://localhost:8080/bridges/transfers" `
  -H "Content-Type: application/json" `
  -d "{\"source_chain\":\"bitcoin-mainnet\",\"target_chain\":\"erbium-mainnet\",\"amount\":1000000,\"asset_id\":\"BTC\",\"sender\":\"0x742d35Cc6634C0532925a3b844Bc454e4438f44e\",\"recipient\":\"0x742d35Cc6634C0532925a3b844Bc454e4438f44f\"}"
```

### RPC API - Ethereum-compatible Methods

```powershell
# Get latest block number
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}"

# Get block by hash
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByHash\",\"params\":[\"0xblock1\", false],\"id\":2}"

# Get block by number
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBlockByNumber\",\"params\":[\"latest\", false],\"id\":3}"

# Get transaction by hash
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getTransactionByHash\",\"params\":[\"0xtx1\"],\"id\":4}"

# Get account balance
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_getBalance\",\"params\":[\"0x742d35Cc6634C0532925a3b844Bc454e4438f44e\", \"latest\"],\"id\":5}"

# Send transaction
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_sendTransaction\",\"params\":[{\"from\": \"0x742d35Cc6634C0532925a3b844Bc454e4438f44e\",\"to\": \"0x742d35Cc6634C0532925a3b844Bc454e4438f44f\",\"value\": \"0x3e8\",\"gas\": \"0x5208\",\"gasPrice\": \"0x4a817c800\"}],\"id\":6}"

# Get network version
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"net_version\",\"params\":[],\"id\":7}"

# Get client version
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"web3_clientVersion\",\"params\":[],\"id\":8}"

# Get chain information (Erbium specific)
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"erb_chainInfo\",\"params\":[],\"id\":9}"

# Get chain ID
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_chainId\",\"params\":[],\"id\":10}"

# Get gas price
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_gasPrice\",\"params\":[],\"id\":11}"

# Estimate gas for transaction
curl.exe -X POST "http://localhost:8545" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_estimateGas\",\"params\":[{\"to\":\"0x742d35Cc6634C0532925a3b844Bc454e4438f44f\",\"value\":\"0x3e8\"}],\"id\":12}"
```

### WebSocket - Tests (Windows)

```powershell
# Install wscat globally via npm (if you have Node.js)
npm install -g wscat

# Connect to WebSocket
wscat -c ws://localhost:8546/ws

# Or use PowerShell for basic tests (limited)
# PowerShell has no native WebSocket support, use external tools
```

## 📜 Automated Test Scripts

### Bash Script for Linux/Mac

```bash
#!/bin/bash
# test_api_linux.sh

BASE_URL="http://localhost:8080"
RPC_URL="http://localhost:8545"

echo "=== Testing Erbium-Node REST API ==="

# GET tests
echo "1. Node information:"
curl -s "$BASE_URL/node" | jq '.'

echo "2. Chain information:"
curl -s "$BASE_URL/chain" | jq '.'

echo "3. Block list:"
curl -s "$BASE_URL/blocks" | jq '.'

echo "4. Transaction list:"
curl -s "$BASE_URL/transactions" | jq '.'

echo "5. Validator list:"
curl -s "$BASE_URL/validators" | jq '.'

echo "6. Health check:"
curl -s "$BASE_URL/health" | jq '.'

echo "7. Bridge list:"
curl -s "$BASE_URL/bridges" | jq '.'

echo "8. Bridge transfers list:"
curl -s "$BASE_URL/bridges/transfers" | jq '.'

echo ""
echo "=== Testing Erbium-Node RPC API ==="

echo "9. Block number:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | jq '.'

echo "10. Network version:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"net_version","params":[],"id":2}' | jq '.'

echo "11. Chain information:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"erb_chainInfo","params":[],"id":3}' | jq '.'

echo ""
echo "=== All tests completed ==="
```

### PowerShell Script for Windows

```powershell
# test_api_windows.ps1

$BASE_URL = "http://localhost:8080"
$RPC_URL = "http://localhost:8545"

Write-Host "=== Testing Erbium-Node REST API ===" -ForegroundColor Green

# GET tests
Write-Host "1. Node information:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/node"

Write-Host "2. Chain information:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/chain"

Write-Host "3. Block list:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/blocks"

Write-Host "4. Transaction list:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/transactions"

Write-Host "5. Validator list:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/validators"

Write-Host "6. Health check:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/health"

Write-Host "7. Bridge list:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/bridges"

Write-Host "8. Bridge transfers list:" -ForegroundColor Yellow
curl.exe -X GET "$BASE_URL/bridges/transfers"

Write-Host ""
Write-Host "=== Testing Erbium-Node RPC API ===" -ForegroundColor Green

Write-Host "9. Block number:" -ForegroundColor Yellow
curl.exe -X POST "$RPC_URL" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"eth_blockNumber\",\"params\":[],\"id\":1}"

Write-Host "10. Network version:" -ForegroundColor Yellow
curl.exe -X POST "$RPC_URL" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"net_version\",\"params\":[],\"id\":2}"

Write-Host "11. Chain information:" -ForegroundColor Yellow
curl.exe -X POST "$RPC_URL" `
  -H "Content-Type: application/json" `
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"erb_chainInfo\",\"params\":[],\"id\":3}"

Write-Host ""
Write-Host "=== All tests completed ===" -ForegroundColor Green
```

## 📝 Important Notes

### Addresses and Formats
- **Addresses**: Use valid Ethereum addresses (42 hex characters starting with `0x`)
- **Hashes**: Use valid hashes or examples provided in mock code
- **Numeric Values**: In RPC, use hexadecimal format (ex: `0x3e8` = 1000 decimal)

### Transactions
- **Nonce**: Must be incremented for each transaction from same account
- **Fees**: Specified in blockchain units
- **ZK Proofs**: For confidential transactions, provide valid proofs

### API Configuration
- **CORS**: Enabled for any origin in REST API
- **Rate Limiting**: 1000 requests per minute by default
- **Authentication**: Currently does not require authentication

### Test Dependencies
- **Linux/Mac**: `curl`, `jq` (optional for JSON formatting)
- **Windows**: `curl.exe` (included in Windows 10+), PowerShell
- **WebSocket**: `websocat` (Linux/Mac) or `wscat` (cross-platform via npm)

### Expected Responses
- **REST API**: Returns JSON with `success`, `data`, and `error` fields
- **RPC API**: Returns JSON-RPC 2.0 with `jsonrpc`, `result`/`error`, and `id` fields

To run the tests, first ensure the node is running with APIs enabled!
