use crate::llm_client::LlmTradeAnalysis;
use crate::market_analysis::MarketAnalyzer;
use crate::reader_csv::CsvBtcFile;
use chrono::{DateTime, Utc};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
    StrongBuy,
    StrongSell,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Configuração do motor de decisão
#[derive(Debug, Clone)]
pub struct DecisionConfig {
    pub llm_weight: f32,           // Peso da decisão do LLM (0.0 a 1.0)
    pub technical_weight: f32,     // Peso da análise técnica (0.0 a 1.0)
    pub min_confidence: f32,       // Confiança mínima para executar trade
    pub risk_tolerance: RiskLevel, // Tolerância a risco
    pub use_llm: bool,            // Se deve usar LLM ou não
    pub fallback_to_technical: bool, // Se deve usar fallback técnico quando LLM falha
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            llm_weight: 0.7,
            technical_weight: 0.3,
            min_confidence: 0.6,
            risk_tolerance: RiskLevel::Medium,
            use_llm: true,
            fallback_to_technical: true,
        }
    }
}

/// Resultado de uma decisão de trading
#[derive(Debug, Clone)]
pub struct TradeDecision {
    pub action: TradeAction,
    pub confidence: f32,
    pub reasoning: String,
    pub risk_assessment: RiskLevel,
    pub should_execute: bool,
    pub suggested_amount: Option<f32>, // Percentual do saldo a usar (0.0 a 1.0)
    pub llm_analysis: Option<LlmTradeAnalysis>,
    pub technical_signals: TechnicalSignals,
    pub timestamp: DateTime<Utc>,
}

