# 🚀 BTC Trade Simulator

Um simulador avançado de trading de Bitcoin em Rust que implementa uma estratégia de **Dollar Cost Averaging (DCA)** inteligente com rastreamento individual de ordens, proteção contra quedas bruscas e **execução em background com persistência de estado**.

## 📋 Características Principais

- **🎯 Estratégia DCA Inteligente**: Compra apenas após 3 quedas consecutivas ou quedas severas
- **🚨 Proteção de Emergência**: Compra imediata em quedas do dobro do percentual
- **📊 Rastreamento Individual**: Cada ordem de compra é rastreada individualmente
- **💰 Take Profit Automático**: Venda automática quando atingir percentual de lucro
- **💾 Persistência de Estado**: Salva progresso automaticamente, pode parar e continuar
- **🚀 Modo Daemon**: Executa em background independente do terminal
- **📊 Logs em Tempo Real**: Acompanhe a simulação mesmo com terminal fechado
- **📈 Relatórios Detalhados**: Histórico completo de transações e estatísticas
- **⚡ Performance**: Dados armazenados em Redis para acesso rápido
- **🔧 Configurável**: Parâmetros ajustáveis para diferentes estratégias

## 🏗️ Arquitetura

```
src/
├── main.rs              # Ponto de entrada e comandos CLI
├── trade_btc.rs         # Simulador de trading e lógica principal
├── redis_client.rs      # Cliente Redis com reconexão automática
└── reader_csv.rs        # Leitor de dados históricos BTC

Arquivos gerados:
├── simulation_state.json    # Estado da simulação (auto-salvo)
├── simulation.pid          # PID do processo daemon
└── logs/btc_trading.log.*  # Logs rotativos diários
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
cd rust-trade-btc
```

2. **Configure as variáveis de ambiente** (opcional)
```bash
export REDIS_URL="redis://localhost:6379"
export REDIS_MAX_RETRIES=3
export REDIS_TIMEOUT=10
```

3. **Primeira execução - Carregue os dados**
```bash
cargo run
```

### Primeira Execução

Na primeira execução, o sistema irá:
1. Carregar dados históricos do arquivo CSV
2. Processar e armazenar milhões de registros no Redis
3. Mostrar comandos disponíveis

⚠️ **Nota**: O carregamento inicial pode levar alguns minutos dependendo do hardware.

## 🎮 Comandos Disponíveis

### **Modo Interativo (Terminal Ativo)**
```bash
# Executar simulação normalmente (pode parar com Ctrl+C)
cargo run simulate

# Iniciar simulação nova (limpa estado anterior)
cargo run fresh
```

### **Modo Daemon (Background)**
```bash
# Iniciar simulação em background
cargo run daemon

# Acompanhar logs em tempo real (pode fechar terminal)
cargo run logs

# Verificar status da simulação
cargo run status

# Parar simulação em background
cargo run stop
```

### **Gerenciamento de Estado**
```bash
# Limpar arquivo de estado (recomeçar simulação)
cargo run clear
```

### **Exemplo de Fluxo Completo**

