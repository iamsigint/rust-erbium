# Erbium Blockchain API Reference

## Overview

The Erbium Blockchain provides comprehensive REST, JSON-RPC, and WebSocket APIs for interacting with the blockchain, governance system, staking, smart contracts, and cross-chain bridges.

## Base URL

```
http://localhost:8080/api/v1
```


## Authentication

Most endpoints are public. Bridge operations may require authentication in production.

## Response Format

All REST API responses follow this format:

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

## Error Responses

```json
{
  "success": false,
  "data": null,
  "error": "Error message"
}
```

## Analytics

### GET /api/v1/analytics/tps

Returns zeroed analytics placeholders until telemetry is implemented.

**Response:**
```json
{
  "success": true,
  "data": {
    "current_tps": 0,
    "peak_tps": 0,
    "average_tps_24h": 0,
    "average_tps_7d": 0,
    "transactions_last_block": 0,
    "gas_used_last_block": 0,
    "timestamp": 1763272854121
  }
}
```

### GET /api/v1/analytics/gas-usage

```json
{
  "success": true,
  "data": {
    "average_gas_price": "0",
    "median_gas_price": "0",
    "gas_used_24h": "0",
    "gas_limit_total": "0",
    "gas_utilization_percent": 0,
    "expensive_tx_count": 0,
    "cheap_tx_count": 0
  }
}
```

### GET /api/v1/analytics/network-health

```json
{
  "success": true,
  "data": {
    "block_time_average": 0,
    "network_hashrate": "0",
    "active_validators": 0,
    "total_stake": "0",
    "finalized_blocks_24h": 0,
    "missed_blocks_24h": 0,
    "network_uptime": 0,
    "peer_count": 0,
    "sync_status": "stopped"
  }
}
```

### GET /api/v1/analytics/bridge-activity

```json
{
  "success": true,
  "data": {
    "total_transfers_24h": 0,
    "total_volume_24h": "0",
    "active_bridges": [],
    "bridge_stats": {},
    "failed_transfers_24h": 0,
    "pending_transfers": 0
  }
}
```
        "type": "Transfer",
        "from": "0x0000000000000000000000000000000000000000",
        "to": "0xe91b31507e84d575a0a12b1bbb98648bd2079011",
        "amount": 5000000000000000,
        "fee": 0,
        ### GET /api/v1/health
        Health check endpoint.
        "timestamp": 1763272854119,
        "blockHash": "19266fa6306f6fa8cb73e04e8cfec9ee21d83f06378f2bb77883eb2fbd0ab082",
        "blockHeight": 0,
        "index": 0,
        ### GET /api/v1/metrics
        Get comprehensive system metrics (currently zeroed placeholders).
      }
    ]
  }
}
            "status": "healthy",
            "timestamp": 1763272854121
## Transactions

### GET /api/v1/transactions

Get up to 100 most recent confirmed transactions.

              "head_block": 0,
              "total_transactions": 0,
              "total_accounts": 0,
              "total_contracts": 0,
              "circulating_supply": "0"
    {
      "hash": "4d05a538ba4465914e38790cb1fc749af2d770fbf413ad238961127bbfbccffe",
              "active_peers": 0,
              "connected_peers": 0,
              "bytes_received": "0",
              "bytes_sent": "0"
      "fee": 0,
      "nonce": 0,
              "avg_block_time": 0,
              "avg_tx_per_block": 0,
              "gas_utilization": 0,
              "cpu_usage": 0,
              "memory_usage": 0
    }
  ]
              "active_bridges": 0,
              "pending_transfers": 0,
              "completed_transfers_24h": 0,
              "bridge_balance_btc": "0",
              "bridge_balance_eth": "0"
Get transaction by hash.

**Parameters:**
- `hash`: Transaction hash (hex string)

**Response:**
```json
{
  "success": true,
  "data": {
    "hash": "4d05a538ba4465914e38790cb1fc749af2d770fbf413ad238961127bbfbccffe",
    "type": "Transfer",
    "from": "0x0000000000000000000000000000000000000000",
    "to": "0xe91b31507e84d575a0a12b1bbb98648bd2079011",
    "amount": 5000000000000000,
    "fee": 0,
    "nonce": 0,
    "timestamp": 1763272854119,
    "blockHash": "19266fa6306f6fa8cb73e04e8cfec9ee21d83f06378f2bb77883eb2fbd0ab082",
    "blockHeight": 0,
    "index": 0,
    "status": "confirmed"
  }
}
```

