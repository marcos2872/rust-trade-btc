use crate::{reader_csv::CsvBtcFile, redis_client::RedisClient};
use chrono::{DateTime, Utc};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone)]
pub struct BuyOrder {
    pub id: u32,
    pub btc_quantity: f64,
    pub buy_price: f64,
    pub buy_time: DateTime<Utc>,
    pub invested_amount: f64,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u32,
    pub transaction_type: String, // "BUY" or "SELL"
    pub btc_quantity: f64,
    pub price: f64,
    pub time: DateTime<Utc>,
    pub amount: f64,
    pub profit_loss: Option<f64>,
    pub buy_order_id: Option<u32>, // Para vendas, referencia a ordem de compra
}

#[derive(Debug, Clone)]
pub struct TradeConfig {
    pub initial_balance: f64,                 // Saldo inicial em USD
    pub max_loss_percentage: f64,             // Perda máxima aceitável (%)
    pub trade_percentage: f64,                // Percentual do saldo para usar em cada trade
    pub stop_loss_percentage: f64,            // Stop loss (%)
    pub take_profit_percentage: f64,          // Take profit (%)
    pub percentual_queda_para_comprar: f64,   // Percentual de queda para comprar mais
    pub preco_inicial_de_compra: Option<f64>, // Preço inicial de referência para primeira compra
}

impl Default for TradeConfig {
    fn default() -> Self {
        Self {
            initial_balance: 10000.0,
            max_loss_percentage: 20.0,
            trade_percentage: 10.0,
            stop_loss_percentage: 5.0,
            take_profit_percentage: 10.0,
            percentual_queda_para_comprar: 5.0,
            preco_inicial_de_compra: None,
        }
    }
}

#[derive(Debug)]
pub struct TradeStats {
    pub current_balance: f64,
    pub btc_balance: f64,
    pub total_trades: u32,
    pub winning_trades: u32,
    pub losing_trades: u32,
    pub total_profit: f64,
    pub total_loss: f64,
    pub max_drawdown: f64,
    pub current_drawdown: f64,
}

impl TradeStats {
    pub fn new(initial_balance: f64) -> Self {
        Self {
            current_balance: initial_balance,
            btc_balance: 0.0,
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            total_profit: 0.0,
            total_loss: 0.0,
            max_drawdown: 0.0,
            current_drawdown: 0.0,
        }
    }

    pub fn update_drawdown(&mut self, initial_balance: f64, current_price: f64) {
        let current_total = self.current_balance + (self.btc_balance * current_price);
        self.current_drawdown = ((initial_balance - current_total) / initial_balance) * 100.0;
        if self.current_drawdown > self.max_drawdown {
            self.max_drawdown = self.current_drawdown;
        }
    }

    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            (self.winning_trades as f64 / self.total_trades as f64) * 100.0
        }
    }

    pub fn net_profit(&self) -> f64 {
        self.total_profit - self.total_loss
    }
}

pub struct TradeSimulator {
    redis_client: RedisClient,
    config: TradeConfig,
    stats: TradeStats,
    current_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    data_index: usize,
    total_records: usize,
    // Variáveis da nova estratégia DCA
    saldo_fiat: f64,
    saldo_btc: f64,
    preco_anterior: Option<f64>, // Para detectar quedas
    preco_pico_recente: f64,     // Para detectar quedas significativas
    total_investido: f64,        // Total já investido em BTC
    // Sistema de ordens individuais
    buy_orders: Vec<BuyOrder>, // Lista de ordens de compra ativas
    transaction_history: Vec<Transaction>, // Histórico completo de transações
    next_order_id: u32,        // ID da próxima ordem
    next_transaction_id: u32,  // ID da próxima transação
    // Contador de quedas para comprar apenas a cada 3 quedas
    quedas_detectadas: u32,   // Contador de quedas consecutivas
    quedas_para_comprar: u32, // Comprar apenas a cada N quedas
}

