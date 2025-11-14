# 🌐 Erbium Blockchain - Cloud Deployment Guide

This guide explains how to deploy the Erbium blockchain on multiple cloud VMs using the **mainnet.toml** configuration (which is already optimized for production).

## 📋 Prerequisites

- **2+ VMs** in the cloud (AWS EC2, Google Cloud, Azure, DigitalOcean, etc.)
- **Rust 1.70+** installed on each VM
- **OpenSSL, LLVM, Perl** installed
- **Open Ports**: 22 (SSH), 3030 (P2P), 8545 (RPC), 8080 (REST), 8546 (WebSocket)
- **SSH Keys** configured for VM access

## 🔒 Network Security Status

### ✅ **Implemented and Secure:**
- **Encryption**: Noise Protocol for all P2P communications
- **Authentication**: Peer authentication with trust levels
- **Firewall**: Rate limiting and DDoS protection
- **Discovery**: Peer discovery system with bootstrap peers
- **Synchronization**: Blockchain sync with integrity validation

### ⚠️ **Attention:**
- **Storage encryption**: Not yet implemented (but planned)
- **API authentication**: Recommended for production

## 🚀 Simple Deployment (Step by Step)

### 1. **Prepare the VMs**

```bash
# On each VM, install dependencies:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clone the repository
git clone https://github.com/your-org/Erbium-Node.git
cd Erbium-Node

# Install system dependencies
sudo ./scripts/install_dependencies.sh

# Compile in release mode
cargo build --release
```

### 2. **Configure Bootstrap Peers**

Identify the public IPs of your VMs:

```
VM1: 1.2.3.4
VM2: 5.6.7.8
```

### 3. **Start the First Node**

```bash
# VM1 - First node (uses mainnet.toml directly)
./target/release/erbium-node --config config/mainnet.toml
```

Wait for the node to start and note the **Peer ID** in the logs:
```
Local peer id: 12D3KooW...
```

### 4. **Configure and Start Additional Nodes**

```bash
# VM2 - Edit config/mainnet.toml and add in the [bootstrap] section:

bootstrap_peers = [
    "/ip4/1.2.3.4/tcp/3030/p2p/12D3KooW..."
]

# Start the node
./target/release/erbium-node --config config/mainnet.toml
```

### 5. **Verify Connectivity**

```bash
# Check connected peers via REST API
curl http://localhost:8080/api/v1/network/peers

# Check synchronization status
curl http://localhost:8080/api/v1/node/status

# Check logs
tail -f /var/log/erbium/node.log
```

## 🔧 Mainnet Configuration (Production)

The `config/mainnet.toml` file is already optimized for cloud production:

```toml
[network]
name = "mainnet"
chain_id = 1

[node]
p2p_port = 3030
rpc_port = 8545
rest_port = 8080
ws_port = 8546
max_peers = 100
min_peers = 10

[bootstrap]
# Add your peers here
bootstrap_peers = [
    "/ip4/VM1_IP/tcp/3030/p2p/VM1_PEER_ID",
    "/ip4/VM2_IP/tcp/3030/p2p/VM2_PEER_ID"
]
```

## 📊 Monitoring

```bash
# Node status
curl http://localhost:8080/api/v1/node/status

# Connected peers
curl http://localhost:8080/api/v1/network/peers

# Blockchain status
curl http://localhost:8080/api/v1/blockchain/status
```

## 🚨 Troubleshooting

### **Nodes not connecting:**
1. Check open ports in firewall/security group
2. Confirm correct Peer IDs in bootstrap
3. Test connectivity: `telnet VM2_IP 3030`
4. Check logs in `/var/log/erbium/node.log`

### **Common errors:**
```
❌ Failed to bind to address: Address already in use
✅ Solution: change port or kill old processes

❌ No peers available for sync
✅ Solution: check bootstrap peers
```

## 🎯 Production Checklist

- [ ] **Security**: Noise encryption active
- [ ] **Connectivity**: All nodes connected
- [ ] **Synchronization**: Blockchain synchronized
- [ ] **Monitoring**: Logs and metrics active

---

**✅ Use `config/mainnet.toml` - it's already optimized for cloud production!** 🚀
