mod reader_csv;
mod redis_client;
mod trade_btc;

use crate::{reader_csv::ReaderBtcFile, redis_client::RedisClient};
use std::env;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "simulate" {
        // Executar simulação de trade
        println!("🎮 Iniciando simulação de trade...");
        if let Err(e) = trade_btc::run_trade_simulation() {
            eprintln!("❌ Erro na simulação: {}", e);
            std::process::exit(1);
        }
        return;
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

            println!("\n💡 Para executar a simulação de trade, use: cargo run simulate");
        }
        Err(err) => {
            println!("Error: {}", err);
        }
    }
}
