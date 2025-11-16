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

---

# REST API Endpoints

## Node Information

### GET /node

Get basic node information.

**Response:**
```json
{
  "success": true,
  "data": {
    "version": "erbium/1.0.0",
    "network": "mainnet",
    "blockHeight": 1000,
    "syncStatus": "synced",
    "peers": 16
  }
}
```

## Blockchain Information

### GET /chain

Get blockchain information.

**Response:**
```json
{
  "success": true,
  "data": {
    "height": 1000,
    "network": "erbium",
    "version": "1.0.0",
    "timestamp": 1635724800000
  }
}
```

## Blocks

### GET /blocks

Get recent blocks.

**Query Parameters:**
- `limit` (optional): Number of blocks to return (default: 10, max: 100)

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "hash": "0xblock1",
      "height": 1000,
      "timestamp": 1635724800000,
      "transactionCount": 10
    }
  ]
}
```

### GET /blocks/{hash}

Get block by hash.

**Parameters:**
- `hash`: Block hash (hex string)

**Response:**
```json
{
  "success": true,
  "data": {
    "hash": "0xblock1",
    "height": 1000,
    "timestamp": 1635724800000,
    "transactions": ["0xtx1", "0xtx2"],
    "validator": "0xvalidator",
    "parentHash": "0xparent"
  }
}
```

## Transactions

### GET /transactions

Get recent transactions.

**Query Parameters:**
- `limit` (optional): Number of transactions to return (default: 10, max: 100)

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "hash": "0xtx1",
      "from": "0xsender1",
      "to": "0xrecipient1",
      "amount": 1000,
      "fee": 10,
      "status": "confirmed"
    }
  ]
}
```

### GET /transactions/{hash}

Get transaction by hash.

**Parameters:**
- `hash`: Transaction hash (hex string)

**Response:**
```json
{
  "success": true,
  "data": {
    "hash": "0xtx1",
    "from": "0xsender",
    "to": "0xrecipient",
    "amount": 1000,
    "fee": 10,
    "nonce": 1,
    "timestamp": 1635724800000,
    "blockHash": "0xblock",
    "blockHeight": 1000
  }
}
```

### POST /transactions

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

### GET /validators

Get all validators.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "address": "0xvalidator1",
      "stake": "1000000",
      "active": true,
      "performance": "99.5%"
    }
  ]
}
```

## Governance

### GET /governance/proposals

Get active governance proposals.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "proposal_001",
      "title": "Increase block gas limit",
      "description": "Proposal to increase the block gas limit from 8M to 12M",
      "proposer": "0xproposer1",
      "status": "Active",
      "votes_for": 150000,
      "votes_against": 25000,
      "start_block": 1000,
      "end_block": 2000,
      "created_at": 1635724800000
    }
  ]
}
```

### GET /governance/proposals/{id}

Get proposal by ID.

**Parameters:**
- `id`: Proposal ID

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "proposal_001",
    "title": "Increase block gas limit",
    "description": "Proposal to increase the block gas limit from 8M to 12M",
    "proposer": "0xproposer1",
    "status": "Active",
    "votes_for": 150000,
    "votes_against": 25000,
    "quorum_required": 100000,
    "start_block": 1000,
    "end_block": 2000,
    "execution_block": 2500,
    "actions": [
      {
        "type": "UpdateParameter",
        "parameter": "block_gas_limit",
        "value": "12000000"
      }
    ],
    "created_at": 1635724800000
  }
}
```

### POST /governance/proposals

Create a new governance proposal.

**Request Body:**
```json
{
  "title": "New Proposal",
  "description": "Proposal description",
  "actions": [
    {
      "type": "UpdateParameter",
      "parameter": "block_gas_limit",
      "value": "12000000"
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "proposal_id": "proposal_123",
    "status": "created",
    "voting_starts_in": "1 block"
  }
}
```

### POST /governance/proposals/{id}/vote

Vote on a governance proposal.

**Parameters:**
- `id`: Proposal ID

**Request Body:**
```json
{
  "voter": "0xvoter_address",
  "option": "for",
  "voting_power": 10000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "proposal_id": "proposal_123",
    "vote_recorded": true,
    "voting_power": 10000
  }
}
```

### GET /governance/dao

Get DAO information.

**Response:**
```json
{
  "success": true,
  "data": {
    "name": "Erbium DAO",
    "total_members": 150,
    "total_supply": "1000000000",
    "circulating_supply": "750000000",
    "treasury_balance": "50000000",
    "active_proposals": 3,
    "passed_proposals": 12,
    "rejected_proposals": 2,
    "governance_token": {
      "symbol": "ERB",
      "decimals": 18,
      "contract_address": "0x0000000000000000000000000000000000000000"
    }
  }
}
```

## Staking

**⚠️ Nota: Staking delegation/undelegation It is NOT implemented in the current version..**

### GET /staking/validators

Get all staking validators.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "address": "0xvalidator1",
      "name": "Validator One",
      "total_stake": "1000000",
      "self_stake": "100000",
      "delegators": 25,
      "commission": "5%",
      "uptime": "99.8%",
      "status": "Active",
      "jailed": false
    }
  ]
}
```

