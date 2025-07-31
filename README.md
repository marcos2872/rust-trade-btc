# 🚀 BTC Trade Simulator com IA

Um simulador avançado de trading de Bitcoin em Rust que implementa uma estratégia de **Dollar Cost Averaging (DCA)** inteligente com **Integração de LLM (Llama3:8b)** para análise de mercado, rastreamento individual de ordens e proteção contra quedas bruscas.

## 📋 Características Principais

- **🤖 IA Integrada**: Análise de mercado com Llama3:8b para decisões de trading
- **🎯 Estratégia DCA Inteligente**: Compra após quedas ou decisões do LLM  
- **🚨 Proteção de Emergência**: Compra imediata em quedas severas
- **💰 Gerenciamento de Ordens**: Sistema inteligente de venda por acúmulo
- **📊 Rastreamento Individual**: Cada ordem de compra é rastreada individualmente
- **💎 Take Profit Dinâmico**: Venda automática com critérios adaptativos
- **📈 Relatórios Detalhados**: Histórico completo com análises de IA
- **⚡ Performance**: Dados armazenados em Redis para acesso rápido
- **🔧 Configurável**: Parâmetros ajustáveis para diferentes estratégias

## 🏗️ Arquitetura

```
src/
├── main.rs              # Ponto de entrada da aplicação
├── trade_btc.rs         # Simulador de trading e lógica principal
├── llm_client.rs        # Cliente para comunicação com Llama3:8b
├── market_analysis.rs   # Análise de mercado com IA e indicadores técnicos
├── decision_engine.rs   # Motor de decisão híbrido (IA + Técnico)
├── redis_client.rs      # Cliente Redis com reconexão automática
└── reader_csv.rs        # Leitor de dados históricos BTC
```

## 🚀 Como Rodar

### Pré-requisitos

1. **Rust** (versão 1.70+)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. **Redis Server**
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install redis-server
sudo systemctl start redis-server

# macOS
brew install redis
brew services start redis

# Docker
docker run -d -p 6379:6379 redis:alpine
```

3. **Servidor Ollama com Llama3:8b** (Opcional)
```bash
# Instalar Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Baixar Llama3:8b
ollama pull llama3:8b

# Executar servidor
ollama serve
```

### Instalação e Execução

1. **Clone o repositório**
```bash
git clone <seu-repositorio>
cd rust-trade-btc
```

2. **Configure as variáveis de ambiente** (opcional)
```bash
# Redis
export REDIS_URL="redis://localhost:6379"
export REDIS_MAX_RETRIES=3
export REDIS_TIMEOUT=10

# LLM (Opcional)
export LLM_BASE_URL="http://localhost:11434"
export LLM_MODEL="llama3:8b"
export LLM_TIMEOUT=30
```

3. **Execute o projeto**
```bash
# Teste conexão com LLM
cargo run llm

# Executar simulação
cargo run simulate
```

### Primeira Execução

Na primeira execução, o sistema irá:
1. Carregar dados históricos do arquivo CSV
2. Processar e armazenar registros no Redis
3. Inicializar sistema LLM (se habilitado)
4. Iniciar a simulação de trading

⚠️ **Nota**: O carregamento inicial pode levar alguns minutos dependendo do hardware.

## 🤖 Sistema de IA

### 🧠 Llama3:8b Integration

O simulador utiliza o **Llama3:8b** via Ollama para análise avançada de mercado:

#### 📊 Análise de Mercado
- **Contexto Histórico**: Analisa últimos 100 períodos de dados
- **Indicadores Técnicos**: RSI, SMA, Bollinger Bands, MACD
- **Análise de Sentimento**: Interpretação de padrões de preço
- **Volatilidade**: Avaliação de risco de mercado

#### 🎯 Decisões de Trading
```rust
// Sistema híbrido de decisão
LLM Weight: 70%        // Peso da análise de IA
Technical Weight: 30%  // Peso da análise técnica tradicional
Min Confidence: 60%    // Confiança mínima para executar trades
```

#### 💬 Exemplo de Prompt para LLM
```
ANÁLISE DE MERCADO DO BITCOIN:
📊 PREÇO ATUAL: $43,250.00
📈 VARIAÇÃO 24H: $-1,230.00 (-2.77%)
🔄 TENDÊNCIA: QUEDA

📊 ESTATÍSTICAS:
• Máxima recente: $45,100.00
• Mínima recente: $42,800.00
• Volume: 1,234,567.89
• Volatilidade: 1,250.50 (MÉDIA)

📈 HISTÓRICO DE PREÇOS: $43,100, $43,400, $43,800...

