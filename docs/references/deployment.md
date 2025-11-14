# 🌐 Erbium Blockchain - Cloud Deployment Guide

Este guia explica como fazer o deployment da blockchain Erbium em múltiplas VMs na nuvem usando a configuração **mainnet.toml** (que já é otimizada para produção).

## 📋 Pré-requisitos

- **2+ VMs** na nuvem (AWS EC2, Google Cloud, Azure, DigitalOcean, etc.)
- **Rust 1.70+** instalado em cada VM
- **OpenSSL, LLVM, Perl** instalados
- **Portas abertas**: 22 (SSH), 3030 (P2P), 8545 (RPC), 8080 (REST), 8546 (WebSocket)
- **Chaves SSH** configuradas para acesso às VMs

## 🔒 Status de Segurança da Rede

### ✅ **Implementado e Seguro:**
- **Criptografia**: Noise Protocol para todas as comunicações P2P
- **Autenticação**: Peer authentication com trust levels
- **Firewall**: Rate limiting e DDoS protection
- **Descoberta**: Sistema de peer discovery com bootstrap peers
- **Sincronização**: Blockchain sync com validação de integridade

### ⚠️ **Atenção:**
- **Storage encryption**: Ainda não implementado (mas planejado)
- **API authentication**: Recomendado para produção

## 🚀 Deployment Simples (Passo a Passo)

### 1. **Preparar as VMs**

```bash
# Em cada VM, instale as dependências:
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Clone o repositório
git clone https://github.com/your-org/Erbium-Node.git
cd Erbium-Node

# Instale dependências do sistema
sudo ./scripts/install_dependencies.sh

# Compile em modo release
cargo build --release
```

### 2. **Configurar Bootstrap Peers**

Identifique os IPs públicos das suas VMs:

```
VM1: 1.2.3.4
VM2: 5.6.7.8
```

### 3. **Iniciar o Primeiro Nó**

```bash
# VM1 - Primeiro nó (usa mainnet.toml diretamente)
./target/release/erbium-node --config config/mainnet.toml
```

Aguarde o nó iniciar e anote o **Peer ID** nos logs:
```
Local peer id: 12D3KooW...
```

### 4. **Configurar e Iniciar Nós Adicionais**

```bash
# VM2 - Edite config/mainnet.toml e adicione na seção [bootstrap]:

bootstrap_peers = [
    "/ip4/1.2.3.4/tcp/3030/p2p/12D3KooW..."
]

# Inicie o nó
./target/release/erbium-node --config config/mainnet.toml
```

### 5. **Verificar Conectividade**

```bash
# Verificar peers conectados via API REST
curl http://localhost:8080/api/v1/network/peers

# Verificar status de sincronização
curl http://localhost:8080/api/v1/node/status

# Verificar logs
tail -f /var/log/erbium/node.log
```

## 🔧 Configuração Mainnet (Produção)

O arquivo `config/mainnet.toml` já está otimizado para produção na nuvem:

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
# Adicione seus peers aqui
bootstrap_peers = [
    "/ip4/IP_VM1/tcp/3030/p2p/PEER_ID_VM1",
    "/ip4/IP_VM2/tcp/3030/p2p/PEER_ID_VM2"
]
```

## 📊 Monitoramento

```bash
# Status do nó
curl http://localhost:8080/api/v1/node/status

# Peers conectados
curl http://localhost:8080/api/v1/network/peers

# Status da blockchain
curl http://localhost:8080/api/v1/blockchain/status
```

## 🚨 Troubleshooting

### **Nós não conectam:**
1. Verifique portas abertas no firewall/security group
2. Confirme Peer IDs corretos no bootstrap
3. Teste conectividade: `telnet IP_VM2 3030`
4. Verifique logs em `/var/log/erbium/node.log`

### **Erros comuns:**
```
❌ Failed to bind to address: Address already in use
✅ Solução: mude a porta ou mate processos antigos

❌ No peers available for sync
✅ Solução: verifique bootstrap peers
```

## 🎯 Checklist de Produção

- [ ] **Segurança**: Noise encryption ativo
- [ ] **Conectividade**: Todos os nós conectados
- [ ] **Sincronização**: Blockchain sincronizada
- [ ] **Monitoramento**: Logs e métricas ativos

---

**✅ Use `config/mainnet.toml` - ele já é otimizado para produção na nuvem!** 🚀
