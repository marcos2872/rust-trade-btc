use redis::{Client, Commands, Connection, RedisError};
use std::env;
use std::thread;
use std::time::{Duration, Instant};

use crate::reader_csv::CsvBtcFile;

/// Configuração para conexão Redis
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub connection_timeout: Duration,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            max_retries: 3,
            retry_delay: Duration::from_secs(2),
            connection_timeout: Duration::from_secs(10),
        }
    }
}

impl RedisConfig {
    /// Cria configuração a partir de variáveis de ambiente
    pub fn from_env() -> Self {
        Self {
            url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://10.105.130.198:6379".to_string()),
            max_retries: env::var("REDIS_MAX_RETRIES")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3),
            retry_delay: Duration::from_secs(
                env::var("REDIS_RETRY_DELAY")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2),
            ),
            connection_timeout: Duration::from_secs(
                env::var("REDIS_TIMEOUT")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .unwrap_or(10),
            ),
        }
    }

    /// Cria configuração customizada
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ..Default::default()
        }
    }
}

/// Cliente Redis com funcionalidades robustas
pub struct RedisClient {
    client: Client,
    config: RedisConfig,
}

impl RedisClient {
    /// Cria um novo cliente Redis
    pub fn new(config: RedisConfig) -> Result<Self, RedisClientError> {
        println!("🔗 Criando cliente Redis para: {}", config.url);

        let client = Client::open(config.url.as_str()).map_err(|e| {
            RedisClientError::ConnectionError(format!("Falha ao criar cliente: {}", e))
        })?;

        let redis_client = Self { client, config };

        // Testa a conexão na criação
        redis_client.test_connection()?;

        Ok(redis_client)
    }

    /// Cria cliente com configuração padrão
    pub fn default() -> Result<Self, RedisClientError> {
        Self::new(RedisConfig::default())
    }

    /// Cria cliente a partir de variáveis de ambiente
    pub fn from_env() -> Result<Self, RedisClientError> {
        Self::new(RedisConfig::from_env())
    }

