// src/network/sync.rs

use crate::core::{Block, Blockchain};
use crate::utils::error::{Result, BlockchainError};
use crate::network::p2p::P2PNode;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Status da sincronização da blockchain
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Nó está sincronizado com a rede
    Synced,
    /// Nó está sincronizando, com o número do bloco atual
    Syncing(u64),
    /// Nó está atrasado, com o número de blocos faltantes
    Behind(u64),
    /// Nó está inicializando
    Initializing,
}

/// Gerenciador de sincronização da blockchain
pub struct SyncManager {
    blockchain: Arc<Mutex<Blockchain>>,
    _p2p_node: Arc<Mutex<P2PNode>>,
    status: SyncStatus,
    highest_seen_block: u64,
}

impl SyncManager {
    /// Cria um novo gerenciador de sincronização
    pub fn new(blockchain: Arc<Mutex<Blockchain>>, p2p_node: Arc<Mutex<P2PNode>>) -> Self {
        Self {
            blockchain,
            _p2p_node: p2p_node,
            status: SyncStatus::Initializing,
            highest_seen_block: 0,
        }
    }

    /// Inicia o processo de sincronização
    pub async fn start_sync(&mut self) -> Result<()> {
        log::info!("Iniciando sincronização da blockchain");
        
        // Atualiza o status para sincronizando
        self.status = SyncStatus::Syncing(0);
        
        // Solicita o último bloco da rede
        self.request_latest_block().await?;
        
        // Inicia o loop de sincronização
        self.sync_loop().await
    }
    
    /// Solicita o último bloco da rede
    async fn request_latest_block(&mut self) -> Result<()> {
        log::debug!("Solicitando último bloco da rede");
        
        // Em uma implementação real, isso enviaria uma mensagem para a rede
        // Aqui, apenas simulamos o recebimento de uma resposta
        
        // Atualiza o bloco mais alto visto
        let blockchain = self.blockchain.lock().await;
        let current_height = blockchain.get_block_height() as u64;
        drop(blockchain);
        
        // Simula que vimos um bloco mais alto na rede
        self.highest_seen_block = current_height + 10;
        
        Ok(())
    }
    
    /// Loop principal de sincronização
    async fn sync_loop(&mut self) -> Result<()> {
        log::info!("Iniciando loop de sincronização");
        
        loop {
            // Verifica o estado atual da blockchain
            let blockchain = self.blockchain.lock().await;
            let current_height = blockchain.get_block_height() as u64;
            drop(blockchain);
            
            if current_height >= self.highest_seen_block {
                // Estamos sincronizados
                self.status = SyncStatus::Synced;
                log::info!("Blockchain sincronizada na altura {}", current_height);
                break;
            } else {
                // Ainda precisamos sincronizar
                let blocks_behind = self.highest_seen_block - current_height;
                self.status = SyncStatus::Behind(blocks_behind);
                
                log::info!(
                    "Sincronizando: altura atual={}, altura alvo={}, blocos faltantes={}",
                    current_height,
                    self.highest_seen_block,
                    blocks_behind
                );
                
                // Solicita o próximo lote de blocos
                self.request_blocks(current_height + 1, 50).await?;
                
                // Simula um atraso para não sobrecarregar o sistema
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        Ok(())
    }
    
    /// Solicita um lote de blocos da rede
    async fn request_blocks(&self, start_height: u64, count: u64) -> Result<()> {
        log::debug!("Solicitando blocos {} a {}", start_height, start_height + count - 1);
        
        // Em uma implementação real, isso enviaria uma mensagem para a rede
        // e processaria os blocos recebidos
        
        // Simula o processamento de blocos recebidos
        self.process_received_blocks(start_height, count).await
    }
    
    /// Processa blocos recebidos da rede
    async fn process_received_blocks(&self, start_height: u64, count: u64) -> Result<()> {
        log::debug!("Processando {} blocos a partir da altura {}", count, start_height);
        
        // Em uma implementação real, isso validaria e adicionaria os blocos à blockchain
        // Aqui, apenas simulamos o processamento
        
        // Simula a adição de blocos à blockchain
        let mut blockchain = self.blockchain.lock().await;
        
        for i in 0..count {
            let height = start_height + i;
            
            // Verifica se já atingimos o bloco mais alto visto
            if height > self.highest_seen_block {
                break;
            }
            
            // Simula a criação de um bloco
            if let Some(last_block) = blockchain.get_latest_block() {
                let transactions = Vec::new(); // Sem transações nesta simulação
                
                // Cria um novo bloco baseado no último
                let mut new_block = Block::new(
                    height,
                    last_block.hash(),
                    transactions,
                    "sync_validator".to_string(),
                    blockchain.get_current_difficulty(),
                );
                
                // Simula a assinatura do bloco (em uma implementação real, isso seria feito corretamente)
                let dummy_key = vec![0u8; 32];
                new_block.sign(&dummy_key)?;
                
                // Adiciona o bloco à blockchain
                blockchain.add_block(new_block)?;
                
                log::debug!("Adicionado bloco de sincronização na altura {}", height);
            } else {
                return Err(BlockchainError::InvalidBlock("Não foi possível obter o último bloco".to_string()));
            }
        }
        
        Ok(())
    }
    
    /// Retorna o status atual de sincronização
    pub fn get_status(&self) -> SyncStatus {
        self.status.clone()
    }
    
    /// Verifica se o nó está sincronizado
    pub fn is_synced(&self) -> bool {
        self.status == SyncStatus::Synced
    }
}