impl TradeSimulator {
    pub fn new(
        redis_client: RedisClient,
        config: TradeConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // let start_time =
        //     DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00")?.with_timezone(&Utc);
        let start_time =
            DateTime::parse_from_rfc3339("2018-01-01T00:00:00+00:00")?.with_timezone(&Utc);
        // let end_time =
        //     DateTime::parse_from_rfc3339("2024-01-01T18:43:00+00:00")?.with_timezone(&Utc);
        let end_time =
            DateTime::parse_from_rfc3339("2025-07-22T18:43:00+00:00")?.with_timezone(&Utc);

        // Estimar total de registros (aproximadamente um por hora)
        let duration = end_time.signed_duration_since(start_time);
        let estimated_records = duration.num_hours() as usize;

        Ok(Self {
            redis_client,
            stats: TradeStats::new(config.initial_balance),
            saldo_fiat: config.initial_balance,
            saldo_btc: 0.0,
            preco_anterior: None,
            preco_pico_recente: 0.0,
            total_investido: 0.0,
            buy_orders: Vec::new(),
            transaction_history: Vec::new(),
            next_order_id: 1,
            next_transaction_id: 1,
            quedas_detectadas: 0,
            quedas_para_comprar: 3, // Comprar apenas a cada 3 quedas
            config,
            current_time: start_time,
            end_time,
            data_index: 0,
            total_records: estimated_records,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🚀 Iniciando simulador de trade BTC");
        info!("💰 Saldo inicial: ${:.2}", self.config.initial_balance);
        info!(
            "📊 Perda máxima aceitável: {:.1}%",
            self.config.max_loss_percentage
        );
        info!(
            "🎯 Stop Loss: {:.1}% | Take Profit: {:.1}%",
            self.config.stop_loss_percentage, self.config.take_profit_percentage
        );
        info!("⏰ Período: {} até {}", self.current_time, self.end_time);

        // Manter println para interface do usuário
        println!("🚀 Iniciando simulador de trade BTC");
        println!("💰 Saldo inicial: ${:.2}", self.config.initial_balance);
        println!(
            "📊 Perda máxima aceitável: {:.1}%",
            self.config.max_loss_percentage
        );
        println!(
            "🎯 Stop Loss: {:.1}% | Take Profit: {:.1}%",
            self.config.stop_loss_percentage, self.config.take_profit_percentage
        );
        println!("⏰ Período: {} até {}", self.current_time, self.end_time);
        println!("{}", "=".repeat(80));

        let start_simulation = Instant::now();
        let mut last_display = Instant::now();

        let mut consecutive_no_data = 0;
        const MAX_NO_DATA_ITERATIONS: usize = 1000; // Parar após 1000 iterações sem dados

        while self.current_time < self.end_time {
            // Buscar dados do Redis para o índice atual
            if let Some(btc_data) = self.get_current_btc_data()? {
                consecutive_no_data = 0; // Reset contador quando encontra dados
                self.process_tick(&btc_data)?;

                // Atualizar display a cada 5 segundos de simulação
                if last_display.elapsed() >= Duration::from_secs(5) {
                    self.display_status(&btc_data);
                    last_display = Instant::now();
                }
            } else {
                consecutive_no_data += 1;

                // Log a cada 100 iterações sem dados
                if consecutive_no_data % 100 == 0 {
                    warn!(
                        "⚠️  {} iterações sem dados - Índice: {} - Data: {} - Progresso: {:.1}%",
                        consecutive_no_data,
                        self.data_index,
                        self.current_time.format("%Y-%m-%d %H:%M"),
                        (self.data_index as f64 / self.total_records as f64) * 100.0
                    );
                    // Manter println para interface do usuário
                    println!(
                        "⚠️  {} iterações sem dados - Índice: {} - Data: {} - Progresso: {:.1}%",
                        consecutive_no_data,
                        self.data_index,
                        self.current_time.format("%Y-%m-%d %H:%M"),
                        (self.data_index as f64 / self.total_records as f64) * 100.0
                    );
                }

                // Parar se muitas iterações consecutivas sem dados
                if consecutive_no_data >= MAX_NO_DATA_ITERATIONS {
                    error!(
                        "🛑 Simulação parada: {} iterações consecutivas sem dados no Redis!",
                        MAX_NO_DATA_ITERATIONS
                    );
                    error!("📊 Último índice tentado: {}", self.data_index);
                    error!(
                        "📅 Última data processada: {}",
                        self.current_time.format("%Y-%m-%d %H:%M")
                    );

                    // Manter println para interface do usuário
                    println!(
                        "\n🛑 Simulação parada: {} iterações consecutivas sem dados no Redis!",
                        MAX_NO_DATA_ITERATIONS
                    );
                    println!("📊 Último índice tentado: {}", self.data_index);
                    println!(
                        "📅 Última data processada: {}",
                        self.current_time.format("%Y-%m-%d %H:%M")
                    );
                    break;
                }
            }

            // Avançar tempo (simulando 1 hora por tick)
            self.current_time = self.current_time + chrono::Duration::minutes(1);
            self.data_index += 1;

            // Pequena pausa para visualização
            thread::sleep(Duration::from_millis(10));

            // Verificar se deve parar por perda máxima
            // if self.should_stop_trading() {
            //     println!("\n🛑 Simulação parada: perda máxima atingida!");
            //     break;
            // }
        }

        info!("🏁 Simulação concluída!");
        info!(
            "⏱️  Tempo total de simulação: {:.2?}",
            start_simulation.elapsed()
        );

        // Manter println para interface do usuário
        println!("\n{}", "=".repeat(80));
        println!("🏁 Simulação concluída!");
        self.display_transaction_history();
        self.display_final_stats();
        println!(
            "⏱️  Tempo total de simulação: {:.2?}",
            start_simulation.elapsed()
        );

        Ok(())
    }

    fn get_current_btc_data(&self) -> Result<Option<CsvBtcFile>, Box<dyn std::error::Error>> {
        self.redis_client.load_by_index(self.data_index)
    }

    fn process_tick(&mut self, btc_data: &CsvBtcFile) -> Result<(), Box<dyn std::error::Error>> {
        let current_price = btc_data.close;

        // Atualizar preço pico recente para detectar quedas significativas
        if current_price > self.preco_pico_recente {
            self.preco_pico_recente = current_price;
        }

        // 1. Verificar condições de COMPRA por queda de preço
        if self.saldo_fiat > 0.0 {
            let mut should_buy = false;
            let limite_investimento = self.config.initial_balance * 0.9; // 90% do valor inicial

            // Se não tem BTC e nunca comprou, comprar na primeira oportunidade
            if self.saldo_btc == 0.0 && self.stats.total_trades == 0 {
                should_buy = true;
                info!("🎯 PRIMEIRA COMPRA detectada!");
                // Log já adicionado acima, manter println para interface
                println!("🎯 PRIMEIRA COMPRA detectada!");
            }
            // Se houve uma queda >= percentual_queda_para_comprar desde o pico recente
            else if self.preco_pico_recente > 0.0 {
                let queda_percentual =
                    ((self.preco_pico_recente - current_price) / self.preco_pico_recente) * 100.0;
                if queda_percentual >= self.config.percentual_queda_para_comprar {
                    let queda_dupla = self.config.percentual_queda_para_comprar * 2.0;

                    // Verificar se é uma queda de emergência (dobro do percentual)
                    if queda_percentual >= queda_dupla {
                        should_buy = true;
                        self.quedas_detectadas = 0; // Reset contador após compra de emergência
                        warn!(
                            "🚨 COMPRA DE EMERGÊNCIA! Queda -{:.2}% (>= -{:.1}% dobro do gatilho)",
                            queda_percentual, queda_dupla
                        );
                        warn!(
                            "⚡ EXECUTANDO COMPRA IMEDIATA do pico ${:.2} para ${:.2}",
                            self.preco_pico_recente, current_price
                        );
                        // Log já adicionado acima, manter println para interface
                        println!(
                            "🚨 COMPRA DE EMERGÊNCIA! Queda -{:.2}% (>= -{:.1}% dobro do gatilho)",
                            queda_percentual, queda_dupla
                        );
                        println!(
                            "⚡ EXECUTANDO COMPRA IMEDIATA do pico ${:.2} para ${:.2}",
                            self.preco_pico_recente, current_price
                        );
                    } else {
                        // Lógica normal: incrementar contador de quedas
                        self.quedas_detectadas += 1;

                        debug!(
                            "📉 QUEDA DETECTADA #{}: -{:.2}% do pico ${:.2} para ${:.2}",
                            self.quedas_detectadas,
                            queda_percentual,
                            self.preco_pico_recente,
                            current_price
                        );
                        // Log já adicionado acima, manter println para interface
                        println!(
                            "📉 QUEDA DETECTADA #{}: -{:.2}% do pico ${:.2} para ${:.2}",
                            self.quedas_detectadas,
                            queda_percentual,
                            self.preco_pico_recente,
                            current_price
                        );

                        // Comprar apenas se atingiu o número necessário de quedas
                        if self.quedas_detectadas >= self.quedas_para_comprar {
                            should_buy = true;
                            self.quedas_detectadas = 0; // Reset contador após compra
                            info!(
                                "✅ COMPRA LIBERADA: {} quedas atingidas!",
                                self.quedas_para_comprar
                            );
                            // Log já adicionado acima, manter println para interface
                            println!(
                                "✅ COMPRA LIBERADA: {} quedas atingidas!",
                                self.quedas_para_comprar
                            );
                        } else {
                            debug!(
                                "⏳ AGUARDANDO: {}/{} quedas para próxima compra (ou queda -{:.1}% para emergência)",
                                self.quedas_detectadas, self.quedas_para_comprar, queda_dupla
                            );
                            // Log já adicionado acima, manter println para interface
                            println!(
                                "⏳ AGUARDANDO: {}/{} quedas para próxima compra (ou queda -{:.1}% para emergência)",
                                self.quedas_detectadas, self.quedas_para_comprar, queda_dupla
                            );
                        }
                    }

                    // Reset do pico após detectar a queda
                    self.preco_pico_recente = current_price;
                }
            }

            // Verificar se não excederá 90% do valor inicial da carteira
            if should_buy {
                let valor_proxima_compra = self.saldo_fiat * (self.config.trade_percentage / 100.0);
                let total_apos_compra = self.total_investido + valor_proxima_compra;

                if total_apos_compra <= limite_investimento {
                    self.realizar_compra(current_price)?;
                } else {
                    warn!(
                        "🚫 COMPRA CANCELADA: Limite de 90% da carteira atingido (${:.2}/{:.2})",
                        total_apos_compra, limite_investimento
                    );
                    // Log já adicionado acima, manter println para interface
                    println!(
                        "🚫 COMPRA CANCELADA: Limite de 90% da carteira atingido (${:.2}/{:.2})",
                        total_apos_compra, limite_investimento
                    );
                }
            }
        }

        // 2. Verificar condições de VENDA (CADA ORDEM INDIVIDUALMENTE)
        self.verificar_vendas_individuais(current_price)?;

        // Atualizar preço anterior para próximo tick
        self.preco_anterior = Some(current_price);

        // Atualizar estatísticas
        self.update_portfolio_value(current_price);

        Ok(())
    }

    fn realizar_compra(&mut self, price: f64) -> Result<(), Box<dyn std::error::Error>> {
        // Calcular quantidade a comprar
        let quantidade_fiat_para_comprar = self.saldo_fiat * (self.config.trade_percentage / 100.0);
        let quantidade_btc_a_comprar = quantidade_fiat_para_comprar / price;

        // Criar nova ordem de compra
        let buy_order = BuyOrder {
            id: self.next_order_id,
            btc_quantity: quantidade_btc_a_comprar,
            buy_price: price,
            buy_time: self.current_time,
            invested_amount: quantidade_fiat_para_comprar,
        };

        // Criar transação de compra
        let transaction = Transaction {
            id: self.next_transaction_id,
            transaction_type: "BUY".to_string(),
            btc_quantity: quantidade_btc_a_comprar,
            price,
            time: self.current_time,
            amount: quantidade_fiat_para_comprar,
            profit_loss: None,
            buy_order_id: Some(self.next_order_id),
        };

        // Atualizar saldos
        self.saldo_fiat -= quantidade_fiat_para_comprar;
        self.saldo_btc += quantidade_btc_a_comprar;
        self.total_investido += quantidade_fiat_para_comprar;

        // Adicionar à lista de ordens e histórico
        self.buy_orders.push(buy_order);
        self.transaction_history.push(transaction);

        // Atualizar contadores
        self.next_order_id += 1;
        self.next_transaction_id += 1;
        self.stats.total_trades += 1;

        let tipo_compra = if self.buy_orders.len() == 1 {
            if self.stats.total_trades == 1 {
                "PRIMEIRA COMPRA"
            } else {
                "COMPRA DE EMERGÊNCIA"
            }
        } else {
            "COMPRA POR QUEDA"
        };

        info!(
            "🎯 {} REALIZADA - Ordem #{} - {:.6} BTC @ ${:.2} - Investido: ${:.2}",
            tipo_compra,
            self.next_order_id - 1,
            quantidade_btc_a_comprar,
            price,
            quantidade_fiat_para_comprar
        );

        // Log já adicionado acima, manter println para interface
        println!("\n{}", "=".repeat(80));
        println!(
            "🎯 {} REALIZADA - Ordem #{}",
            tipo_compra,
            self.next_order_id - 1
        );
        println!("{}", "-".repeat(80));
        println!("💰 Quantidade BTC: {:.6} BTC", quantidade_btc_a_comprar);
        println!("💵 Preço de compra: ${:.2}", price);
        println!("💸 Valor investido: ${:.2}", quantidade_fiat_para_comprar);
        println!("🏦 Saldo fiat restante: ${:.2}", self.saldo_fiat);
        println!("📊 Total BTC em carteira: {:.6} BTC", self.saldo_btc);
        println!("📋 Ordens ativas: {}", self.buy_orders.len());
        println!(
            "💸 Total investido: ${:.2} / ${:.2} (90% limite)",
            self.total_investido,
            self.config.initial_balance * 0.9
        );
        println!("{}", "=".repeat(80));

        Ok(())
    }

    fn verificar_vendas_individuais(
        &mut self,
        current_price: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut orders_to_sell = Vec::new();

        // Verificar cada ordem individualmente
        for (index, order) in self.buy_orders.iter().enumerate() {
            let ganho_percentual = ((current_price - order.buy_price) / order.buy_price) * 100.0;

            if ganho_percentual >= self.config.take_profit_percentage {
                orders_to_sell.push(index);
            }
        }

        // Vender ordens que atingiram o lucro (de trás para frente para não alterar índices)
        for &index in orders_to_sell.iter().rev() {
            self.vender_ordem_individual(index, current_price)?;
        }

        Ok(())
    }

    fn vender_ordem_individual(
        &mut self,
        order_index: usize,
        current_price: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let order = self.buy_orders.remove(order_index);
        let sale_amount = order.btc_quantity * current_price;
        let profit = sale_amount - order.invested_amount;
        let profit_percentage = (profit / order.invested_amount) * 100.0;

        // Criar transação de venda
        let transaction = Transaction {
            id: self.next_transaction_id,
            transaction_type: "SELL".to_string(),
            btc_quantity: order.btc_quantity,
            price: current_price,
            time: self.current_time,
            amount: sale_amount,
            profit_loss: Some(profit),
            buy_order_id: Some(order.id),
        };

        // Atualizar saldos
        self.saldo_fiat += sale_amount;
        self.saldo_btc -= order.btc_quantity;
        self.total_investido -= order.invested_amount;

        // Atualizar estatísticas
        self.stats.winning_trades += 1;
        self.stats.total_profit += profit;
        self.next_transaction_id += 1;

        // Adicionar ao histórico
        self.transaction_history.push(transaction);

        // Calcular tempo de holding
        let holding_duration = self.current_time.signed_duration_since(order.buy_time);
        let holding_days = holding_duration.num_days();
        let holding_hours = holding_duration.num_hours() % 24;

        info!(
            "💚 VENDA COM LUCRO - Ordem #{} - {:.6} BTC @ ${:.2} - Lucro: ${:.2} ({:.2}%) - Holding: {}d {}h",
            order.id,
            order.btc_quantity,
            current_price,
            profit,
            profit_percentage,
            holding_days,
            holding_hours
        );

        // Log já adicionado acima, manter println para interface
        println!("\n{}", "=".repeat(80));
        println!("💚 VENDA COM LUCRO - Ordem de Compra #{} VENDIDA", order.id);
        println!("{}", "-".repeat(80));
        println!(
            "📅 Comprada em: {} - Vendida em: {}",
            order.buy_time.format("%Y-%m-%d %H:%M"),
            self.current_time.format("%Y-%m-%d %H:%M")
        );
        println!(
            "⏱️  Tempo em carteira: {} dias e {} horas",
            holding_days, holding_hours
        );
        println!("💰 BTC vendido: {:.6} BTC", order.btc_quantity);
        println!(
            "💵 Preço COMPRA: ${:.2} → Preço VENDA: ${:.2}",
            order.buy_price, current_price
        );
        println!(
            "💸 Investimento: ${:.2} → Valor recebido: ${:.2}",
            order.invested_amount, sale_amount
        );
        println!("🎉 LUCRO: ${:.2} ({:.2}%)", profit, profit_percentage);
        println!("🏦 Saldo fiat atual: ${:.2}", self.saldo_fiat);
        println!("📋 Ordens restantes: {}", self.buy_orders.len());
        println!("💸 Total ainda investido: ${:.2}", self.total_investido);
        println!("{}", "=".repeat(80));

        Ok(())
    }

    fn update_portfolio_value(&mut self, current_price: f64) {
        // Atualizar o valor do portfolio incluindo BTC holdings
        let total_value = self.saldo_fiat + (self.saldo_btc * current_price);
        let drawdown =
            ((self.config.initial_balance - total_value) / self.config.initial_balance) * 100.0;

        self.stats.current_balance = self.saldo_fiat;
        self.stats.btc_balance = self.saldo_btc;
        self.stats.current_drawdown = drawdown.max(0.0);

        if self.stats.current_drawdown > self.stats.max_drawdown {
            self.stats.max_drawdown = self.stats.current_drawdown;
        }
    }

    fn should_stop_trading(&self) -> bool {
        // Verificar se deve parar baseado no drawdown atual
        self.stats.current_drawdown >= self.config.max_loss_percentage
    }

    fn display_status(&self, btc_data: &CsvBtcFile) {
        let btc_value = self.saldo_btc * btc_data.close;
        let total_value = self.saldo_fiat + btc_value;
        let profit_loss = total_value - self.config.initial_balance;
        let profit_loss_percent = (profit_loss / self.config.initial_balance) * 100.0;
        let progress = (self.data_index as f64 / self.total_records as f64) * 100.0;

        println!("\n┌{:─<78}┐", "");
        println!(
            "│ 📊 STATUS DA SIMULAÇÃO - {:<48} │",
            self.current_time.format("%Y-%m-%d %H:%M")
        );
        println!("├{:─<78}┤", "");
        println!(
            "│ 💵 Preço BTC atual: ${:<10.2} │ 🏦 Saldo Fiat: ${:<15.2} │",
            btc_data.close, self.saldo_fiat
        );
        println!(
            "│ 💰 BTC em carteira: {:<8.6} BTC │ 💎 Valor BTC: ${:<15.2} │",
            self.saldo_btc, btc_value
        );
        println!(
            "│ 💰 Valor total: ${:<13.2} │ 📈 P&L: ${:<8.2} ({:<+5.1}%) │",
            total_value, profit_loss, profit_loss_percent
        );
        println!(
            "│ 🏆 Trades vencedores: {:<8} │ 😞 Trades perdedores: {:<8} │",
            self.stats.winning_trades, self.stats.losing_trades
        );
        println!(
            "│ ⏳ Progresso: {:<5.1}%               │ 📊 Total trades: {:<12} │",
            progress, self.stats.total_trades
        );
        println!(
            "│ 💸 Total investido: ${:<10.2} │ 🎯 Limite 90%: ${:<15.2} │",
            self.total_investido,
            self.config.initial_balance * 0.9
        );

        if !self.buy_orders.is_empty() {
            let queda_do_pico = if self.preco_pico_recente > 0.0 {
                ((self.preco_pico_recente - btc_data.close) / self.preco_pico_recente) * 100.0
            } else {
                0.0
            };

            println!("├{:─<78}┤", "");
            println!(
                "│ 🎯 ORDENS ATIVAS ({:<2})                                               │",
                self.buy_orders.len()
            );

            for (_i, order) in self.buy_orders.iter().take(3).enumerate() {
                let unrealized_pnl = (btc_data.close - order.buy_price) * order.btc_quantity;
                let unrealized_percent =
                    ((btc_data.close - order.buy_price) / order.buy_price) * 100.0;
                println!(
                    "│ #{:<2} {:.4} BTC @ ${:<8.2} │ P&L: ${:<6.2} ({:<+5.1}%) │",
                    order.id,
                    order.btc_quantity,
                    order.buy_price,
                    unrealized_pnl,
                    unrealized_percent
                );
            }

            if self.buy_orders.len() > 3 {
                println!(
                    "│ ... e mais {} ordens                                             │",
                    self.buy_orders.len() - 3
                );
            }

            println!(
                "│ 📊 Pico recente: ${:<11.2} │ 📉 Queda do pico: -{:<6.2}%        │",
                self.preco_pico_recente, queda_do_pico
            );
            println!(
                "│ 🎯 Gatilho compra: -{:<6.1}%        │ 🎯 Take profit: +{:<6.1}%        │",
                self.config.percentual_queda_para_comprar, self.config.take_profit_percentage
            );
            println!(
                "│ 🚨 Emergência: -{:<6.1}%           │ 📊 Quedas detectadas: {}/{:<8}    │",
                self.config.percentual_queda_para_comprar * 2.0,
                self.quedas_detectadas,
                self.quedas_para_comprar
            );
            println!(
                "│ 🎯 Próxima compra em: {:<2} quedas     │ ⚡ Ou queda -{:.1}% (emergência)     │",
                self.quedas_para_comprar - self.quedas_detectadas,
                self.config.percentual_queda_para_comprar * 2.0
            );
        }

        println!("└{:─<78}┘", "");
    }

    fn display_transaction_history(&self) {
        if self.transaction_history.is_empty() {
            info!("📊 Nenhuma transação foi realizada durante a simulação");
            return;
        }

        info!(
            "📊 Exibindo histórico completo de {} transações",
            self.transaction_history.len()
        );

        println!("\n");
        println!("╔{:═<98}╗", "");
        println!("║{:^98}║", "📊 HISTÓRICO COMPLETO DE TRANSAÇÕES 📊");
        println!("╠{:═<98}╣", "");

        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_profit = 0.0;

        for transaction in &self.transaction_history {
            match transaction.transaction_type.as_str() {
                "BUY" => {
                    buy_count += 1;
                    info!(
                        "🟢 COMPRA #{} - {:.6} BTC @ ${:.2} em {} - Valor: ${:.2}",
                        transaction.id,
                        transaction.btc_quantity,
                        transaction.price,
                        transaction.time.format("%Y-%m-%d %H:%M"),
                        transaction.amount
                    );
                    println!(
                        "║ 🟢 COMPRA #{:<3} │ {:.6} BTC @ ${:<10.2} │ {} │ ${:<12.2} ║",
                        transaction.id,
                        transaction.btc_quantity,
                        transaction.price,
                        transaction.time.format("%Y-%m-%d %H:%M"),
                        transaction.amount
                    );
                }
                "SELL" => {
                    sell_count += 1;
                    let profit = transaction.profit_loss.unwrap_or(0.0);
                    let profit_percent = if let Some(buy_order_id) = transaction.buy_order_id {
                        // Encontrar a transação de compra correspondente
                        if let Some(buy_tx) = self.transaction_history.iter().find(|tx| {
                            tx.transaction_type == "BUY" && tx.buy_order_id == Some(buy_order_id)
                        }) {
                            (profit / buy_tx.amount) * 100.0
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    total_profit += profit;

                    info!(
                        "🔴 VENDA #{} - {:.6} BTC @ ${:.2} em {} - Lucro: ${:.2} (+{:.1}%)",
                        transaction.id,
                        transaction.btc_quantity,
                        transaction.price,
                        transaction.time.format("%Y-%m-%d %H:%M"),
                        profit,
                        profit_percent
                    );
                    println!(
                        "║ 🔴 VENDA  #{:<3} │ {:.6} BTC @ ${:<10.2} │ {} │ ${:<6.2} (+{:<4.1}%) ║",
                        transaction.id,
                        transaction.btc_quantity,
                        transaction.price,
                        transaction.time.format("%Y-%m-%d %H:%M"),
                        profit,
                        profit_percent
                    );
                }
                _ => {}
            }
        }

        info!(
            "📊 RESUMO TRANSAÇÕES: {} compras, {} vendas, lucro total: ${:.2}",
            buy_count, sell_count, total_profit
        );

        println!("╠{:─<98}╣", "");
        println!(
            "║ 📊 RESUMO: {} compras, {} vendas │ Lucro total das vendas: ${:<12.2} ║",
            buy_count, sell_count, total_profit
        );

        // Mostrar ordens ainda abertas
        if !self.buy_orders.is_empty() {
            info!(
                "🔄 {} ordens ainda abertas (não vendidas)",
                self.buy_orders.len()
            );
            for order in &self.buy_orders {
                info!(
                    "📋 Ordem #{} aberta - {:.6} BTC @ ${:.2} em {} - Investido: ${:.2}",
                    order.id,
                    order.btc_quantity,
                    order.buy_price,
                    order.buy_time.format("%Y-%m-%d %H:%M"),
                    order.invested_amount
                );
            }

            println!("╠{:─<98}╣", "");
            println!(
                "║ 🔄 ORDENS AINDA ABERTAS ({})                                                        ║",
                self.buy_orders.len()
            );
            for order in &self.buy_orders {
                println!(
                    "║ 📋 Ordem #{:<3} │ {:.6} BTC @ ${:<10.2} │ {} │ Investido: ${:<8.2} ║",
                    order.id,
                    order.btc_quantity,
                    order.buy_price,
                    order.buy_time.format("%Y-%m-%d %H:%M"),
                    order.invested_amount
                );
            }
        }

        println!("╚{:═<98}╝", "");
    }

    fn display_final_stats(&self) {
        // Calcular valor total incluindo BTC restante se houver
        let total_value = self.saldo_fiat + (self.saldo_btc * 50000.0); // Assumir preço final
        let net_return =
            ((total_value - self.config.initial_balance) / self.config.initial_balance) * 100.0;
        let profit_total = self.stats.net_profit();

        // Log estruturado dos resultados finais
        info!(
            "🏁 RESULTADO FINAL: Saldo inicial ${:.2} → Final ${:.2} | Retorno: {:.2}% | Lucro: ${:.2}",
            self.config.initial_balance, total_value, net_return, profit_total
        );

        info!(
            "📊 ESTATÍSTICAS: {} trades | {} vencedores ({:.1}%) | {} perdedores | Drawdown máx: {:.2}%",
            self.stats.total_trades,
            self.stats.winning_trades,
            self.stats.win_rate(),
            self.stats.losing_trades,
            self.stats.max_drawdown
        );

        info!(
            "💰 BALANÇO: Saldo fiat ${:.2} | BTC restante {:.6} | Valor BTC ${:.2}",
            self.saldo_fiat,
            self.saldo_btc,
            self.saldo_btc * 50000.0
        );

        if net_return >= 0.0 {
            info!("🎉 RESULTADO: LUCRO - Estratégia foi lucrativa!");
        } else {
            warn!("💔 RESULTADO: PREJUÍZO - Estratégia teve perda");
        }

        println!("\n");
        println!("╔{:═<78}╗", "");
        println!("║{:^78}║", "🏁 RELATÓRIO FINAL DE TRADING 🏁");
        println!("╠{:═<78}╣", "");

        // Resultado geral
        if net_return >= 0.0 {
            println!("║ 🎉 RESULTADO: LUCRO                                                 ║");
        } else {
            println!("║ 💔 RESULTADO: PREJUÍZO                                              ║");
        }

        println!("╠{:─<78}╣", "");
        println!(
            "║ 💰 SALDO INICIAL:       ${:<15.2} │ 💰 SALDO FINAL: ${:<15.2} ║",
            self.config.initial_balance, total_value
        );
        println!(
            "║ 📊 RETORNO LÍQUIDO:     ${:<8.2} ({:<+6.2}%) │ 🏦 Saldo Fiat: ${:<15.2} ║",
            profit_total, net_return, self.saldo_fiat
        );

        if self.saldo_btc > 0.0 {
            println!(
                "║ 💎 BTC restante:        {:<8.6} BTC │ 💎 Valor BTC: ${:<16.2} ║",
                self.saldo_btc,
                self.saldo_btc * 50000.0
            );
        }

        println!("╠{:─<78}╣", "");
        println!(
            "║ 📈 TOTAL DE LUCROS:     ${:<15.2} │ 📉 TOTAL DE PERDAS: ${:<12.2} ║",
            self.stats.total_profit, self.stats.total_loss
        );
        println!(
            "║ 🎯 TRADES REALIZADOS:   {:<15} │ 📉 DRAWDOWN MÁXIMO: {:<8.2}% ║",
            self.stats.total_trades, self.stats.max_drawdown
        );
        println!(
            "║ ✅ TRADES VENCEDORES:   {:<8} ({:<5.1}%) │ ❌ TRADES PERDEDORES: {:<8} ║",
            self.stats.winning_trades,
            self.stats.win_rate(),
            self.stats.losing_trades
        );

        if self.stats.total_trades > 0 {
            println!("╠{:─<78}╣", "");
            println!(
                "║ 💰 LUCRO MÉDIO/TRADE VENCEDOR: ${:<8.2} │ 💸 PERDA MÉDIA/PERDEDOR: ${:<8.2} ║",
                self.stats.total_profit / self.stats.winning_trades.max(1) as f64,
                self.stats.total_loss / self.stats.losing_trades.max(1) as f64
            );
        }

        println!("╠{:─<78}╣", "");
        println!("║ 📊 CONFIGURAÇÃO USADA:                                              ║");
        println!(
            "║ • Percentual por trade: {:<5.1}%    • Take Profit: {:<5.1}%             ║",
            self.config.trade_percentage, self.config.take_profit_percentage
        );
        println!(
            "║ • Gatilho compra: -{:<5.1}%          • Limite investimento: 90%      ║",
            self.config.percentual_queda_para_comprar
        );

        info!(
            "⚙️ CONFIGURAÇÃO: Trade {}% | Take Profit {}% | Gatilho compra -{}%",
            self.config.trade_percentage,
            self.config.take_profit_percentage,
            self.config.percentual_queda_para_comprar
        );

        println!("╚{:═<78}╝", "");

        // Resumo final colorido e logs
        if net_return >= 10.0 {
            info!("🎉 AVALIAÇÃO FINAL: EXCELENTE RESULTADO! Retorno acima de 10%");
            println!("🎉🎉🎉 PARABÉNS! EXCELENTE RESULTADO! 🎉🎉🎉");
        } else if net_return >= 0.0 {
            info!("😊 AVALIAÇÃO FINAL: BOM RESULTADO! Estratégia lucrativa");
            println!("😊 BOM RESULTADO! ESTRATÉGIA LUCRATIVA! 😊");
        } else if net_return >= -10.0 {
            warn!("😐 AVALIAÇÃO FINAL: RESULTADO NEUTRO. Considere ajustar a estratégia");
            println!("😐 RESULTADO NEUTRO. CONSIDERE AJUSTAR A ESTRATÉGIA.");
        } else {
            error!("😞 AVALIAÇÃO FINAL: RESULTADO NEGATIVO. Revise a estratégia");
            println!("😞 RESULTADO NEGATIVO. REVISE A ESTRATÉGIA.");
        }
    }
}

// Função para executar o simulador
pub fn run_trade_simulation() -> Result<(), Box<dyn std::error::Error>> {
    info!("🚀 Iniciando simulação de trading BTC");

    let redis_client = RedisClient::from_env()?;

    // Configuração personalizada do trade DCA
    let config = TradeConfig {
        initial_balance: 100.0,
        max_loss_percentage: 50.0,
        trade_percentage: 5.0,              // 10% do saldo por compra
        stop_loss_percentage: 0.0,          // NÃO usado - sem stop loss
        take_profit_percentage: 6.0,        // Vender APENAS com 15% de lucro
        percentual_queda_para_comprar: 3.0, // Comprar quando cair 5% do pico
        preco_inicial_de_compra: None,      // Começar na primeira oportunidade
    };

    info!(
        "📊 Configuração carregada: saldo inicial ${}, take profit {}%, gatilho compra {}%",
        config.initial_balance, config.take_profit_percentage, config.percentual_queda_para_comprar
    );

    let mut simulator = TradeSimulator::new(redis_client, config)?;
    let result = simulator.run();

    match &result {
        Ok(_) => info!("✅ Simulação concluída com sucesso"),
        Err(e) => error!("❌ Simulação falhou: {}", e),
    }

    result
}
