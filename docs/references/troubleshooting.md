# Troubleshooting Guide

## Compilação Errors

### Erro: `cargo check` falha com erros de compilação

**Sintomas:**
```
error[E0425]: cannot find value `filled_amount` in this scope
```

**Solução:**
1. Verifique se todas as variáveis estão sendo usadas corretamente
2. Certifique-se de que underscore prefix (`_`) não está sendo usado incorretamente
3. Adicione `#[allow(dead_code)]` para campos não utilizados ainda

### Erro: Dependências não encontradas

**Sintomas:**
```
error: linking with `link.exe` failed: exit code: 1120
```

**Windows:**
```powershell
# Instalar dependências necessárias
powershell -ExecutionPolicy Bypass -File scripts/install_dependencies.ps1

# Verificar se OpenSSL está instalado
openssl version
```

**Linux:**
```bash
sudo apt update
sudo apt install build-essential libssl-dev pkg-config
```

## Erros de Execução

### Erro: Nó não consegue sincronizar

**Sintomas:**
- Logs mostram erros de conexão de rede
- Nó não encontra peers

**Solução:**
```bash
# Verificar configuração de rede
cargo run --bin erbium-node -- --config config/testnet.toml --bootstrap-node

# Verificar portas abertas
netstat -tulpn | grep 30303
```

### Erro: Transações falhando

**Sintomas:**
```
error: Insufficient balance
```

**Solução:**
```rust
// Verificar saldo da conta
let balance = state.get_balance(&sender_address)?;
ensure!(balance >= transaction.amount + transaction.fee, "Insufficient balance");
```

## Problemas de Performance

### TPS baixo

**Causas possíveis:**
- Memória insuficiente
- CPU sobrecarregada
- Rede lenta

**Otimização:**
```toml
# config/mainnet.toml
[performance]
max_concurrent_transactions = 1000
db_cache_size = "2GB"
network_buffer_size = 67108864  # 64MB
```

### Alto uso de CPU

**Análise:**
```bash
# Monitorar processos
top -p $(pidof erbium-node)

# Verificar uso de memoria
vmstat 1
```

## Problemas de Rede

### Conexões rejeitadas

**Sintomas:**
- Falha ao conectar com peers
- Timeouts de rede

**Solução:**
```rust
// Verificar timeout de conexão
transport_config.connection_timeout = Duration::from_secs(30);
transport_config.heartbeat_interval = Duration::from_secs(30);
```

### Alta latência

**Diagnóstico:**
```bash
# Testar latência da rede
ping us-central.pool.erbium.io

# Verificar conectividade do nó
curl http://localhost:8080/status
```

## Problemas de Segurança

### Assinaturas inválidas

**Sintomas:**
```
error: Invalid signature
```

**Verificação:**
```rust
// Verificar chave pública
let pk = PublicKey::from_bytes(&pk_bytes)?;
let sig = Signature::from_bytes(&sig_bytes)?;
pk.verify(message, &sig)?;
```

### Ataque de negação de serviço

**Proteção:**
1. Limite de conexões por IP
2. Rate limiting de transações
3. Filtros de spam
4. Circuit breakers

```rust
pub struct DoSProtection {
    pub max_connections_per_ip: usize,
    pub transaction_rate_limit: usize,
    pub request_size_limit: usize,
}
```

## Problemas de DEX

### Ordens não sendo preenchidas

**Possíveis causas:**
- Liquidez insuficiente no livro de ofertas
- Preço fora do spread
- Ordem expirada

**Verificação:**
```rust
// Verificar status da ordem
let order = dex.get_order(&order_id)?;
match order.status {
    OrderStatus::Expired => println!("Order expired"),
    OrderStatus::Filled => println!("Order filled"),
    _ => println!("Order active"),
}
```

## Problemas de Layer 2

### Canais não abrindo

**Verificação:**
```rust
// Verificar saldos
let balance_a = state.get_balance(&participant_a)?;
let balance_b = state.get_balance(&participant_b)?;

// Capacidade mínima
const MIN_CHANNEL_CAPACITY: u64 = 1000; // 1000 ERB
```

### Disputas falhando

**Solução:**
```rust
// Challenge period mínimo
const MIN_CHALLENGE_PERIOD: u64 = 100; // blocos

// Verificar prova de disputa
fraud_proof.validate(&channel_state)?;
```