/// Sinais técnicos básicos
#[derive(Debug, Clone)]
pub struct TechnicalSignals {
    pub price_trend: TrendSignal,
    pub volume_signal: VolumeSignal,
    pub volatility_signal: VolatilitySignal,
    pub momentum_signal: MomentumSignal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrendSignal {
    StrongBullish,
    Bullish,
    Neutral,
    Bearish,
    StrongBearish,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VolumeSignal {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VolatilitySignal {
    VeryHigh,
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MomentumSignal {
    StrongPositive,
    Positive,
    Neutral,
    Negative,
    StrongNegative,
}

impl TechnicalSignals {
    /// Calcula sinais técnicos a partir dos dados
    pub fn calculate(current: &CsvBtcFile, historical: &[CsvBtcFile]) -> Self {
        let price_trend = Self::calculate_price_trend(current, historical);
        let volume_signal = Self::calculate_volume_signal(current, historical);
        let volatility_signal = Self::calculate_volatility_signal(current, historical);
        let momentum_signal = Self::calculate_momentum_signal(current, historical);

        Self {
            price_trend,
            volume_signal,
            volatility_signal,
            momentum_signal,
        }
    }

    fn calculate_price_trend(_current: &CsvBtcFile, historical: &[CsvBtcFile]) -> TrendSignal {
        if historical.len() < 5 {
            return TrendSignal::Neutral;
        }

        let recent_prices: Vec<f64> = historical
            .iter()
            .rev()
            .take(10)
            .map(|d| d.close)
            .collect();

        let avg_old = recent_prices.iter().skip(5).sum::<f64>() / 5.0;
        let avg_new = recent_prices.iter().take(5).sum::<f64>() / 5.0;
        
        let change_percent = ((avg_new - avg_old) / avg_old) * 100.0;

        match change_percent {
            x if x > 3.0 => TrendSignal::StrongBullish,
            x if x > 1.0 => TrendSignal::Bullish,
            x if x < -3.0 => TrendSignal::StrongBearish,
            x if x < -1.0 => TrendSignal::Bearish,
            _ => TrendSignal::Neutral,
        }
    }

    fn calculate_volume_signal(current: &CsvBtcFile, historical: &[CsvBtcFile]) -> VolumeSignal {
        if historical.is_empty() {
            return VolumeSignal::Normal;
        }

        let avg_volume: f64 = historical
            .iter()
            .rev()
            .take(20)
            .map(|d| d.volume)
            .sum::<f64>() / (20.0_f64).min(historical.len() as f64);

        let volume_ratio = current.volume / avg_volume;

        match volume_ratio {
            x if x > 1.5 => VolumeSignal::High,
            x if x < 0.7 => VolumeSignal::Low,
            _ => VolumeSignal::Normal,
        }
    }

    fn calculate_volatility_signal(current: &CsvBtcFile, historical: &[CsvBtcFile]) -> VolatilitySignal {
        if historical.len() < 10 {
            return VolatilitySignal::Normal;
        }

        let recent_highs_lows: Vec<f64> = historical
            .iter()
            .rev()
            .take(10)
            .map(|d| ((d.high - d.low) / d.close) * 100.0)
            .collect();

        let avg_volatility = recent_highs_lows.iter().sum::<f64>() / recent_highs_lows.len() as f64;
        let current_volatility = ((current.high - current.low) / current.close) * 100.0;

        match current_volatility {
            x if x > avg_volatility * 2.0 => VolatilitySignal::VeryHigh,
            x if x > avg_volatility * 1.5 => VolatilitySignal::High,
            x if x < avg_volatility * 0.5 => VolatilitySignal::Low,
            _ => VolatilitySignal::Normal,
        }
    }

    fn calculate_momentum_signal(current: &CsvBtcFile, historical: &[CsvBtcFile]) -> MomentumSignal {
        if historical.len() < 3 {
            return MomentumSignal::Neutral;
        }

        // Calcular momentum baseado nas últimas 3 velas
        let recent_closes: Vec<f64> = historical
            .iter()
            .rev()
            .take(3)
            .map(|d| d.close)
            .chain(std::iter::once(current.close))
            .collect();

        let momentum = recent_closes.windows(2)
            .map(|window| if window[1] > window[0] { 1.0 } else { -1.0 })
            .sum::<f64>();

        match momentum {
            x if x >= 2.0 => MomentumSignal::StrongPositive,
            x if x >= 1.0 => MomentumSignal::Positive,
            x if x <= -2.0 => MomentumSignal::StrongNegative,
            x if x <= -1.0 => MomentumSignal::Negative,
            _ => MomentumSignal::Neutral,
        }
    }

    /// Converte sinais técnicos em ação recomendada
    pub fn to_trade_action(&self) -> TradeAction {
        let mut buy_score = 0;
        let mut sell_score = 0;

        // Avaliar tendência de preço
        match self.price_trend {
            TrendSignal::StrongBullish => buy_score += 3,
            TrendSignal::Bullish => buy_score += 1,
            TrendSignal::StrongBearish => sell_score += 3,
            TrendSignal::Bearish => sell_score += 1,
            TrendSignal::Neutral => {},
        }

        // Avaliar momentum
        match self.momentum_signal {
            MomentumSignal::StrongPositive => buy_score += 2,
            MomentumSignal::Positive => buy_score += 1,
            MomentumSignal::StrongNegative => sell_score += 2,
            MomentumSignal::Negative => sell_score += 1,
            MomentumSignal::Neutral => {},
        }

        // Avaliar volume (confirma sinais)
        match self.volume_signal {
            VolumeSignal::High => {
                if buy_score > sell_score {
                    buy_score += 1;
                } else if sell_score > buy_score {
                    sell_score += 1;
                }
            },
            _ => {},
        }

        // Determinar ação final
        match buy_score - sell_score {
            x if x >= 3 => TradeAction::StrongBuy,
            x if x >= 1 => TradeAction::Buy,
            x if x <= -3 => TradeAction::StrongSell,
            x if x <= -1 => TradeAction::Sell,
            _ => TradeAction::Hold,
        }
    }

    /// Calcula confiança dos sinais técnicos
    pub fn confidence(&self) -> f32 {
        let mut confidence: f32 = 0.5; // Base

        // Ajustar confiança baseado na força dos sinais
        match self.price_trend {
            TrendSignal::StrongBullish | TrendSignal::StrongBearish => confidence += 0.2,
            TrendSignal::Bullish | TrendSignal::Bearish => confidence += 0.1,
            _ => {},
        }

        match self.momentum_signal {
            MomentumSignal::StrongPositive | MomentumSignal::StrongNegative => confidence += 0.15,
            MomentumSignal::Positive | MomentumSignal::Negative => confidence += 0.1,
            _ => {},
        }

        if self.volume_signal == VolumeSignal::High {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }
}

/// Motor de decisão que combina LLM e análise técnica
pub struct DecisionEngine {
    config: DecisionConfig,
    market_analyzer: MarketAnalyzer,
}

impl DecisionEngine {
    pub fn new(config: DecisionConfig, market_analyzer: MarketAnalyzer) -> Self {
        Self {
            config,
            market_analyzer,
        }
    }

    /// Toma decisão de trading baseada em todos os fatores disponíveis
    pub async fn make_decision(
        &self,
        current: &CsvBtcFile,
        historical: &[CsvBtcFile],
        current_balance: f64,
        current_btc: f64,
    ) -> Result<TradeDecision> {
        // Calcular sinais técnicos
        let technical_signals = TechnicalSignals::calculate(current, historical);
        let technical_action = technical_signals.to_trade_action();
        let technical_confidence = technical_signals.confidence();

        // Tentar obter análise do LLM se habilitado
        let llm_analysis = if self.config.use_llm {
            match self.market_analyzer.analyze_with_fallback(current, historical).await {
                analysis => Some(analysis),
            }
        } else {
            None
        };

        // Combinar análises
        let (final_action, final_confidence, reasoning) = self.combine_analyses(
            &technical_action,
            technical_confidence,
            &llm_analysis,
        );

        // Avaliar risco
        let risk_assessment = self.assess_risk(&technical_signals, &llm_analysis);

        // Decidir se deve executar
        let should_execute = self.should_execute_trade(
            final_confidence,
            &risk_assessment,
            &final_action,
            current_balance,
            current_btc,
        );

        // Calcular quantidade sugerida
        let suggested_amount = if should_execute {
            Some(self.calculate_trade_amount(&final_action, final_confidence, &risk_assessment))
        } else {
            None
        };

        Ok(TradeDecision {
            action: final_action,
            confidence: final_confidence,
            reasoning,
            risk_assessment,
            should_execute,
            suggested_amount,
            llm_analysis,
            technical_signals,
            timestamp: Utc::now(),
        })
    }

    /// Combina análises técnica e do LLM
    fn combine_analyses(
        &self,
        technical_action: &TradeAction,
        technical_confidence: f32,
        llm_analysis: &Option<LlmTradeAnalysis>,
    ) -> (TradeAction, f32, String) {
        match llm_analysis {
            Some(llm) => {
                // Combinar com pesos configurados
                let technical_score = self.action_to_score(technical_action) * self.config.technical_weight;
                let llm_score = self.action_to_score(&llm.action) * self.config.llm_weight;
                let combined_score = technical_score + llm_score;

                let final_action = self.score_to_action(combined_score);
                let final_confidence = (technical_confidence * self.config.technical_weight) + 
                                     (llm.confidence * self.config.llm_weight);

                let reasoning = format!(
                    "Análise combinada: Técnica ({}), LLM ({}). Razão LLM: {}",
                    technical_action_name(technical_action),
                    technical_action_name(&llm.action),
                    llm.reasoning
                );

                (final_action, final_confidence, reasoning)
            }
            None => {
                (
                    technical_action.clone(),
                    technical_confidence,
                    "Análise baseada apenas em indicadores técnicos".to_string(),
                )
            }
        }
    }

    fn action_to_score(&self, action: &TradeAction) -> f32 {
        match action {
            TradeAction::StrongBuy => 2.0,
            TradeAction::Buy => 1.0,
            TradeAction::Hold => 0.0,
            TradeAction::Sell => -1.0,
            TradeAction::StrongSell => -2.0,
        }
    }

    fn score_to_action(&self, score: f32) -> TradeAction {
        match score {
            x if x >= 1.5 => TradeAction::StrongBuy,
            x if x >= 0.5 => TradeAction::Buy,
            x if x <= -1.5 => TradeAction::StrongSell,
            x if x <= -0.5 => TradeAction::Sell,
            _ => TradeAction::Hold,
        }
    }

    /// Avalia risco geral da operação
    fn assess_risk(
        &self,
        technical_signals: &TechnicalSignals,
        llm_analysis: &Option<LlmTradeAnalysis>,
    ) -> RiskLevel {
        let mut risk_factors = 0;

        // Risco por volatilidade
        match technical_signals.volatility_signal {
            VolatilitySignal::VeryHigh => risk_factors += 2,
            VolatilitySignal::High => risk_factors += 1,
            _ => {},
        }

        // Risco por LLM
        if let Some(llm) = llm_analysis {
            match llm.risk_level {
                RiskLevel::VeryHigh => risk_factors += 2,
                RiskLevel::High => risk_factors += 1,
                _ => {},
            }
        }

        // Converter em nível de risco
        match risk_factors {
            0 => RiskLevel::Low,
            1 => RiskLevel::Medium,
            2..=3 => RiskLevel::High,
            _ => RiskLevel::VeryHigh,
        }
    }

    /// Decide se deve executar o trade
    fn should_execute_trade(
        &self,
        confidence: f32,
        risk: &RiskLevel,
        action: &TradeAction,
        _current_balance: f64,
        _current_btc: f64,
    ) -> bool {
        // Verificar confiança mínima
        if confidence < self.config.min_confidence {
            return false;
        }

        // Verificar tolerância a risco
        let risk_acceptable = match (&self.config.risk_tolerance, risk) {
            (RiskLevel::Low, RiskLevel::Medium | RiskLevel::High | RiskLevel::VeryHigh) => false,
            (RiskLevel::Medium, RiskLevel::VeryHigh) => false,
            (RiskLevel::High, RiskLevel::VeryHigh) => false,
            _ => true,
        };

        if !risk_acceptable {
            return false;
        }

        // Não executar se ação é Hold
        !matches!(action, TradeAction::Hold)
    }

    /// Calcula quantidade a ser negociada
    fn calculate_trade_amount(&self, action: &TradeAction, confidence: f32, risk: &RiskLevel) -> f32 {
        let base_amount = match action {
            TradeAction::StrongBuy | TradeAction::StrongSell => 0.15, // 15% do saldo
            TradeAction::Buy | TradeAction::Sell => 0.10,             // 10% do saldo
            TradeAction::Hold => 0.0,
        };

        // Ajustar por confiança
        let confidence_multiplier = confidence;

        // Ajustar por risco (menor quantidade para maior risco)
        let risk_multiplier = match risk {
            RiskLevel::Low => 1.0,
            RiskLevel::Medium => 0.8,
            RiskLevel::High => 0.6,
            RiskLevel::VeryHigh => 0.4,
        };

        (base_amount * confidence_multiplier * risk_multiplier).min(0.20) // Máximo 20%
    }
}

fn technical_action_name(action: &TradeAction) -> &'static str {
    match action {
        TradeAction::StrongBuy => "COMPRA FORTE",
        TradeAction::Buy => "COMPRA",
        TradeAction::Hold => "MANTER",
        TradeAction::Sell => "VENDA",
        TradeAction::StrongSell => "VENDA FORTE",
    }
}