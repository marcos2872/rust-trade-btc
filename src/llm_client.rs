use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use anyhow::{Result, anyhow};

/// Configuração para o cliente LLM
#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: "http://10.105.130.198:11434".to_string(),
            model: "llama3:8b".to_string(),
            timeout: Duration::from_secs(30),
            max_tokens: 1000,
            temperature: 0.7,
        }
    }
}

impl LlmConfig {
    pub fn from_env() -> Self {
        Self {
            base_url: env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "http://10.105.130.198:11434".to_string()),
            model: env::var("LLM_MODEL")
                .unwrap_or_else(|_| "llama3:8b".to_string()),
            timeout: Duration::from_secs(
                env::var("LLM_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
            ),
            max_tokens: env::var("LLM_MAX_TOKENS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            temperature: env::var("LLM_TEMPERATURE")
                .unwrap_or_else(|_| "0.7".to_string())
                .parse()
                .unwrap_or(0.7),
        }
    }
}

/// Request para o Ollama API
#[derive(Debug, Serialize)]
pub struct OllamaRequest {
    pub model: String,
    pub prompt: String,
    pub stream: bool,
    pub options: OllamaOptions,
}

#[derive(Debug, Serialize)]
pub struct OllamaOptions {
    pub num_predict: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub top_k: u32,
}

/// Response do Ollama API
#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub model: String,
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
    pub total_duration: Option<u64>,
    pub load_duration: Option<u64>,
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

/// Resposta estruturada do LLM para análise de trading
#[derive(Debug, Clone)]
pub struct LlmTradeAnalysis {
    pub action: crate::decision_engine::TradeAction,
    pub confidence: f32,
    pub reasoning: String,
    pub risk_level: crate::decision_engine::RiskLevel,
    pub price_prediction: Option<f64>,
}

/// Cliente para comunicação com LLM
pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client, config }
    }

    pub fn from_env() -> Self {
        Self::new(LlmConfig::from_env())
    }

    /// Testa a conexão com o servidor LLM
    pub async fn test_connection(&self) -> Result<bool> {
        println!("🧪 Testando conexão com LLM em {}...", self.config.base_url);

        let url = format!("{}/api/tags", self.config.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!("✅ LLM conectado com sucesso!");
                    Ok(true)
                } else {
                    println!("❌ LLM respondeu com status: {}", response.status());
                    Ok(false)
                }
            }
            Err(e) => {
                println!("❌ Erro ao conectar com LLM: {}", e);
                Ok(false)
            }
        }
    }

    /// Envia prompt para o LLM e retorna a resposta
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.config.base_url);
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            options: OllamaOptions {
                num_predict: self.config.max_tokens,
                temperature: self.config.temperature,
                top_p: 0.9,
                top_k: 40,
            },
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("LLM API retornou status: {}", response.status()));
        }

        let ollama_response: OllamaResponse = response.json().await?;
        Ok(ollama_response.response)
    }

    /// Analisa dados de mercado e retorna recomendação de trading
    pub async fn analyze_market_data(&self, market_context: &str) -> Result<LlmTradeAnalysis> {
        let prompt = format!(
            r#"Você é um especialista em trading de Bitcoin. Analise os seguintes dados de mercado e forneça uma recomendação de trading.

DADOS DE MERCADO:
{}

Por favor, responda EXATAMENTE no seguinte formato JSON:
{{
    "action": "BUY|SELL|HOLD|STRONG_BUY|STRONG_SELL",
    "confidence": 0.85,
    "reasoning": "Sua análise detalhada aqui",
    "risk_level": "LOW|MEDIUM|HIGH|VERY_HIGH",
    "price_prediction": 45000.50
}}

Regras importantes:
1. "action" deve ser uma das opções: BUY, SELL, HOLD, STRONG_BUY, STRONG_SELL
2. "confidence" deve ser um número entre 0.0 e 1.0
3. "reasoning" deve explicar sua análise em português
4. "risk_level" deve ser: LOW, MEDIUM, HIGH ou VERY_HIGH
5. "price_prediction" é opcional, mas se fornecida deve ser um número
6. Responda APENAS o JSON, sem texto adicional

ANÁLISE:"#, market_context);

        let response = self.generate(&prompt).await?;
        self.parse_trade_analysis(&response)
    }

    /// Faz parsing da resposta do LLM para estrutura de análise
    fn parse_trade_analysis(&self, response: &str) -> Result<LlmTradeAnalysis> {
        // Tentar extrair JSON da resposta
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];

        // Parse do JSON
        let json_value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Erro ao fazer parse do JSON do LLM: {} - Resposta: {}", e, response))?;

        // Extrair campos
        let action_str = json_value["action"]
            .as_str()
            .ok_or_else(|| anyhow!("Campo 'action' não encontrado na resposta do LLM"))?;

        let action = match action_str.to_uppercase().as_str() {
            "BUY" => crate::decision_engine::TradeAction::Buy,
            "SELL" => crate::decision_engine::TradeAction::Sell,
            "HOLD" => crate::decision_engine::TradeAction::Hold,
            "STRONG_BUY" => crate::decision_engine::TradeAction::StrongBuy,
            "STRONG_SELL" => crate::decision_engine::TradeAction::StrongSell,
            _ => crate::decision_engine::TradeAction::Hold, // Default para Hold se não reconhecer
        };

        let confidence = json_value["confidence"]
            .as_f64()
            .unwrap_or(0.5) as f32;

        let reasoning = json_value["reasoning"]
            .as_str()
            .unwrap_or("Análise não disponível")
            .to_string();

        let risk_level_str = json_value["risk_level"]
            .as_str()
            .unwrap_or("MEDIUM");

        let risk_level = match risk_level_str.to_uppercase().as_str() {
            "LOW" => crate::decision_engine::RiskLevel::Low,
            "MEDIUM" => crate::decision_engine::RiskLevel::Medium,
            "HIGH" => crate::decision_engine::RiskLevel::High,
            "VERY_HIGH" => crate::decision_engine::RiskLevel::VeryHigh,
            _ => crate::decision_engine::RiskLevel::Medium,
        };

        let price_prediction = json_value["price_prediction"]
            .as_f64();

        Ok(LlmTradeAnalysis {
            action,
            confidence: confidence.clamp(0.0, 1.0),
            reasoning,
            risk_level,
            price_prediction,
        })
    }

    /// Verifica se o LLM está disponível
    pub async fn is_available(&self) -> bool {
        self.test_connection().await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_client_creation() {
        let client = LlmClient::from_env();
        assert_eq!(client.config.base_url, "http://10.105.130.198:11434");
    }
}