use crate::reader_csv::CsvBtcFile;
use crate::llm_client::{LlmClient, LlmTradeAnalysis};
use crate::decision_engine::TradeAction;
use chrono::{DateTime, Utc};
use anyhow::Result;

/// Contexto de mercado para análise
#[derive(Debug, Clone)]
pub struct MarketContext {
    pub current_price: f64,
    pub previous_prices: Vec<f64>,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub price_change_24h: f64,
    pub price_change_percentage: f64,
    pub recent_high: f64,
    pub recent_low: f64,
    pub volatility: f64,
}

impl MarketContext {
    /// Cria contexto de mercado a partir de dados históricos
    pub fn from_btc_data(current: &CsvBtcFile, historical: &[CsvBtcFile]) -> Self {
        let current_price = current.close;
        let previous_prices: Vec<f64> = historical.iter()
            .rev()
            .take(20) // Últimos 20 períodos
            .map(|data| data.close)
            .collect();

        let price_24h_ago = historical.last()
            .map(|data| data.close)
            .unwrap_or(current_price);

        let price_change_24h = current_price - price_24h_ago;
        let price_change_percentage = (price_change_24h / price_24h_ago) * 100.0;

        let recent_high = historical.iter()
            .chain(std::iter::once(current))
            .map(|data| data.high)
            .fold(0.0, f64::max);

        let recent_low = historical.iter()
            .chain(std::iter::once(current))
            .map(|data| data.low)
            .fold(f64::INFINITY, f64::min);

        // Calcular volatilidade simples (desvio padrão dos preços recentes)
        let avg_price = previous_prices.iter().sum::<f64>() / previous_prices.len() as f64;
        let variance = previous_prices.iter()
            .map(|price| (price - avg_price).powi(2))
            .sum::<f64>() / previous_prices.len() as f64;
        let volatility = variance.sqrt();

        let timestamp = DateTime::parse_from_rfc3339(&current.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        Self {
            current_price,
            previous_prices,
            volume: current.volume,
            timestamp,
            price_change_24h,
            price_change_percentage,
            recent_high,
            recent_low,
            volatility,
        }
    }

    /// Converte contexto de mercado em string formatada para o LLM
    pub fn to_llm_prompt(&self) -> String {
        let trend = if self.price_change_percentage > 2.0 {
            "FORTE ALTA"
        } else if self.price_change_percentage > 0.5 {
            "ALTA"
        } else if self.price_change_percentage < -2.0 {
            "FORTE QUEDA"
        } else if self.price_change_percentage < -0.5 {
            "QUEDA"
        } else {
            "LATERAL"
        };

        let volatility_level = if self.volatility > 2000.0 {
            "MUITO ALTA"
        } else if self.volatility > 1000.0 {
            "ALTA"
        } else if self.volatility > 500.0 {
            "MÉDIA"
        } else {
            "BAIXA"
        };

        format!(
            r#"ANÁLISE DE MERCADO DO BITCOIN:

📊 PREÇO ATUAL: ${:.2}
📈 VARIAÇÃO 24H: ${:.2} ({:.2}%)
🔄 TENDÊNCIA: {}

📊 ESTATÍSTICAS:
• Máxima recente: ${:.2}
• Mínima recente: ${:.2}
• Volume: {:.2}
• Volatilidade: {:.2} ({})

📈 HISTÓRICO DE PREÇOS (últimos 20 períodos):
{}

🎯 INDICADORES TÉCNICOS:
• Distância da máxima: {:.2}%
• Distância da mínima: {:.2}%
• Posição no range: {:.1}%

⏰ TIMESTAMP: {}

Por favor, analise estes dados e forneça uma recomendação de trading considerando:
1. A tendência atual do preço
2. O nível de volatilidade
3. A posição no range de preços recentes
4. O volume de negociação
5. Padrões identificados no histórico de preços"#,
            self.current_price,
            self.price_change_24h,
            self.price_change_percentage,
            trend,
            self.recent_high,
            self.recent_low,
            self.volume,
            self.volatility,
            volatility_level,
            self.format_price_history(),
            ((self.recent_high - self.current_price) / self.recent_high) * 100.0,
            ((self.current_price - self.recent_low) / self.recent_low) * 100.0,
            ((self.current_price - self.recent_low) / (self.recent_high - self.recent_low)) * 100.0,
            self.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    fn format_price_history(&self) -> String {
        self.previous_prices
            .iter()
            .enumerate()
            .map(|(i, price)| format!("{}. ${:.2}", i + 1, price))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Indicadores técnicos calculados
#[derive(Debug, Clone)]
pub struct TechnicalIndicators {
    pub sma_20: f64,      // Média móvel simples 20 períodos
    pub rsi: f64,         // Índice de força relativa
    pub bollinger_upper: f64,
    pub bollinger_lower: f64,
    pub macd: f64,
    pub support_level: f64,
    pub resistance_level: f64,
}

impl TechnicalIndicators {
    /// Calcula indicadores técnicos a partir de dados históricos
    pub fn calculate(data: &[CsvBtcFile]) -> Self {
        let prices: Vec<f64> = data.iter().map(|d| d.close).collect();
        
        // SMA 20
        let sma_20 = if prices.len() >= 20 {
            prices.iter().rev().take(20).sum::<f64>() / 20.0
        } else {
            prices.iter().sum::<f64>() / prices.len() as f64
        };

        // RSI simplificado
        let rsi = Self::calculate_rsi(&prices);

        // Bollinger Bands
        let (bollinger_upper, bollinger_lower) = Self::calculate_bollinger_bands(&prices, sma_20);

        // MACD simplificado
        let macd = Self::calculate_macd(&prices);

        // Suporte e resistência
        let (support_level, resistance_level) = Self::calculate_support_resistance(&prices);

        Self {
            sma_20,
            rsi,
            bollinger_upper,
            bollinger_lower,
            macd,
            support_level,
            resistance_level,
        }
    }

    fn calculate_rsi(prices: &[f64]) -> f64 {
        if prices.len() < 14 {
            return 50.0; // RSI neutro se não há dados suficientes
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=14.min(prices.len() - 1) {
            let change = prices[prices.len() - i] - prices[prices.len() - i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / 14.0;
        let avg_loss = losses / 14.0;

        if avg_loss == 0.0 {
            return 100.0;
        }

        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }

    fn calculate_bollinger_bands(prices: &[f64], sma: f64) -> (f64, f64) {
        if prices.len() < 20 {
            return (sma * 1.02, sma * 0.98); // ±2% se não há dados suficientes
        }

        let recent_prices: Vec<f64> = prices.iter().rev().take(20).cloned().collect();
        let variance = recent_prices.iter()
            .map(|price| (price - sma).powi(2))
            .sum::<f64>() / 20.0;
        let std_dev = variance.sqrt();

        (sma + (2.0 * std_dev), sma - (2.0 * std_dev))
    }

    fn calculate_macd(prices: &[f64]) -> f64 {
        if prices.len() < 26 {
            return 0.0;
        }

        let ema_12 = Self::calculate_ema(prices, 12);
        let ema_26 = Self::calculate_ema(prices, 26);
        ema_12 - ema_26
    }

    fn calculate_ema(prices: &[f64], period: usize) -> f64 {
        if prices.is_empty() {
            return 0.0;
        }

        let multiplier = 2.0 / (period as f64 + 1.0);
        let mut ema = prices[0];

        for &price in prices.iter().skip(1) {
            ema = (price * multiplier) + (ema * (1.0 - multiplier));
        }

        ema
    }

    fn calculate_support_resistance(prices: &[f64]) -> (f64, f64) {
        if prices.is_empty() {
            return (0.0, 0.0);
        }

        let recent_prices: Vec<f64> = prices.iter().rev().take(50).cloned().collect();
        let min_price = recent_prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_price = recent_prices.iter().cloned().fold(0.0, f64::max);

        (min_price, max_price)
    }

    /// Converte indicadores para string formatada para o LLM
    pub fn to_llm_string(&self) -> String {
        let rsi_signal = if self.rsi > 70.0 {
            "SOBRECOMPRADO"
        } else if self.rsi < 30.0 {
            "SOBREVENDIDO"
        } else {
            "NEUTRO"
        };

        let macd_signal = if self.macd > 0.0 {
            "BULLISH"
        } else {
            "BEARISH"
        };

        format!(
            r#"🔍 INDICADORES TÉCNICOS:
• SMA 20: ${:.2}
• RSI: {:.1} ({})
• Bollinger Superior: ${:.2}
• Bollinger Inferior: ${:.2}
• MACD: {:.2} ({})
• Suporte: ${:.2}
• Resistência: ${:.2}"#,
            self.sma_20,
            self.rsi,
            rsi_signal,
            self.bollinger_upper,
            self.bollinger_lower,
            self.macd,
            macd_signal,
            self.support_level,
            self.resistance_level
        )
    }
}

/// Analisador de mercado que combina dados técnicos com análise do LLM
pub struct MarketAnalyzer {
    llm_client: LlmClient,
}

impl MarketAnalyzer {
    pub fn new(llm_client: LlmClient) -> Self {
        Self { llm_client }
    }

    /// Análise completa de mercado
    pub async fn analyze_comprehensive(
        &self,
        current: &CsvBtcFile,
        historical: &[CsvBtcFile],
    ) -> Result<LlmTradeAnalysis> {
        // Criar contexto de mercado
        let market_context = MarketContext::from_btc_data(current, historical);
        
        // Calcular indicadores técnicos
        let technical_indicators = TechnicalIndicators::calculate(historical);

        // Combinar informações para prompt do LLM
        let comprehensive_prompt = format!(
            "{}\n\n{}\n\n💡 OBJETIVO: Com base nesta análise completa, forneça uma recomendação de trading precisa.",
            market_context.to_llm_prompt(),
            technical_indicators.to_llm_string()
        );

        // Obter análise do LLM
        self.llm_client.analyze_market_data(&comprehensive_prompt).await
    }

    /// Análise rápida apenas com contexto básico
    pub async fn analyze_quick(&self, current: &CsvBtcFile) -> Result<LlmTradeAnalysis> {
        let basic_context = format!(
            r#"ANÁLISE RÁPIDA DO BITCOIN:
📊 Preço atual: ${:.2}
📊 Volume: {:.2}
📊 Máxima: ${:.2}  
📊 Mínima: ${:.2}
⏰ Timestamp: {}

Forneça uma recomendação rápida de trading."#,
            current.close,
            current.volume,
            current.high,
            current.low,
            current.timestamp
        );

        self.llm_client.analyze_market_data(&basic_context).await
    }

    /// Verifica se o LLM está disponível
    pub async fn is_llm_available(&self) -> bool {
        self.llm_client.is_available().await
    }

    /// Análise com fallback (retorna análise técnica básica se LLM não estiver disponível)
    pub async fn analyze_with_fallback(
        &self,
        current: &CsvBtcFile,
        historical: &[CsvBtcFile],
    ) -> LlmTradeAnalysis {
        match self.analyze_comprehensive(current, historical).await {
            Ok(analysis) => analysis,
            Err(_) => {
                // Fallback para análise técnica básica
                self.fallback_technical_analysis(current, historical)
            }
        }
    }

    /// Análise técnica básica como fallback quando LLM não está disponível
    fn fallback_technical_analysis(&self, current: &CsvBtcFile, historical: &[CsvBtcFile]) -> LlmTradeAnalysis {
        let indicators = TechnicalIndicators::calculate(historical);
        let market_context = MarketContext::from_btc_data(current, historical);

        // Lógica básica de decisão baseada em indicadores
        let action = if indicators.rsi < 30.0 && current.close < indicators.bollinger_lower {
            TradeAction::Buy
        } else if indicators.rsi > 70.0 && current.close > indicators.bollinger_upper {
            TradeAction::Sell
        } else if market_context.price_change_percentage < -5.0 {
            TradeAction::Buy
        } else if market_context.price_change_percentage > 5.0 {
            TradeAction::Sell
        } else {
            TradeAction::Hold
        };

        let confidence = match action {
            TradeAction::Buy | TradeAction::Sell => 0.6,
            _ => 0.4,
        };

        LlmTradeAnalysis {
            action,
            confidence,
            reasoning: "Análise técnica básica (LLM indisponível)".to_string(),
            risk_level: crate::decision_engine::RiskLevel::Medium,
            price_prediction: None,
        }
    }
}