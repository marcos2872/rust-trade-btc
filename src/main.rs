mod reader_csv;
mod redis_client;
mod trade_btc;
mod llm_client;
mod market_analysis;
mod decision_engine;

use crate::{reader_csv::ReaderBtcFile, redis_client::RedisClient};
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "simulate" => {
                // Executar simulação de trade original
                println!("🎮 Iniciando simulação de trade com LLM...");
                let rt = tokio::runtime::Runtime::new().unwrap();
                if let Err(e) = rt.block_on(async {
                    trade_btc::run_trade_simulation().await
                }) {
                    eprintln!("❌ Erro na simulação: {}", e);
                    std::process::exit(1);
                }
                return;
            }
            "llm" => {
                // Testar conexão com LLM
                println!("🤖 Testando conexão com LLM...");
                let rt = tokio::runtime::Runtime::new().unwrap();
                let llm_client = llm_client::LlmClient::from_env();
                
                match rt.block_on(llm_client.test_connection()) {
                    Ok(true) => {
                        println!("✅ LLM conectado com sucesso!");
                        
                        // Fazer uma consulta de teste
                        let test_prompt = "Olá! Você é um especialista em trading de Bitcoin. Responda apenas: 'Sistema funcionando.'";
                        match rt.block_on(llm_client.generate(test_prompt)) {
                            Ok(response) => {
                                println!("🤖 Resposta do LLM: {}", response.trim());
                            }
                            Err(e) => {
                                println!("⚠️  Erro ao testar geração: {}", e);
                            }
                        }
                    }
                    Ok(false) => {
                        println!("❌ LLM não está respondendo corretamente");
                    }
                    Err(e) => {
                        println!("❌ Erro ao conectar com LLM: {}", e);
                    }
                }
                return;
            }
            _ => {
                println!("❌ Comando não reconhecido. Use:");
                println!("  cargo run simulate  - Simulação de trading com LLM");
                println!("  cargo run llm       - Testar conexão com LLM");
                std::process::exit(1);
            }
        }
    }

    // Código original para carregar dados CSV
    let csv_path = "data/btc_historical_data.csv";
    let start_time = Instant::now();

    let redis = match RedisClient::from_env() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("❌ Erro ao criar cliente Redis: {}", e);
            std::process::exit(1);
        }
    };

    match ReaderBtcFile::read_btc_csv_file(csv_path) {
        Ok(data) => {
            let duration = start_time.elapsed();
            println!("✅ Dados carregados com sucesso: {} registros", data.len());
            println!("⏱️  Tempo de carregamento: {:.2?}", duration);

            let start_time = Instant::now();
            if let Err(e) = redis.set_all_btc(&data) {
                eprintln!("❌ Erro ao salvar no Redis: {}", e);
                std::process::exit(1);
            }
            let duration = start_time.elapsed();
            println!("⏱️  Tempo de salvamento no Redis: {:.2?}", duration);

            println!("\n💡 Comandos disponíveis:");
            println!("  cargo run simulate  - Simulação de trading com LLM");
            println!("  cargo run llm       - Testar conexão com LLM Llama3:8b");
        }
        Err(err) => {
            println!("Error: {}", err);
        }
    }
}