    /// Testa a conexão com Redis
    pub fn test_connection(&self) -> Result<(), RedisClientError> {
        println!("🧪 Testando conexão Redis...");

        let start_time = Instant::now();

        for attempt in 1..=self.config.max_retries {
            if start_time.elapsed() > self.config.connection_timeout {
                return Err(RedisClientError::TimeoutError);
            }

            println!("🔄 Tentativa {} de {}", attempt, self.config.max_retries);

            match self.client.get_connection() {
                Ok(mut con) => match redis::cmd("PING").query::<String>(&mut con) {
                    Ok(_) => {
                        println!("✅ Redis conectado com sucesso na tentativa {}", attempt);
                        return Ok(());
                    }
                    Err(e) => {
                        eprintln!("⚠️  PING falhou: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("⚠️  Conexão falhou na tentativa {}: {}", attempt, e);
                }
            }

            if attempt < self.config.max_retries {
                println!(
                    "⏳ Aguardando {:?} antes da próxima tentativa...",
                    self.config.retry_delay
                );
                thread::sleep(self.config.retry_delay);
            }
        }

        Err(RedisClientError::MaxRetriesReached(self.config.max_retries))
    }

    /// Obtém uma conexão Redis
    pub fn get_connection(&self) -> Result<Connection, RedisClientError> {
        self.client
            .get_connection()
            .map_err(|e| RedisClientError::ConnectionError(e.to_string()))
    }

    /// Executa PING
    pub fn ping(&self) -> Result<String, RedisClientError> {
        let mut con = self.get_connection()?;
        redis::cmd("PING")
            .query::<String>(&mut con)
            .map_err(|e| RedisClientError::OperationError(format!("PING falhou: {}", e)))
    }

    pub fn set_all_btc(&self, data: &[CsvBtcFile]) -> Result<(), Box<dyn std::error::Error>> {
        let mut con = self.client.get_connection()?;
        let batch_size = 20000; // Ajuste conforme sua memória RAM disponível

        let mut total_records_saved = 0;
        let mut total_records_skipped = 0;

        println!(
            "🚀 Iniciando processamento com batches de {} registros",
            batch_size
        );

        for (batch_num, chunk) in data.chunks(batch_size).enumerate() {
            let batch_start_index = batch_num * batch_size;

            // 1. Gera todas as chaves do batch atual
            let keys: Vec<String> = (0..chunk.len())
                .map(|i| format!("btc_{}", batch_start_index + i))
                .collect();

            // 2. Busca todos os valores existentes de uma vez com MGET
            let existing_values: Vec<Option<String>> = if !keys.is_empty() {
                con.get(&keys)?
            } else {
                vec![]
            };

            // 3. Prepara pipeline apenas com registros que precisam ser atualizados
            let mut pipe = redis::pipe();
            let mut records_to_save = 0;
            let mut records_skipped = 0;

            for (i, record) in chunk.iter().enumerate() {
                let json_data = serde_json::to_string(record)?;

                // Verifica se deve salvar comparando com valor existente
                let should_save = match existing_values.get(i) {
                    Some(Some(existing_data)) => json_data != *existing_data,
                    Some(None) => true, // Chave não existe
                    None => true,       // Índice fora do range (não deveria acontecer)
                };

                if should_save {
                    pipe.set(&keys[i], &json_data);
                    records_to_save += 1;
                } else {
                    records_skipped += 1;
                }
            }

            // 4. Executa o pipeline apenas se houver registros para salvar
            if records_to_save > 0 {
                let _: Vec<()> = pipe.query(&mut con)?;
            }

            // 5. Atualiza contadores totais
            total_records_saved += records_to_save;
            total_records_skipped += records_skipped;

            // 6. Log de progresso por batch
            let progress = ((batch_num + 1) * batch_size).min(data.len());
            println!(
                "📦 Batch {}: {}/{} - Salvos: {} | Ignorados: {} | Progresso: {}/{}",
                batch_num + 1,
                records_to_save,
                chunk.len(),
                records_to_save,
                records_skipped,
                progress,
                data.len()
            );
        }

        // 7. Relatório final
        if total_records_saved > 0 {
            println!(
                "✅ {} registros salvos no Redis em batches",
                total_records_saved
            );
        }
        if total_records_skipped > 0 {
            println!(
                "⏭️  {} registros já existiam e foram ignorados",
                total_records_skipped
            );
        }
        println!(
            "📊 Total processado: {} registros (btc_0 a btc_{})",
            data.len(),
            data.len() - 1
        );

        Ok(())
    }
    // Método para carregar por índice
    pub fn load_by_index(
        &self,
        index: usize,
    ) -> Result<Option<CsvBtcFile>, Box<dyn std::error::Error>> {
        let mut con = self.client.get_connection()?;
        let key = format!("btc_{}", index);

        match con.get::<String, String>(key) {
            Ok(json_data) => {
                let record: CsvBtcFile = serde_json::from_str(&json_data)?;
                Ok(Some(record))
            }
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    /// Método para carregar dados BTC por chave
    pub fn get_btc_data(
        &self,
        key: &str,
    ) -> Result<Option<CsvBtcFile>, Box<dyn std::error::Error>> {
        let mut con = self.client.get_connection()?;

        match con.get::<String, String>(key.to_string()) {
            Ok(json_data) => {
                let record: CsvBtcFile = serde_json::from_str(&json_data)?;
                Ok(Some(record))
            }
            Err(e) => {
                if e.kind() == redis::ErrorKind::TypeError {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

/// Erros personalizados para o cliente Redis
#[derive(Debug)]
pub enum RedisClientError {
    ConnectionError(String),
    OperationError(String),
    TimeoutError,
    MaxRetriesReached(u32),
}

impl std::fmt::Display for RedisClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RedisClientError::ConnectionError(msg) => write!(f, "Erro de conexão Redis: {}", msg),
            RedisClientError::OperationError(msg) => write!(f, "Erro de operação Redis: {}", msg),
            RedisClientError::TimeoutError => write!(f, "Timeout na conexão Redis"),
            RedisClientError::MaxRetriesReached(retries) => {
                write!(
                    f,
                    "Máximo de tentativas ({}) atingido para conexão Redis",
                    retries
                )
            }
        }
    }
}

impl std::error::Error for RedisClientError {}

impl From<RedisError> for RedisClientError {
    fn from(err: RedisError) -> Self {
        RedisClientError::OperationError(err.to_string())
    }
}