### GET /staking/validators/{address}

Get validator by address.

**Parameters:**
- `address`: Validator address

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "0xvalidator1",
    "name": "Validator One",
    "description": "High-performance validator node",
    "website": "https://validator1.erbium.io",
    "total_stake": "1000000",
    "self_stake": "100000",
    "delegators": 25,
    "commission": "5%",
    "commission_reward": "50000",
    "uptime": "99.8%",
    "blocks_proposed": 1250,
    "status": "Active",
    "jailed": false,
    "unjail_time": null,
    "delegations": [
      {
        "delegator": "0xdelegator1",
        "amount": "50000",
        "rewards": "2500"
      }
    ]
  }
}
```

### POST /staking/delegate

Delegate tokens to a validator.

**Request Body:**
```json
{
  "delegator": "0xdelegator",
  "validator": "0xvalidator",
  "amount": 100000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "delegation_id": "delegation_123",
    "status": "confirmed",
    "voting_power_granted": 100000
  }
}
```

### POST /staking/undelegate

Undelegate tokens from a validator.

**Request Body:**
```json
{
  "delegator": "0xdelegator",
  "validator": "0xvalidator",
  "amount": 50000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "initiated",
    "unbonding_period": "21 days",
    "completion_time": 1635724800000
  }
}
```

### GET /staking/rewards/{address}

Get staking rewards for an address.

**Parameters:**
- `address`: Delegator address

**Response:**
```json
{
  "success": true,
  "data": {
    "delegator": "0xdelegator",
    "total_rewards": "12500",
    "rewards_by_validator": [
      {
        "validator": "0xvalidator1",
        "rewards": "10000",
        "commission": "500"
      },
      {
        "validator": "0xvalidator2",
        "rewards": "2500",
        "commission": "125"
      }
    ],
    "last_claim": 1635723800000,
    "next_claim_available": 1635724800000
  }
}
```

### POST /staking/claim-rewards

Claim staking rewards.

**Request Body:**
```json
{
  "delegator": "0xdelegator",
  "validator": "0xvalidator"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "claimed_amount": "12500",
    "transaction_hash": "0xclaim_tx_hash",
    "status": "confirmed"
  }
}
```

## Smart Contracts

### GET /contracts/{address}

Get contract information.

**Parameters:**
- `address`: Contract address

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "0xcontract",
    "creator": "0xcreator",
    "creation_tx": "0xcreation_tx",
    "creation_block": 100,
    "code_size": 24576,
    "storage_slots": 15,
    "last_transaction": 1635724800000,
    "balance": "5000000",
    "verified": true,
    "name": "Sample Token",
    "symbol": "TOK",
    "decimals": 18
  }
}
```

### GET /contracts/{address}/abi

Get contract ABI.

**Parameters:**
- `address`: Contract address

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "inputs": [{"name": "to", "type": "address"}, {"name": "amount", "type": "uint256"}],
      "name": "transfer",
      "outputs": [{"name": "", "type": "bool"}],
      "stateMutability": "nonpayable",
      "type": "function"
    }
  ]
}
```

### POST /contracts

Deploy a new contract.

**Request Body:**
```json
{
  "bytecode": "0x608060405234801561001057600080fd5b50d3801561001d57600080fd5b50d2801561002a57600080fd5b5061012f806100396000396000f3fe6080604052348015600f57600080fd5b506004361060325760003560e01c80635c60da1b146037575b600080fd5b603d6051565b60405160488463ffffffff1660e01b81526004018060405180830381600087803b15801560...",
  "constructor_args": [],
  "gas_limit": 3000000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "contract_address": "0xnew_contract",
    "deployer": "0xdeployer",
    "transaction_hash": "0xdeploy_tx",
    "gas_used": 1500000
  }
}
```

### POST /contracts/{address}/call

Call a contract function.

**Parameters:**
- `address`: Contract address

**Request Body:**
```json
{
  "function": "transfer",
  "args": ["0xrecipient", "1000000000000000000"],
  "gas_limit": 100000
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "return_value": "0x0000000000000000000000000000000000000000000000000000000000000001",
    "gas_used": 25000,
    "logs": [
      {
        "address": "0xcontract",
        "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
        "data": "0x000000000000000000000000sender000000000000000000000000recipient00000000000000000000000000000000000000000000000000000000000003e8"
      }
    ]
  }
}
```

### GET /contracts/events

Get contract events.

**Query Parameters:**
- `contract_address` (optional): Filter by contract
- `event_signature` (optional): Filter by event signature
- `from_block` (optional): Starting block
- `to_block` (optional): Ending block

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "contract_address": "0xcontract1",
      "event_signature": "Transfer(address,address,uint256)",
      "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
      "data": "0x000000000000000000000000sender000000000000000000000000recipient00000000000000000000000000000000000000000000000000000000000003e8",
      "block_number": 1000,
      "transaction_hash": "0xtx1",
      "log_index": 0
    }
  ]
}
```