```bash
# 1. Iniciar em background
cargo run daemon
# ✅ Simulação iniciada em background (PID: 12345)

# 2. Fechar terminal/desconectar SSH
exit

# 3. Reconectar depois e verificar
cargo run status
# 🟢 Status: RODANDO (PID: 12345)
# 📅 Última data: 2018-01-05T14:30:00
# 💰 Saldo Fiat: $9,456.78

# 4. Acompanhar logs em tempo real
cargo run logs
# [14:30:45] INFO: 📉 QUEDA DETECTADA #2: -3.15%
# [14:30:46] INFO: ⏳ AGUARDANDO: 2/3 quedas
# [14:31:12] INFO: ✅ COMPRA LIBERADA: 3 quedas atingidas!

# 5. Parar quando quiser
cargo run stop
# ✅ Simulação parada
# 💾 Estado foi salvo automaticamente
```

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
    initial_balance: 100.0,             // Saldo inicial em USD
    trade_percentage: 5.0,              // 5% do saldo por compra
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
💰 Saldo inicial: $100.00
📊 Perda máxima aceitável: 50.0%
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
💰 Quantidade BTC: 0.001218 BTC
💵 Preço de compra: $41084.45
💸 Valor investido: $5.00
🏦 Saldo fiat restante: $95.00
📊 Total BTC em carteira: 0.001218 BTC
📋 Ordens ativas: 1
💸 Total investido: $5.00 / $90.00 (90% limite)
================================================================================
```

## 💾 Sistema de Persistência

### **Salvamento Automático**
- Estado salvo automaticamente a cada **30 segundos**
- Estado salvo no final da simulação
- Estado salvo quando a simulação é interrompida

### **Recuperação de Estado**
- Continua exatamente de onde parou
- Mantém todos os saldos, ordens ativas e progresso
- Funciona entre reinicializações do sistema

### **Arquivos de Estado**
```json
// simulation_state.json (exemplo)
{
  "current_time": "2018-01-05T14:30:00Z",
  "data_index": 5350,
  "saldo_fiat": 91.07,
  "saldo_btc": 0.000665,
  "buy_orders": [...],
  "transaction_history": [...],
  "stats": {...}
}
```

## 📊 Monitoramento e Logs

### **Status em Tempo Real**
```bash
cargo run status
```
```
📊 Status da Simulação
==================================================
🟢 Status: RODANDO (PID: 12345)
💾 Estado salvo: SIM
📅 Última data: 2018-01-05T14:30:00
📊 Índice atual: 5350
💰 Saldo Fiat: $91.07
₿  Saldo BTC: 0.000665 BTC
📄 Log de hoje: logs/btc_trading.log.2025-08-03 (36280 bytes)
```

### **Logs em Tempo Real**
```bash
cargo run logs
```
```
📊 Acompanhando logs da simulação em tempo real...
================================================================================
[14:30:45] INFO: 📉 QUEDA DETECTADA #2: -3.15% do pico $43769.23
[14:30:46] INFO: ⏳ AGUARDANDO: 2/3 quedas para próxima compra
[14:31:12] INFO: ✅ COMPRA LIBERADA: 3 quedas atingidas!
[14:31:12] INFO: 🎯 COMPRA POR QUEDA REALIZADA - Ordem #5
[14:35:22] INFO: 💚 VENDA COM LUCRO - Ordem #3 - Lucro: $0.45 (6.01%)
```

### **Logs Importantes**

- `📉 QUEDA DETECTADA`: Queda identificada
- `✅ COMPRA LIBERADA`: Compra após 3 quedas
- `🚨 COMPRA DE EMERGÊNCIA`: Compra em queda severa
- `💚 VENDA COM LUCRO`: Ordem vendida com lucro
- `⏳ AGUARDANDO`: Esperando mais quedas
- `💾 Estado salvo`: Progresso salvo automaticamente

## 📈 Relatórios e Estatísticas

### **Relatório Final Detalhado**
```
🏁 RELATÓRIO FINAL DE TRADING 🏁
═══════════════════════════════════════════════════════════════════════════════
🎉 RESULTADO: LUCRO
───────────────────────────────────────────────────────────────────────────────
💰 SALDO INICIAL:       $100.00      │ 💰 SALDO FINAL: $156.78
📊 RETORNO LÍQUIDO:     $56.78 (+56.78%) │ 🏦 Saldo Fiat: $145.23
💎 BTC restante:        0.000231 BTC │ 💎 Valor BTC: $11.55
───────────────────────────────────────────────────────────────────────────────
📈 TOTAL DE LUCROS:     $78.45       │ 📉 TOTAL DE PERDAS: $0.00
🎯 TRADES REALIZADOS:   15           │ 📉 DRAWDOWN MÁXIMO: 8.45%
✅ TRADES VENCEDORES:   15 (100.0%)  │ ❌ TRADES PERDEDORES: 0
───────────────────────────────────────────────────────────────────────────────
💰 LUCRO MÉDIO/TRADE VENCEDOR: $5.23 │ 💸 PERDA MÉDIA/PERDEDOR: $0.00
───────────────────────────────────────────────────────────────────────────────
📊 CONFIGURAÇÃO USADA:
• Percentual por trade: 5.0%    • Take Profit: 6.0%
• Gatilho compra: -3.0%         • Limite investimento: 90%
═══════════════════════════════════════════════════════════════════════════════
```

### **Histórico Completo de Transações**
- Lista todas as compras e vendas
- Tempo de holding de cada posição
- Lucro/prejuízo individual
- Ordens ainda abertas

## 🛠️ Configuração Avançada

### **Modificar Estratégia**

Edite `src/trade_btc.rs` função `run_trade_simulation()`:

```rust
let config = TradeConfig {
    initial_balance: 100.0,                // Saldo inicial
    trade_percentage: 5.0,                 // % do saldo por trade
    percentual_queda_para_comprar: 3.0,    // Gatilho normal
    take_profit_percentage: 6.0,           // Take profit
    quedas_para_comprar: 3,                // Quedas necessárias
    // Emergência automática = percentual_queda_para_comprar * 2
};
```

### **Variáveis de Ambiente**

```bash
# Redis
REDIS_URL="redis://localhost:6379"
REDIS_MAX_RETRIES=3
REDIS_RETRY_DELAY=2
REDIS_TIMEOUT=10

# Logs
RUST_LOG=info,btc_trading_simulator=debug
```

### **Personalizar Logs**

```bash
# Logs básicos
RUST_LOG=info cargo run daemon

# Logs detalhados (debug)
RUST_LOG=debug cargo run daemon

# Logs apenas de erros
RUST_LOG=error cargo run daemon
```

## 🧪 Dados de Teste

O projeto utiliza dados históricos reais do Bitcoin de **2018 a 2025** com:
- ⏰ Frequência: 1 registro por minuto
- 📊 Total: Milhões de registros
- 💾 Armazenamento: Redis para performance
- 🔄 Simulação: Velocidade acelerada (1 minuto = 10ms)

## 🚀 Casos de Uso

### **1. Desenvolvimento/Teste Local**
```bash
cargo run simulate    # Execução interativa
```

### **2. Servidor/VPS (Long Running)**
```bash
cargo run daemon      # Executar em background
cargo run logs        # Monitorar via SSH
```

### **3. Análise de Backtest**
```bash
cargo run fresh       # Nova simulação
cargo run status      # Verificar progresso
cargo run stop        # Parar e analisar resultados
```

### **4. Recuperação após Crash**
```bash
cargo run simulate    # Continua automaticamente
```

## 🤝 Contribuição

Contribuições são bem-vindas! Áreas de melhoria:

- 📈 Novas estratégias de trading
- 🛡️ Mecanismos de stop-loss
- 📊 Análise técnica (RSI, MACD, etc.)
- 🎨 Interface web/dashboard
- 📈 Backtesting com diferentes ativos
- 🔔 Notificações (email, webhook, etc.)
- 📱 API REST para integração

## 📄 Licença

Este projeto é open source e está disponível sob a licença MIT.

---

**⚠️ Aviso Legal**: Este é um simulador educacional. Não constitui aconselhamento financeiro. Trading de criptomoedas envolve risco de perda total do capital investido.