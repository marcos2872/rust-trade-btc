# 🚀 BTC Trade Simulator

Um simulador avançado de trading de Bitcoin em Rust que implementa uma estratégia de **Dollar Cost Averaging (DCA)** inteligente com rastreamento individual de ordens e proteção contra quedas bruscas.

## 📋 Características Principais

- **🎯 Estratégia DCA Inteligente**: Compra apenas após 3 quedas consecutivas ou quedas severas
- **🚨 Proteção de Emergência**: Compra imediata em quedas do dobro do percentual
- **📊 Rastreamento Individual**: Cada ordem de compra é rastreada individualmente
- **💰 Take Profit Automático**: Venda automática quando atingir percentual de lucro
- **📈 Relatórios Detalhados**: Histórico completo de transações e estatísticas
- **⚡ Performance**: Dados armazenados em Redis para acesso rápido
- **🔧 Configurável**: Parâmetros ajustáveis para diferentes estratégias

## 🏗️ Arquitetura

```
src/
├── main.rs              # Ponto de entrada da aplicação
├── trade_btc.rs         # Simulador de trading e lógica principal
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

### Instalação e Execução

1. **Clone o repositório**
```bash
git clone <seu-repositorio>
cd btc-trade
```

2. **Configure as variáveis de ambiente** (opcional)
```bash
export REDIS_URL="redis://localhost:6379"
export REDIS_MAX_RETRIES=3
export REDIS_TIMEOUT=10
```

3. **Execute o projeto**
```bash
cargo run
```

### Primeira Execução

Na primeira execução, o sistema irá:
1. Carregar dados históricos do arquivo `data.zip`
2. Processar e armazenar ~4 milhões de registros no Redis
3. Iniciar a simulação de trading

⚠️ **Nota**: O carregamento inicial pode levar alguns minutos dependendo do hardware.

## 🧠 Lógica de Trading

### 📈 Estratégia DCA Inteligente

O simulador implementa uma versão sofisticada do Dollar Cost Averaging:

#### 🎯 Condições de Compra

1. **Primeira Compra**: Executa imediatamente ao iniciar
2. **Compra por Quedas**: Após detectar 3 quedas consecutivas
3. **Compra de Emergência**: Imediata em quedas severas (dobro do percentual)

#### 📊 Parâmetros Configuráveis

```rust
TradeConfig {
    initial_balance: 10000.0,           // Saldo inicial em USD
    trade_percentage: 10.0,             // 10% do saldo por compra
    percentual_queda_para_comprar: 3.0, // Gatilho de queda: 3%
    take_profit_percentage: 6.0,        // Venda com 6% de lucro
    quedas_para_comprar: 3,             // Comprar a cada 3 quedas
}
```

### 🔄 Fluxo de Trading

#### Detecção de Quedas
```
Pico: $50,000
├── Queda 3% → $48,500 (Queda #1) ⏳
├── Queda 3% → $47,045 (Queda #2) ⏳  
├── Queda 3% → $45,634 (Queda #3) ✅ COMPRA!
└── Queda 6% → $47,000 🚨 EMERGÊNCIA!
```

#### Sistema de Ordens
```
Ordem #1: 0.1 BTC @ $45,000 → Vende @ $47,700 (6% lucro)
Ordem #2: 0.1 BTC @ $42,000 → Aguardando...
Ordem #3: 0.1 BTC @ $40,000 → Aguardando...
```

### 💡 Exemplo de Execução

```bash
🚀 Iniciando simulador de trade BTC
💰 Saldo inicial: $10000.00
📊 Perda máxima aceitável: 25.0%
🎯 Stop Loss: 0.0% | Take Profit: 6.0%
⏰ Período: 2018-01-01 00:00:00 UTC até 2025-07-22 18:43:00 UTC

📉 QUEDA DETECTADA #1: -3.24% do pico $45234.50 para $43769.23
⏳ AGUARDANDO: 1/3 quedas para próxima compra (ou queda -6.0% para emergência)

📉 QUEDA DETECTADA #2: -3.15% do pico $43769.23 para $42391.12
⏳ AGUARDANDO: 2/3 quedas para próxima compra (ou queda -6.0% para emergência)

📉 QUEDA DETECTADA #3: -3.08% do pico $42391.12 para $41084.45
✅ COMPRA LIBERADA: 3 quedas atingidas!

================================================================================
🎯 COMPRA POR QUEDA REALIZADA - Ordem #1
--------------------------------------------------------------------------------
💰 Quantidade BTC: 0.024332 BTC
💵 Preço de compra: $41084.45
💸 Valor investido: $1000.00
🏦 Saldo fiat restante: $9000.00
📊 Total BTC em carteira: 0.024332 BTC
📋 Ordens ativas: 1
💸 Total investido: $1000.00 / $9000.00 (90% limite)
================================================================================

💚 VENDA COM LUCRO - Ordem de Compra #1 VENDIDA
--------------------------------------------------------------------------------
📅 Comprada em: 2018-02-06 14:00 - Vendida em: 2018-02-08 09:00
⏱️  Tempo em carteira: 1 dias e 19 horas
💰 BTC vendido: 0.024332 BTC
💵 Preço COMPRA: $41084.45 → Preço VENDA: $43549.52
💸 Investimento: $1000.00 → Valor recebido: $1060.00
🎉 LUCRO: $60.00 (6.00%)
🏦 Saldo fiat atual: $10060.00
📋 Ordens restantes: 0
💸 Total ainda investido: $0.00
================================================================================
```

## 📊 Estatísticas e Relatórios

### Status em Tempo Real
- 💰 Valor total da carteira
- 📈 Profit & Loss atual
- 🎯 Ordens ativas com P&L não realizado
- 📊 Quedas detectadas e próxima compra
- 🚨 Gatilhos de emergência

### Relatório Final
- 🏆 Total de trades vencedores/perdedores
- 💰 Lucro/prejuízo total
- 📉 Drawdown máximo
- ⚡ Taxa de acerto
- 📊 Configuração utilizada

## 🛠️ Configuração Avançada

### Modificar Estratégia

Edite `src/trade_btc.rs` linha 701-709:

```rust
let config = TradeConfig {
    initial_balance: 10000.0,              // Saldo inicial
    trade_percentage: 10.0,                // % do saldo por trade
    percentual_queda_para_comprar: 3.0,    // Gatilho normal
    take_profit_percentage: 6.0,           // Take profit
    quedas_para_comprar: 3,                // Quedas necessárias
    // Emergência automática = percentual_queda_para_comprar * 2
};
```

### Variáveis de Ambiente

```bash
# Redis
REDIS_URL="redis://localhost:6379"
REDIS_MAX_RETRIES=3
REDIS_RETRY_DELAY=2
REDIS_TIMEOUT=10
```

## 🔍 Monitoramento

### Logs Importantes

- `📉 QUEDA DETECTADA`: Queda identificada
- `✅ COMPRA LIBERADA`: Compra após 3 quedas
- `🚨 COMPRA DE EMERGÊNCIA`: Compra em queda severa
- `💚 VENDA COM LUCRO`: Ordem vendida com lucro
- `⚠️ AGUARDANDO`: Esperando mais quedas

### Métricas de Performance

- **Win Rate**: Percentual de trades lucrativos
- **Profit Factor**: Razão lucro/prejuízo
- **Max Drawdown**: Maior perda da carteira
- **Holding Time**: Tempo médio de cada posição

## 🧪 Dados de Teste

O projeto utiliza dados históricos reais do Bitcoin de **2018 a 2025** com:
- ⏰ Frequência: 1 registro por hora
- 📊 Total: ~4 milhões de registros
- 💾 Armazenamento: Redis para performance
- 🔄 Simulação: Velocidade acelerada

## 🤝 Contribuição

Contribuições são bem-vindas! Áreas de melhoria:

- 📈 Novas estratégias de trading
- 🛡️ Mecanismos de stop-loss
- 📊 Análise técnica (RSI, MACD, etc.)
- 🎨 Interface gráfica
- 📈 Backtesting com diferentes ativos

## 📄 Licença

Este projeto é open source e está disponível sob a licença MIT.

---

**⚠️ Aviso Legal**: Este é um simulador educacional. Não constitui aconselhamento financeiro. Trading de criptomoedas envolve risco de perda total do capital investido.