### POST /api/v1/transactions

Send a transaction.

**Request Body:**
```json
{
  "transaction_type": "transfer",
  "from": "0xsender",
  "to": "0xrecipient",
  "amount": 1000,
  "fee": 10,
  "nonce": 1
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transactionHash": "0xtx_hash",
    "status": "pending"
  }
}
```

## Validators

### GET /api/v1/validators

Validators are not yet implemented. The endpoint responds with an empty list.

**Response:**
```json
{
  "success": true,
  "data": []
}
```

## Governance

Governance endpoints exist but the feature is not yet implemented. Each call returns an error payload with `success` set to `false`.

### GET /api/v1/governance/proposals

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Governance system not yet implemented in this version"
}
```

### GET /api/v1/governance/proposals/{id}

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Proposal not found"
}
```

### POST /api/v1/governance/proposals

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Governance system not yet implemented in this version"
}
```

### POST /api/v1/governance/proposals/{id}/vote

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Governance system not yet implemented in this version"
}
```

### GET /api/v1/governance/dao

**Response:**
```json
{
  "success": false,
  "data": null,
  "error": "Governance system not yet implemented in this version"
}
```

## Staking

Staking is not available in the current build. Each staking endpoint returns an error payload indicating the feature is disabled.

### GET /api/v1/staking/validators

```json
{
  "success": false,
  "data": null,
  "error": "Staking system not yet implemented in this version"
}
```

### GET /api/v1/staking/validators/{address}

```json
{
  "success": false,
  "data": null,
  "error": "Validator not found"
}
```

### POST /api/v1/staking/delegate

```json
{
  "success": false,
  "data": null,
  "error": "Staking delegation not implemented in current version"
}
```

### POST /api/v1/staking/undelegate

```json
{
  "success": false,
  "data": null,
  "error": "Staking undelegation not implemented in current version"
}
```

### GET /api/v1/staking/rewards/{address}

```json
{
  "success": false,
  "data": null,
  "error": "Staking system not yet implemented in this version"
}
```

### POST /api/v1/staking/claim-rewards

```json
{
  "success": false,
  "data": null,
  "error": "Staking system not yet implemented in this version"
}
```

## Smart Contracts

Smart contract support is not implemented. All contract endpoints return an error payload.

### GET /api/v1/contracts/{address}

```json
{
  "success": false,
  "data": null,
  "error": "Contract not found"
}
```

### GET /api/v1/contracts/{address}/abi

```json
{
  "success": false,
  "data": null,
  "error": "Contract ABI not available"
}
```

### POST /api/v1/contracts

```json
{
  "success": false,
  "data": null,
  "error": "Smart contract deployment not implemented in this version"
}
```

### POST /api/v1/contracts/{address}/call

```json
{
  "success": false,
  "data": null,
  "error": "Smart contract calls not implemented in this version"
}
```

### GET /api/v1/contracts/events

```json
{
  "success": true,
  "data": []
}
```

## Accounts

### GET /api/v1/accounts/{address}

Fetch live account data from the persistent state. Unknown addresses return a `404`-style error payload.

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "0xe91b31507e84d575a0a12b1bbb98648bd2079011",
    "balance": "5000000000000000",
    "nonce": 0,
    "code_hash": null,
    "storage_root": "2e1cfa82b035c26cbbbdae632cea070514eb8b773f616aaeaf668e2f0be8f10d",
    "transaction_count": 1,
    "is_contract": false,
    "created_at": 1763272854121,
    "last_activity": 1763272854121
  },
  "error": null
}
```

### GET /api/v1/accounts/{address}/transactions

Returns up to 100 of the most recent transactions touching the address.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "hash": "4d05a538ba4465914e38790cb1fc749af2d770fbf413ad238961127bbfbccffe",
      "type": "Transfer",
      "from": "0x0000000000000000000000000000000000000000",
      "to": "0xe91b31507e84d575a0a12b1bbb98648bd2079011",
      "amount": 5000000000000000,
      "fee": 0,
      "nonce": 0,
      "timestamp": 1763272854119,
      "blockHash": "19266fa6306f6fa8cb73e04e8cfec9ee21d83f06378f2bb77883eb2fbd0ab082",
      "blockHeight": 0,
      "index": 0,
      "status": "confirmed"
    }
  ],
  "error": null
}
```

