// src/node/node_metrics.rs

use crate::node::metrics::{MetricsConfig, MetricsService};
use crate::utils::error::{BlockchainError, Result};

/// Métricas do nó Erbium
pub struct NodeMetrics {
    service: MetricsService,
}

impl NodeMetrics {
    /// Cria uma nova instância de NodeMetrics
    pub fn new() -> Result<Self> {
        let config = MetricsConfig::default();
        let service = MetricsService::new(config);

        // Registrar métricas padrão
        service.register_default_metrics();
        service.register_system_metrics();

        Ok(Self { service })
    }

    /// Inicia o servidor de métricas
    pub fn start_server(&self, addr: &str) -> Result<()> {
        // Converter o endereço de string para SocketAddr
        let socket_addr = match addr.parse() {
            Ok(addr) => addr,
            Err(e) => {
                return Err(BlockchainError::Other(format!(
                    "Invalid metrics address: {}",
                    e
                )))
            }
        };

        // Criar uma nova configuração com o endereço fornecido
        let mut config = MetricsConfig::default();
        config.listen_addr = socket_addr;

        // Iniciar o servidor
        match self.service.start_server() {
            Ok(_) => Ok(()),
            Err(e) => Err(BlockchainError::Other(format!(
                "Failed to start metrics server: {}",
                e
            ))),
        }
    }

    /// Registra uma nova transação
    pub fn register_transaction(&self) {
        self.service.increment_counter("transaction_count", 1);
    }

    /// Registra um novo bloco
    pub fn register_block(&self, height: u64) {
        self.service.increment_counter("block_count", 1);
        self.service.set_gauge("block_height", height as f64);
    }

    /// Registra o tempo de processamento de um bloco
    pub fn register_block_time(&self, time_seconds: f64) {
        self.service.observe_histogram("block_time", time_seconds);
    }

    /// Registra o tempo de processamento de uma transação
    pub fn register_transaction_time(&self, time_seconds: f64) {
        self.service
            .observe_histogram("transaction_processing_time", time_seconds);
    }

    /// Atualiza a contagem de peers
    pub fn update_peer_count(&self, count: usize) {
        self.service.set_gauge("peer_count", count as f64);
    }

    /// Registra uma mensagem de rede
    pub fn register_network_message(&self) {
        self.service.increment_counter("network_message_count", 1);
    }
}