## Problemas de VM

### Contratos revertendo

**Debugging:**
```rust
// Verificar gas suficiente
ensure!(context.gas_limit >= MIN_GAS, "Insufficient gas");

// Verificar dados válidos
ensure!(!data.is_empty(), "Empty call data");
```

### Loops infinitos

**Proteção:**
```rust
pub struct ExecutionLimits {
    pub max_steps: usize,
    pub max_memory: usize,
    pub timeout: Duration,
}

// Verificação de loop infinito
fn check_execution_limits(&self, context: &ExecutionContext) -> Result<()> {
    ensure!(self.steps < self.max_steps, "Execution timeout");
    ensure!(self.memory.len() <= self.max_memory, "Out of memory");
    Ok(())
}
```

## Problemas de Armazenamento

### Corrupção de banco de dados

**Recuperação:**
```bash
# Backup da base de dados
cp -r data/ data.backup/

# Verificação de integridade
parity-db --db data/ --verify

# Reconstruir índices
parity-db --db data/ --rebuild-indexes
```

### Espaço em disco insuficiente

**Monitorammero:**
```bash
# Verificar uso do espaço
df -h

# Limpar dados antigos (periódico)
find data/ -name "*.log" -mtime +30 -delete
```

## Configuração de Ambiente

### Windows Issues

**Erro de compilação:**
```powershell
# Instalar Visual Studio Build Tools
# https://visualstudio.microsoft.com/visual-cpp-build-tools/

# Configurar vcvars
call "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\x64\vcvars64.bat"
```

### Linux Issues

**Permissões:**
```bash
# Fixar permissões de dados
sudo chown -R $USER:$USER data/
sudo chmod -R 755 data/
```

**Dependências faltando:**
```bash
# Ubuntu/Debian
sudo apt install build-essential libssl-dev llvm-dev clang

# Fedora/CentOS
sudo dnf install make gcc openssl-devel llvm clang
```

## Logging e Debug

### Níveis de Log

```rust
use log::{info, warn, error, debug, trace};

// Configuração
env_logger::Builder::new()
    .filter_level(log::LevelFilter::Debug)
    .init();
```

### Depurar transações

```rust
// Logs detalhados para transações
log::debug!("Processing transaction: {:?}", tx.hash());
log::debug!("Sender balance: {}", sender_balance);
log::debug!("Transaction fee: {}", tx.fee);
log::debug!("Gas used: {}", gas_used);
```

## Atualização e Migração

### Breaking Changes

**v0.1.0 → v0.2.0:**
1. Campo `state` movido para DEX manager
2. Mudança na estrutura de OrderSide
3. Remoção do Hash::random() method

**Migração:**
```bash
# Backup de configuração
cp config/ config.backup/

# Atualizar binários
cargo build --release

# Migrar dados se necessário
./migrate_data.sh
```

## Suporte DIY

### Recursos Úteis

- **GitHub Issues:** [Relatar bugs](https://github.com/iamsigint/rust-erbium/issues)
- **Documentação:** [docs.erbium.io](https://docs.erbium.io)
- **Discord:** [discord.gg/erbium](https://discord.gg/erbium)

### Ferramentas de Debug

```bash
# Analisador de logs
cargo run --bin log_analyzer -- logs/

# Monitor de performance
cargo run --bin performance_monitor

# Utilitário de rede
cargo run --bin network_debugger
```

## Prevenção de Problemas

### Manutenção Regular

```bash
# Limpeza semanal
0 2 * * 1 /path/to/cleanup.sh

# Backup diario
0 1 * * * /path/to/backup.sh

# Monitoramento contínuo
*/5 * * * * /path/to/health_check.sh
```

### Monitoramento

| Métrica | Threshold | Ação |
|---------|-----------|------|
| CPU Usage | >80% | Scale up |
| Memory Usage | >90% | Restart |
| Disk Space | <10% free | Cleanup |
| Error Rate | >1% | Alert |

## Contato de Emergência

**Critical Issues:**
- Email: security@erbium.io
- Response: < 1 hour

**General Support:**
- GitHub Discussions
- Discord #support channel

---

*Esta documentação é mantida atualizada. Última atualização: 2025-11-14*