🔍 INDICADORES TÉCNICOS:
• SMA 20: $43,890.25
• RSI: 42.5 (NEUTRO)
• Bollinger Superior: $45,200.30
• Bollinger Inferior: $42,580.20
• MACD: -125.75 (BEARISH)

Por favor, analise e forneça recomendação de trading...
```

### 🔄 Fallback System

Se o LLM não estiver disponível:
- **Análise Técnica**: Continua com indicadores tradicionais
- **DCA Inteligente**: Mantém lógica de quedas e compras
- **Zero Downtime**: Sistema funciona mesmo sem IA

## 🧠 Lógica de Trading

### 📈 Estratégia Híbrida IA + DCA

O simulador combina análise de IA com Dollar Cost Averaging inteligente:

#### 🎯 Condições de Compra

1. **Análise de IA**: LLM recomenda BUY/STRONG_BUY com alta confiança
2. **Primeira Compra**: Executa imediatamente ao iniciar
3. **Compra por Quedas**: Após detectar quedas significativas
4. **Compra de Emergência**: Imediata em quedas severas

#### 💰 Sistema de Vendas Inteligente

```rust
// Vendas Adaptativas por Acúmulo
if ordens_ativas > 5 {
    vender_com_lucro_minimo(1.0%);  // Venda com 1% se muitas ordens
} else {
    vender_com_take_profit(2.0%);   // Venda normal com 2%
}
```

#### 📊 Parâmetros Configuráveis

```rust
TradeConfig {
    // Básico
    initial_balance: 100.0,                 // Saldo inicial em USD
    trade_percentage: 5.0,                  // 5% do saldo por compra
    
    // Take Profit
    take_profit_percentage: 2.0,            // Venda normal: 2%
    max_ordens_acumuladas: 10,              // Máx. ordens antes de venda 1%
    lucro_minimo_acumuladas: 0.7,           // Venda por acúmulo: 0.7%
    
    // DCA Tradicional
    percentual_queda_para_comprar: 0.5,     // Gatilho de queda: 0.5%
    
    // Sistema LLM
    use_llm: true,                          // Habilitar IA
    llm_weight: 0.7,                        // 70% peso IA, 30% técnico
    min_llm_confidence: 0.6,                // Mín. 60% confiança
}
```

### 🔄 Fluxo de Trading com IA

#### Análise de Mercado
```
1. 📊 Coleta dados atuais + últimos 100 períodos
2. 🤖 LLM analisa contexto e indicadores técnicos  
3. 🎯 Motor de decisão combina IA (70%) + técnico (30%)
4. ✅ Executa se confiança > 60%
```

#### Sistema de Ordens Inteligente
```
Cenário: 12 ordens ativas (> 10 limite)
├── Ordem #1: 0.1 BTC @ $40,000 → +2.5% → 🟢 VENDE (>0.7%)
├── Ordem #2: 0.1 BTC @ $39,500 → +1.2% → 🟢 VENDE (>0.7%)  
├── Ordem #3: 0.1 BTC @ $41,000 → -0.5% → ⏳ Aguarda
└── ... 9 outras ordens → 🔄 Critério de acúmulo ativo
```

### 💡 Exemplo de Execução com IA

```bash
🚀 Iniciando simulador de trade BTC
🤖 Inicializando sistema LLM...
✅ Sistema LLM inicializado com sucesso!
💰 Saldo inicial: $100.00
🎯 Take Profit: +0.7% (ACÚMULO) | LLM: 70% peso

🤖 LLM: COMPRA (conf: 78.5%) - Análise técnica indica oversold com RSI em 25.
Padrões históricos sugerem reversão próxima. Volume crescente confirma interesse.

================================================================================
🎯 COMPRA LLM REALIZADA - Ordem #1
--------------------------------------------------------------------------------
💰 Quantidade BTC: 0.001150 BTC
💵 Preço de compra: $43,478.26
💸 Valor investido: $5.00
🏦 Saldo fiat restante: $95.00
🤖 Razão: Análise de IA identificou oportunidade de compra
================================================================================

🔄 MUITAS ORDENS ACUMULADAS (12 ordens) - Usando lucro mínimo de 0.7%

