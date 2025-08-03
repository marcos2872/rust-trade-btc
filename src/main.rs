mod reader_csv;
mod redis_client;
mod trade_btc;

use crate::{reader_csv::ReaderBtcFile, redis_client::RedisClient, trade_btc::TradeSimulator};
use std::env;
use std::time::Instant;
use tracing::{info, error, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::{rolling, non_blocking};

fn init_logging() {
    // Criar diretório de logs se não existir
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    
    // Configurar appender para rotação diária
    let file_appender = rolling::daily("logs", "btc_trading.log");
    let (file_writer, guard) = non_blocking(file_appender);
    
    // IMPORTANTE: Manter o guard vivo para garantir que os logs sejam escritos
    std::mem::forget(guard);
    
    // Configurar filtros - aceitar variável de ambiente RUST_LOG ou usar padrão
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,btc_trading_simulator=debug"));
    
    // Configurar subscriber com múltiplas camadas
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true)
        )
        .with(
            fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(true)
                .with_level(true)
                .json()
        )
        .init();
    
    info!("🚀 Sistema de logging inicializado");
    info!("📁 Logs salvos em: logs/btc_trading.log.YYYY-MM-DD");
}

fn main() {
    dotenv::dotenv().ok();
    init_logging();
    
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "simulate" => {
                // Executar simulação de trade (pode continuar do estado salvo)
                info!("🎮 Iniciando simulação de trade (continuando do estado salvo se existir)...");
                if let Err(e) = trade_btc::run_trade_simulation() {
                    error!("❌ Erro na simulação: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "fresh" => {
                // Executar simulação nova (limpa estado anterior)
                info!("🧹 Iniciando simulação nova (sem estado anterior)...");
                if let Err(e) = trade_btc::run_fresh_simulation() {
                    error!("❌ Erro na simulação: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "clear" => {
                // Limpar apenas o arquivo de estado
                info!("🗑️  Limpando arquivo de estado...");
                if let Err(e) = TradeSimulator::clear_state_file() {
                    error!("❌ Erro ao limpar estado: {}", e);
                    std::process::exit(1);
                } else {
                    println!("✅ Arquivo de estado limpo com sucesso!");
                }
                return;
            }
            _ => {
                error!("❌ Comando não reconhecido: {}", args[1]);
                error!("Comandos disponíveis:");
                error!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
                error!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
                error!("  cargo run clear     - Limpar apenas o arquivo de estado");
                println!("❌ Comando não reconhecido. Use:");
                println!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
                println!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
                println!("  cargo run clear     - Limpar apenas o arquivo de estado");
                std::process::exit(1);
            }
        }
    }

    // Código original para carregar dados CSV
    let csv_path = "data/btc_historical_data.csv";
    info!("📁 Iniciando carregamento de dados CSV: {}", csv_path);
    let start_time = Instant::now();

    let redis = match RedisClient::from_env() {
        Ok(client) => {
            info!("✅ Cliente Redis criado com sucesso");
            client
        },
        Err(e) => {
            error!("❌ Erro ao criar cliente Redis: {}", e);
            eprintln!("❌ Erro ao criar cliente Redis: {}", e);
            std::process::exit(1);
        }
    };

    match ReaderBtcFile::read_btc_csv_file(csv_path) {
        Ok(data) => {
            let duration = start_time.elapsed();
            info!("✅ Dados CSV carregados: {} registros em {:.2?}", data.len(), duration);
            println!("✅ Dados carregados com sucesso: {} registros", data.len());
            println!("⏱️  Tempo de carregamento: {:.2?}", duration);

            let start_time = Instant::now();
            if let Err(e) = redis.set_all_btc(&data) {
                error!("❌ Erro ao salvar dados no Redis: {}", e);
                eprintln!("❌ Erro ao salvar no Redis: {}", e);
                std::process::exit(1);
            }
            let duration = start_time.elapsed();
            info!("✅ Dados salvos no Redis em {:.2?}", duration);
            println!("⏱️  Tempo de salvamento no Redis: {:.2?}", duration);

            info!("💡 Sistema pronto para uso");
            info!("💡 Comandos disponíveis:");
            info!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
            info!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
            info!("  cargo run clear     - Limpar apenas o arquivo de estado");
            println!("\n💡 Comandos disponíveis:");
            println!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
            println!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
            println!("  cargo run clear     - Limpar apenas o arquivo de estado");
        }
        Err(err) => {
            error!("❌ Erro ao carregar dados CSV: {}", err);
            println!("Error: {}", err);
        }
    }
}
