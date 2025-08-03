mod reader_csv;
mod redis_client;
mod trade_btc;

use crate::{reader_csv::ReaderBtcFile, redis_client::RedisClient, trade_btc::TradeSimulator};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
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

fn follow_logs() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Acompanhando logs da simulação em tempo real...");
    println!("💡 Use Ctrl+C para parar de acompanhar\n");

    let today = chrono::Utc::now().format("%Y-%m-%d");
    let log_file_path = format!("logs/btc_trading.log.{}", today);
    
    if !Path::new(&log_file_path).exists() {
        println!("❌ Arquivo de log não encontrado: {}", log_file_path);
        println!("💡 Certifique-se de que a simulação está rodando");
        return Ok(());
    }

    println!("📂 Lendo arquivo: {}", log_file_path);
    println!("{}", "=".repeat(80));

    let mut file = fs::File::open(&log_file_path)?;
    file.seek(SeekFrom::End(0))?; // Começar do final do arquivo
    
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // Arquivo não cresceu, aguardar um pouco
                thread::sleep(Duration::from_millis(100));
            }
            Ok(_) => {
                // Nova linha encontrada, processar
                if let Ok(log_entry) = serde_json::from_str::<serde_json::Value>(&line.trim()) {
                    if let (Some(timestamp), Some(level), Some(message)) = (
                        log_entry["timestamp"].as_str(),
                        log_entry["level"].as_str(),
                        log_entry["message"].as_str(),
                    ) {
                        let level_color = match level {
                            "INFO" => "32", // Verde
                            "WARN" => "33", // Amarelo
                            "ERROR" => "31", // Vermelho
                            "DEBUG" => "36", // Ciano
                            _ => "37", // Branco
                        };
                        
                        println!("\x1b[{}m[{}] {}: {}\x1b[0m", 
                                level_color, 
                                timestamp.get(11..19).unwrap_or("--:--:--"), 
                                level, 
                                message);
                    }
                }
            }
            Err(_) => {
                // Erro na leitura, aguardar e tentar novamente
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

fn start_daemon() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Iniciando simulação em modo daemon...");
    
    // Verificar se já existe um processo rodando
    if is_simulation_running() {
        println!("⚠️  Uma simulação já está rodando!");
        println!("💡 Use 'cargo run logs' para acompanhar os logs");
        println!("💡 Use 'cargo run status' para verificar o status");
        return Ok(());
    }
    
    let exe_path = env::current_exe()?;
    let current_dir = env::current_dir()?;
    
    let mut child = Command::new(&exe_path)
        .arg("simulate")
        .current_dir(&current_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    
    // Salvar PID do processo
    let pid = child.id();
    fs::write("simulation.pid", pid.to_string())?;
    
    println!("✅ Simulação iniciada em background (PID: {})", pid);
    println!("📂 Arquivo de estado salvo a cada 30 segundos");
    println!("📊 Use 'cargo run logs' para acompanhar em tempo real");
    println!("🛑 Use 'cargo run stop' para parar a simulação");
    
    Ok(())
}

fn stop_daemon() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(pid_str) = fs::read_to_string("simulation.pid") {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            println!("🛑 Parando simulação (PID: {})...", pid);
            
            #[cfg(unix)]
            {
                use std::process;
                Command::new("kill")
                    .arg(pid.to_string())
                    .output()?;
            }
            
            #[cfg(windows)]
            {
                Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/F"])
                    .output()?;
            }
            
            fs::remove_file("simulation.pid").ok();
            println!("✅ Simulação parada");
            println!("💾 Estado foi salvo automaticamente");
        } else {
            println!("❌ PID inválido no arquivo");
        }
    } else {
        println!("❌ Nenhuma simulação em execução encontrada");
    }
    
    Ok(())
}

fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Status da Simulação");
    println!("{}", "=".repeat(50));
    
    // Verificar se existe processo rodando
    if is_simulation_running() {
        if let Ok(pid_str) = fs::read_to_string("simulation.pid") {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                println!("🟢 Status: RODANDO (PID: {})", pid);
            }
        } else {
            println!("🟢 Status: RODANDO");
        }
    } else {
        println!("🔴 Status: PARADO");
    }
    
    // Verificar se existe arquivo de estado
    if Path::new("simulation_state.json").exists() {
        if let Ok(state_data) = fs::read_to_string("simulation_state.json") {
            if let Ok(state) = serde_json::from_str::<serde_json::Value>(&state_data) {
                if let (Some(current_time), Some(data_index), Some(saldo_fiat), Some(saldo_btc)) = (
                    state["current_time"].as_str(),
                    state["data_index"].as_u64(),
                    state["saldo_fiat"].as_f64(),
                    state["saldo_btc"].as_f64(),
                ) {
                    println!("💾 Estado salvo: SIM");
                    println!("📅 Última data: {}", current_time.get(..19).unwrap_or("--"));
                    println!("📊 Índice atual: {}", data_index);
                    println!("💰 Saldo Fiat: ${:.2}", saldo_fiat);
                    println!("₿  Saldo BTC: {:.6} BTC", saldo_btc);
                } else {
                    println!("💾 Estado salvo: SIM (formato inválido)");
                }
            }
        }
    } else {
        println!("💾 Estado salvo: NÃO");
    }
    
    // Verificar logs
    let today = chrono::Utc::now().format("%Y-%m-%d");
    let log_file_path = format!("logs/btc_trading.log.{}", today);
    if Path::new(&log_file_path).exists() {
        if let Ok(metadata) = fs::metadata(&log_file_path) {
            println!("📄 Log de hoje: {} ({} bytes)", log_file_path, metadata.len());
        }
    } else {
        println!("📄 Log de hoje: Não encontrado");
    }
    
    println!("\n💡 Comandos disponíveis:");
    println!("  cargo run daemon  - Iniciar em background");
    println!("  cargo run logs    - Acompanhar logs");
    println!("  cargo run stop    - Parar daemon");
    
    Ok(())
}

fn is_simulation_running() -> bool {
    if let Ok(pid_str) = fs::read_to_string("simulation.pid") {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            #[cfg(unix)]
            {
                return Command::new("kill")
                    .args(["-0", &pid.to_string()])
                    .output()
                    .map(|output| output.status.success())
                    .unwrap_or(false);
            }
            
            #[cfg(windows)]
            {
                return Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid)])
                    .output()
                    .map(|output| {
                        String::from_utf8_lossy(&output.stdout).contains(&pid.to_string())
                    })
                    .unwrap_or(false);
            }
        }
    }
    false
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
            "daemon" => {
                // Iniciar simulação em background
                if let Err(e) = start_daemon() {
                    error!("❌ Erro ao iniciar daemon: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "logs" => {
                // Acompanhar logs em tempo real
                if let Err(e) = follow_logs() {
                    error!("❌ Erro ao acompanhar logs: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "stop" => {
                // Parar daemon
                if let Err(e) = stop_daemon() {
                    error!("❌ Erro ao parar daemon: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "status" => {
                // Mostrar status da simulação
                if let Err(e) = show_status() {
                    error!("❌ Erro ao mostrar status: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            _ => {
                error!("❌ Comando não reconhecido: {}", args[1]);
                error!("Comandos disponíveis:");
                error!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
                error!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
                error!("  cargo run daemon    - Iniciar simulação em background");
                error!("  cargo run logs      - Acompanhar logs em tempo real");
                error!("  cargo run stop      - Parar simulação em background");
                error!("  cargo run status    - Verificar status da simulação");
                error!("  cargo run clear     - Limpar apenas o arquivo de estado");
                println!("❌ Comando não reconhecido. Use:");
                println!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
                println!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
                println!("  cargo run daemon    - Iniciar simulação em background");
                println!("  cargo run logs      - Acompanhar logs em tempo real");
                println!("  cargo run stop      - Parar simulação em background");
                println!("  cargo run status    - Verificar status da simulação");
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
            info!("  cargo run daemon    - Iniciar simulação em background");
            info!("  cargo run logs      - Acompanhar logs em tempo real");
            info!("  cargo run stop      - Parar simulação em background");
            info!("  cargo run status    - Verificar status da simulação");
            info!("  cargo run clear     - Limpar apenas o arquivo de estado");
            println!("\n💡 Comandos disponíveis:");
            println!("  cargo run simulate  - Continuar simulação do estado salvo (ou iniciar nova)");
            println!("  cargo run fresh     - Iniciar simulação nova (limpa estado anterior)");
            println!("  cargo run daemon    - Iniciar simulação em background");
            println!("  cargo run logs      - Acompanhar logs em tempo real");
            println!("  cargo run stop      - Parar simulação em background");
            println!("  cargo run status    - Verificar status da simulação");
            println!("  cargo run clear     - Limpar apenas o arquivo de estado");
        }
        Err(err) => {
            error!("❌ Erro ao carregar dados CSV: {}", err);
            println!("Error: {}", err);
        }
    }
}