💰 VENDA POR ACÚMULO: Ordem #3 com 0.85% de lucro (critério: 0.7%)
💚 VENDA COM LUCRO - Ordem de Compra #3 VENDIDA
💸 Investimento: $5.00 → Valor recebido: $5.04
🎉 LUCRO: $0.04 (0.85%)
```

## 📊 Comandos Disponíveis

### 🤖 Teste do Sistema LLM
```bash
cargo run llm
```
Verifica conectividade com Llama3:8b e faz teste de geração.

### 🎯 Simulação de Trading
```bash
cargo run simulate
```
Executa simulação completa com IA habilitada.

## 📊 Estatísticas e Relatórios

### Status em Tempo Real
- 💰 Valor total da carteira
- 📈 Profit & Loss atual  
- 🤖 Últimas decisões do LLM
- 🎯 Ordens ativas com P&L não realizado
- 💎 Critério de venda atual (Normal/Acúmulo)
- 📊 Confiança das análises de IA

### Relatório Final
- 🏆 Total de trades vencedores/perdedores
- 💰 Lucro/prejuízo total
- 📉 Drawdown máximo
- ⚡ Taxa de acerto
- 🤖 Performance das decisões de IA vs. técnicas
- 📊 Configuração utilizada

## 🛠️ Configuração Avançada

### Modificar Estratégia

Edite `src/trade_btc.rs` função `run_trade_simulation()`:

```rust
let config = TradeConfig {
    // Saldo e percentuais
    initial_balance: 100.0,
    trade_percentage: 5.0,                  // % do saldo por trade
    
    // Sistema de vendas
    take_profit_percentage: 2.0,            // Take profit normal
    max_ordens_acumuladas: 10,              // Máx ordens para acúmulo  
    lucro_minimo_acumuladas: 0.7,           // Lucro mín. no acúmulo
    
    // DCA tradicional
    percentual_queda_para_comprar: 0.5,     // Gatilho de queda
    
    // Sistema LLM
    use_llm: true,                          // Habilitar/desabilitar IA
    llm_weight: 0.7,                        // Peso da IA (0.0-1.0)
    min_llm_confidence: 0.6,                // Confiança mínima
};
```

### Variáveis de Ambiente

```bash
# Redis
REDIS_URL="redis://10.105.130.198:6379"
REDIS_MAX_RETRIES=3
REDIS_RETRY_DELAY=2
REDIS_TIMEOUT=10

# LLM Llama3:8b
LLM_BASE_URL="http://10.105.130.198:11434"  
LLM_MODEL="llama3:8b"
LLM_TIMEOUT=30
LLM_MAX_TOKENS=1000
LLM_TEMPERATURE=0.7
```

## 🔍 Monitoramento

### Logs Importantes

- `🤖 LLM: COMPRA (conf: 78%)`: Decisão de IA para compra
- `💰 VENDA POR ACÚMULO`: Venda com critério reduzido
- `🔄 MUITAS ORDENS ACUMULADAS`: Ativação do modo acúmulo
- `✅ LLM conectado`: Sistema de IA operacional
- `⚠️ Erro na decisão LLM`: Fallback para análise técnica

### Métricas de Performance

- **IA vs. Técnico**: Comparação de performance entre métodos
- **Win Rate**: Percentual de trades lucrativos
- **Confidence Score**: Média de confiança das decisões de IA
- **LLM Uptime**: Disponibilidade do sistema de IA
- **Max Drawdown**: Maior perda da carteira
- **Order Accumulation**: Frequência de ativação do modo acúmulo

## 🧪 Dados de Teste

O projeto utiliza dados históricos reais do Bitcoin com:
- ⏰ Frequência: 1 registro por minuto/hora
- 📊 Período: Configurável (padrão: fev/2025 - mar/2025)
- 💾 Armazenamento: Redis para performance
- 🔄 Simulação: Velocidade acelerada
- 🤖 IA: Análise em tempo real dos padrões

## 🚀 Tecnologias Utilizadas

- **🦀 Rust**: Performance e segurança
- **🤖 Llama3:8b**: IA para análise de mercado via Ollama
- **📊 Redis**: Armazenamento de dados de alta performance
- **⚡ Tokio**: Runtime async
- **🌐 Reqwest**: Cliente HTTP para comunicação com LLM
- **📈 Chrono**: Manipulação de datas e tempo

## 🤝 Contribuição

Contribuições são bem-vindas! Áreas de melhoria:

- 🤖 Novos modelos de LLM (GPT-4, Claude, etc.)
- 📈 Estratégias de trading avançadas
- 🛡️ Mecanismos de stop-loss inteligentes
- 📊 Análise técnica adicional (Fibonacci, Ichimoku, etc.)
- 🎨 Interface gráfica para visualização
- 📈 Backtesting com diferentes ativos
- 🔄 Trading em tempo real

## 📄 Licença

Este projeto é open source e está disponível sob a licença MIT.

---

**⚠️ Aviso Legal**: Este é um simulador educacional com IA experimental. Não constitui aconselhamento financeiro. Trading de criptomoedas envolve risco de perda total do capital investido. As decisões de IA são baseadas em padrões históricos e não garantem resultados futuros.

**🤖 Disclaimer de IA**: O sistema utiliza Llama3:8b para análise, mas as decisões finais dependem da combinação de múltiplos fatores. A IA pode gerar recomendações imprecisas ou inconsistentes.