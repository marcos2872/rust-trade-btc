# Sistema de Logging - BTC Trading Simulator

## Visão Geral

Este projeto implementa um sistema completo de logging estruturado para análise futura de todas as operações e eventos do simulador de trading BTC.

## Características do Sistema

### 1. Logging Duplo
- **Console**: Logs formatados e coloridos para visualização em tempo real
- **Arquivo**: Logs estruturados em formato JSON para análise posterior

### 2. Rotação Automática
- Logs são salvos diariamente em `logs/btc_trading.log.YYYY-MM-DD`
- Rotação automática por dia evita arquivos muito grandes

### 3. Níveis de Log
- **ERROR**: Erros críticos e falhas
- **WARN**: Avisos e situações importantes
- **INFO**: Informações gerais sobre operações
- **DEBUG**: Detalhes de execução para depuração

### 4. Formato dos Logs

#### Console (para visualização)
```
2025-08-02T22:12:36.892497Z INFO ThreadId(01) btc_trading_simulator: 🚀 Sistema de logging inicializado
```

#### Arquivo JSON (para análise)
```json
{
  "timestamp": "2025-08-02T22:12:36.892519Z",
  "level": "INFO",
  "fields": {
    "message": "🚀 Sistema de logging inicializado"
  },
  "target": "btc_trading_simulator",
  "threadId": "ThreadId(1)"
}
```

## Eventos Logados

### Sistema
- ✅ Inicialização do sistema de logging
- ✅ Carregamento de dados CSV
- ✅ Conexões Redis
- ✅ Início/fim de simulações

### Trading
- 🎯 **Compras**: Detalhes completos de cada ordem de compra
- 💚 **Vendas**: Informações de lucro, tempo de holding, percentuais
- 📉 **Quedas detectadas**: Monitoramento de gatilhos de compra
- ⚠️ **Limites atingidos**: Avisos sobre limites de investimento
- 🚨 **Emergências**: Compras por quedas severas

### Dados Estruturados nos Logs
- Timestamps precisos (UTC)
- IDs de thread para debugging
- Módulo/função de origem
- Valores numéricos para análise quantitativa

## Configuração

### Variável de Ambiente
Você pode controlar o nível de logging usando a variável `RUST_LOG`:

```bash
# Logs detalhados (DEBUG)
export RUST_LOG=debug
cargo run simulate

# Apenas informações importantes (INFO - padrão)
export RUST_LOG=info
cargo run simulate

# Apenas avisos e erros
export RUST_LOG=warn
cargo run simulate
```

### Localização dos Logs
- **Diretório**: `logs/`
- **Arquivo atual**: `btc_trading.log.YYYY-MM-DD`
- **Formato**: JSON estruturado

## Análise dos Logs

### Exemplos de Filtros para Análise

#### 1. Extrair todas as compras realizadas
```bash
grep '"COMPRA REALIZADA"' logs/btc_trading.log.* | jq .
```

#### 2. Filtrar vendas com lucro
```bash
grep '"VENDA COM LUCRO"' logs/btc_trading.log.* | jq .
```

#### 3. Monitorar quedas detectadas
```bash
grep '"QUEDA DETECTADA"' logs/btc_trading.log.* | jq .
```

#### 4. Análise de performance por tempo
```bash
jq 'select(.level == "INFO" and (.fields.message | contains("LUCRO")))' logs/btc_trading.log.*
```

### Campos Úteis para Análise
- `timestamp`: Para análise temporal
- `level`: Para filtrar por tipo de evento
- `fields.message`: Conteúdo principal do log
- `target`: Módulo que gerou o log

## Benefícios para Análise Futura

1. **Backtesting**: Todos os eventos são preservados com timestamps
2. **Performance**: Métricas detalhadas de cada operação
3. **Debugging**: Logs estruturados facilitam identificação de problemas
4. **Auditoria**: Histórico completo de todas as decisões de trading
5. **Otimização**: Dados para ajustar parâmetros da estratégia

## Exemplo de Uso para Análise

```python
import json
import pandas as pd
from datetime import datetime

# Carregar logs
logs = []
with open('logs/btc_trading.log.2025-08-02', 'r') as f:
    for line in f:
        logs.append(json.loads(line))

# Converter para DataFrame
df = pd.DataFrame(logs)

# Filtrar compras
compras = df[df['fields'].str.contains('COMPRA REALIZADA')]

# Análise temporal
df['timestamp'] = pd.to_datetime(df['timestamp'])
df.set_index('timestamp', inplace=True)

# Estatísticas por hora/dia
stats_daily = df.groupby(df.index.date).size()
```

## Manutenção

- Logs antigos podem ser arquivados ou removidos conforme necessário
- O sistema cria automaticamente o diretório `logs/` se não existir
- Não há limite de tamanho por arquivo - cada dia é um arquivo separado