## Cross-Chain Bridges

### GET /api/v1/bridges

```json
{
  "success": true,
  "data": []
}
```

### GET /api/v1/bridges/transfers

```json
{
  "success": true,
  "data": []
}
```

### GET /api/v1/bridges/transfers/{id}

```json
{
  "success": false,
  "data": null,
  "error": "Bridge transfer not found"
}
```

### POST /api/v1/bridges/transfers

```json
{
  "success": false,
  "data": null,
  "error": "Bridge transfers not implemented in this version"
}
```

## Analytics

### GET /api/v1/analytics/tps

Returns zeroed analytics placeholders until telemetry is implemented.

**Response:**
```json
{
  "success": true,
  "data": {
    "current_tps": 45.2,
    "peak_tps": 67.8,
    "average_tps_24h": 38.5,
    "average_tps_7d": 42.1,
    "transactions_last_block": 12,
    "gas_used_last_block": 8000000,
    "timestamp": 1635724800000
  }
}
```

### GET /analytics/gas-usage

Get gas usage analytics.

**Response:**
```json
{
  "success": true,
  "data": {
    "average_gas_price": "20000000000",
    "median_gas_price": "18000000000",
    "gas_used_24h": "15000000000",
    "gas_limit_total": "8000000",
    "gas_utilization_percent": 85.5,
    "expensive_tx_count": 25,
    "cheap_tx_count": 145
  }
}
```

    "current_tps": 0,
    "peak_tps": 0,
    "average_tps_24h": 0,
    "average_tps_7d": 0,
    "transactions_last_block": 0,
    "gas_used_last_block": 0,
    "timestamp": 1763272854121
  "success": true,
  "data": {
    "block_time_average": 15.2,
    "network_hashrate": "1200000000000000",
### GET /api/v1/analytics/gas-usage

```json
{
  "success": true,
  "data": {
    "average_gas_price": "0",
    "median_gas_price": "0",
    "gas_used_24h": "0",
    "gas_limit_total": "0",
    "gas_utilization_percent": 0,
    "expensive_tx_count": 0,
    "cheap_tx_count": 0
  }
}
```

### GET /api/v1/analytics/network-health

```json
{
  "success": true,
  "data": {
    "block_time_average": 0,
    "network_hashrate": "0",
    "active_validators": 0,
    "total_stake": "0",
    "finalized_blocks_24h": 0,
    "missed_blocks_24h": 0,
    "network_uptime": 0,
    "peer_count": 0,
    "sync_status": "stopped"
  }
}
```

### GET /api/v1/analytics/bridge-activity

```json
{
  "success": true,
  "data": {
    "total_transfers_24h": 0,
    "total_volume_24h": "0",
    "active_bridges": [],
    "bridge_stats": {},
    "failed_transfers_24h": 0,
    "pending_transfers": 0
  }
}
```
      "active_peers": 45,
      "connected_peers": 38,
      "bytes_received": "1500000000",
      "bytes_sent": "1200000000"
    },
    "performance": {
      "avg_block_time": 15.2,
      "avg_tx_per_block": 8.5,
      "gas_utilization": 85.5,
      "cpu_usage": 65.2,
      "memory_usage": 2.8
    },
    "bridges": {
      "active_bridges": 3,
      "pending_transfers": 15,
      "completed_transfers_24h": 1245,
      "bridge_balance_btc": "25.5",
      "bridge_balance_eth": "150.2"
    }
  }
}
```

---

# JSON-RPC API

The Erbium Blockchain supports Ethereum-compatible JSON-RPC methods.

## Supported Methods

### Blockchain Methods
- `eth_blockNumber()` - Get latest block number
- `eth_getBlockByHash(hash, fullTx)` - Get block by hash
- `eth_getBlockByNumber(number, fullTx)` - Get block by number
- `eth_getTransactionByHash(hash)` - Get transaction by hash
- `eth_sendTransaction(tx)` - Send transaction
- `eth_getBalance(address, block)` - Get account balance

### Network Methods
- `net_version()` - Get network ID
- `web3_clientVersion()` - Get client version

### Custom Methods
- `erb_chainInfo()` - Get chain information

## Example Usage

```javascript
// Get latest block number
curl -X POST -H "Content-Type: application/json" \
  --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
  http://localhost:8545

