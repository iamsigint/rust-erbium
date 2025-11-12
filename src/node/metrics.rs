// src/node/metrics.rs

use metrics::{counter, gauge, histogram, register_counter, register_gauge, register_histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::net::SocketAddr;
use std::time::Duration;
use warp::Filter;

/// Configuração para o servidor de métricas
pub struct MetricsConfig {
    /// Endereço para o servidor de métricas
    pub listen_addr: SocketAddr,
    /// Prefixo para as métricas
    pub metrics_prefix: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:9615".parse().unwrap(),
            metrics_prefix: "erbium".to_string(),
        }
    }
}

/// Gerenciador de métricas
pub struct MetricsService {
    handle: PrometheusHandle,
    config: MetricsConfig,
}

impl MetricsService {
    /// Cria um novo serviço de métricas
    pub fn new(config: MetricsConfig) -> Self {
        // Configurar o exportador Prometheus
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder().expect("install prometheus recorder");
        
        Self { handle, config }
    }
    
    /// Inicia o servidor de métricas
    pub fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let metrics_addr = self.config.listen_addr;
        let handle = self.handle.clone();

        // Iniciar o servidor em uma nova task async
        tokio::spawn(async move {
            let route = warp::path("metrics").map(move || handle.render());
            warp::serve(route).run(metrics_addr).await;
        });

        log::info!("Metrics server started at {}", metrics_addr);
        Ok(())
    }
    
    /// Registra métricas padrão
    pub fn register_default_metrics(&self) {
        // Contadores
        register_counter!("block_count");
        register_counter!("transaction_count");
        register_counter!("network_message_count");
        
        // Gauges
        register_gauge!("block_height");
        register_gauge!("peer_count");
        register_gauge!("memory_usage_bytes");
        
        // Histogramas
        register_histogram!("block_time");
        register_histogram!("transaction_processing_time");
    }
    
    /// Incrementa um contador
    pub fn increment_counter(&self, name: &'static str, value: u64) {
        counter!(name, value);
    }
    
    /// Define um valor para um gauge
    pub fn set_gauge(&self, name: &'static str, value: f64) {
        gauge!(name, value);
    }
    
    /// Registra um valor em um histograma
    pub fn observe_histogram(&self, name: &'static str, value: f64) {
        histogram!(name, value);
    }
    
    /// Registra métricas do sistema
    pub fn register_system_metrics(&self) {
        // Registrar métricas do sistema a cada 15 segundos
        let _handle = self.handle.clone();
        std::thread::spawn(move || {
            loop {
                // Exemplo: registrar uso de memória
                // Em uma implementação real, você usaria uma biblioteca como sysinfo
                gauge!("memory_usage_bytes", 1000.0);
                
                // Dormir por 15 segundos
                std::thread::sleep(Duration::from_secs(15));
            }
        });
    }
}