## Accounts

### GET /accounts/{address}

Get account information.

**Parameters:**
- `address`: Account address

**Response:**
```json
{
  "success": true,
  "data": {
    "address": "0xaccount",
    "balance": "1000000",
    "nonce": 5,
    "code_hash": "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470",
    "storage_root": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "transaction_count": 15,
    "is_contract": false,
    "created_at": 1635723800000,
    "last_activity": 1635724800000
  }
}
```

### GET /accounts/{address}/transactions

Get account transaction history.

**Parameters:**
- `address`: Account address

**Query Parameters:**
- `limit` (optional): Number of transactions (default: 10, max: 100)
- `offset` (optional): Pagination offset

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "hash": "0xtx1",
      "block_number": 1000,
      "timestamp": 1635724800000,
      "from": "0xaccount",
      "to": "0xrecipient1",
      "value": "1000",
      "gas_used": 21000,
      "status": "success"
    }
  ]
}
```

## Cross-Chain Bridges

### GET /bridges

Get available bridges.

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "chain_id": "bitcoin-mainnet",
      "chain_type": "Bitcoin",
      "enabled": true,
      "security_level": "Medium"
    },
    {
      "chain_id": "ethereum-mainnet",
      "chain_type": "Ethereum",
      "enabled": true,
      "security_level": "High"
    }
  ]
}
```

### GET /bridges/transfers

Get bridge transfers.

**Query Parameters:**
- `status` (optional): Filter by status (pending, completed, failed)
- `limit` (optional): Number of transfers (default: 10)

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": "transfer_001",
      "source_chain": "bitcoin-mainnet",
      "target_chain": "erbium-mainnet",
      "amount": 1000000,
      "asset_id": "BTC",
      "status": "Pending",
      "created_at": 1635724800000
    }
  ]
}
```

### GET /bridges/transfers/{id}

Get bridge transfer by ID.

**Parameters:**
- `id`: Transfer ID

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "transfer_001",
    "source_chain": "bitcoin-mainnet",
    "target_chain": "erbium-mainnet",
    "amount": 1000000,
    "asset_id": "BTC",
    "sender": "0xsender",
    "recipient": "0xrecipient",
    "status": "Pending",
    "fee": 1000,
    "created_at": 1635724800000,
    "completed_at": null
  }
}
```

### POST /bridges/transfers

Initiate a bridge transfer.

**Request Body:**
```json
{
  "source_chain": "bitcoin-mainnet",
  "target_chain": "erbium-mainnet",
  "amount": 1000000,
  "asset_id": "BTC",
  "sender": "0xsender",
  "recipient": "0xrecipient"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "transfer_id": "transfer_123",
    "status": "initiated",
    "estimated_completion": "5-10 minutes"
  }
}
```

## Analytics

### GET /analytics/tps

Get current TPS (Transactions Per Second).

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

### GET /analytics/network-health

Get network health metrics.

**Response:**
```json
{
  "success": true,
  "data": {
    "block_time_average": 15.2,
    "network_hashrate": "1200000000000000",
    "active_validators": 25,
    "total_stake": "25000000",
    "finalized_blocks_24h": 5760,
    "missed_blocks_24h": 12,
    "network_uptime": 99.8,
    "peer_count": 45,
    "sync_status": "synced"
  }
}
```

### GET /analytics/bridge-activity

Get bridge activity analytics.

**Response:**
```json
{
  "success": true,
  "data": {
    "total_transfers_24h": 1250,
    "total_volume_24h": "500000000",
    "active_bridges": ["bitcoin", "ethereum", "polkadot"],
    "bridge_stats": {
      "bitcoin": {
        "transfers": 450,
        "volume": "150000000",
        "avg_confirmation_time": 3600
      },
      "ethereum": {
        "transfers": 600,
        "volume": "250000000",
        "avg_confirmation_time": 180
      },
      "polkadot": {
        "transfers": 200,
        "volume": "100000000",
        "avg_confirmation_time": 120
      }
    },
    "failed_transfers_24h": 5,
    "pending_transfers": 15
  }
}
```

## System

### GET /health

Health check endpoint.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "timestamp": 1635724800000
  }
}
```

### GET /metrics

Get comprehensive system metrics.

**Response:**
```json
{
  "success": true,
  "data": {
    "blockchain": {
      "head_block": 1500,
      "total_transactions": 45000,
      "total_accounts": 1200,
      "total_contracts": 45,
      "circulating_supply": "750000000"
    },
    "network": {
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

For more information, visit the [Erbium Documentation](https://docs.erbium.io) or join our [Discord Community](https://discord.gg/erbium).