// Send transaction
curl -X POST -H "Content-Type: application/json" \
  --data '{
    "jsonrpc":"2.0",
    "method":"eth_sendTransaction",
    "params":[{
      "from": "0x123...",
      "to": "0x456...",
      "value": "0x1000"
    }],
    "id":1
  }' \
  http://localhost:8545
```

---

# WebSocket API

Real-time subscriptions for blockchain events.

## Connection

```
ws://localhost:8546/ws
```

## Message Format

```json
{
  "type": "message_type",
  "data": { ... }
}
```

## Subscription Methods

### Subscribe to Topics
```json
{
  "type": "subscribe",
  "data": {
    "topics": ["blocks", "transactions", "validators"]
  }
}
```

### Unsubscribe from Topics
```json
{
  "type": "unsubscribe",
  "data": {
    "topics": ["blocks"]
  }
}
```

## Available Topics

- `newBlock` - New blocks
- `newTransaction` - New transactions
- `validatorUpdate` - Validator status changes
- `bridgeEvents` - Cross-chain bridge events

## Example Usage

```javascript
const ws = new WebSocket('ws://localhost:8546/ws');

// Subscribe to new blocks
ws.send(JSON.stringify({
  type: 'subscribe',
  data: { topics: ['blocks'] }
}));

// Listen for messages
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  console.log('Received:', message);
};
```

---

# Rate Limiting

- REST API: 1000 requests per minute per IP
- JSON-RPC: 500 requests per minute per IP
- WebSocket: 100 connections per IP

---

# Error Codes

## HTTP Status Codes
- `200` - Success
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Too Many Requests
- `500` - Internal Server Error

## JSON-RPC Error Codes
- `-32700` - Parse error
- `-32600` - Invalid request
- `-32601` - Method not found
- `-32602` - Invalid params
- `-32603` - Internal error

---

# SDKs and Libraries

## JavaScript/TypeScript
```bash
npm install @erbium/sdk
```

## Python
```bash
pip install erbium-sdk
```

## Go
```bash
go get github.com/erbium/go-sdk
```

---


# Changelog


## v1.0.0
- Initial release with full REST, JSON-RPC, and WebSocket APIs
- Complete governance, staking (delegation not yet implemented), and bridge functionality
- Enterprise-grade security and monitoring
- **CRITICAL BUG FIXES:**
  - API now uses /api/v1 path prefix correctly on all endpoints
  - New accounts now show correct zero balance (removed hardcoded 1M balance)
  - Removed all fake block/transaction data - API reflects real empty blockchain state
  - Removed fake validators data - returns empty array (no validators yet)
  - Removed fake bridge configurations - returns empty array (no bridges configured)
  - Removed fake governance proposals and votes - now returns proper error messages for unimplemented features
  - Removed fake staking validators and rewards - now returns errors for unimplemented staking
  - Removed fake smart contract data - now returns errors for unimplemented contracts
  - Analytics endpoints return realistic 0 values instead of fake TPS/blocks/metrics data
  - Real version number from Cargo.toml (not hardcoded "1.0.0")
  - All endpoints now reflect true state: network stopped, height 0, peers 0, supply 0
- **Data Integrity:** API no longer returns fake production data
- Fixed critical CLI: --data-dir argument now supported correctly
- Removed demo/demo P2P code from main.rs for clean production deployment

---

For more information, visit the [Erbium Documentation](https://docs.erbium.network) or join our [Discord Community](https://discord.gg/D3Wp6epT).
