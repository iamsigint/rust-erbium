#!/bin/bash
# Erbium Node API Testing Script for Linux/Mac
# Usage: ./test_api_linux.sh

BASE_URL="http://localhost:8080"
RPC_URL="http://localhost:8545"

echo "🧪 === Testando API REST do Erbium-Node ==="
echo ""

# Testes GET
echo "1. 📊 Informações do nó:"
curl -s "$BASE_URL/node" | jq '.' 2>/dev/null || curl -s "$BASE_URL/node"
echo ""

echo "2. ⛓️  Informações da chain:"
curl -s "$BASE_URL/chain" | jq '.' 2>/dev/null || curl -s "$BASE_URL/chain"
echo ""

echo "3. 🧱 Lista de blocos:"
curl -s "$BASE_URL/blocks" | jq '.' 2>/dev/null || curl -s "$BASE_URL/blocks"
echo ""

echo "4. 💸 Lista de transações:"
curl -s "$BASE_URL/transactions" | jq '.' 2>/dev/null || curl -s "$BASE_URL/transactions"
echo ""

echo "5. 🛡️  Lista de validadores:"
curl -s "$BASE_URL/validators" | jq '.' 2>/dev/null || curl -s "$BASE_URL/validators"
echo ""

echo "6. ❤️  Health check:"
curl -s "$BASE_URL/health" | jq '.' 2>/dev/null || curl -s "$BASE_URL/health"
echo ""

echo "7. 🌉 Lista de bridges:"
curl -s "$BASE_URL/bridges" | jq '.' 2>/dev/null || curl -s "$BASE_URL/bridges"
echo ""

echo "8. 🔄 Lista de transferências de bridge:"
curl -s "$BASE_URL/bridges/transfers" | jq '.' 2>/dev/null || curl -s "$BASE_URL/bridges/transfers"
echo ""

echo "🔗 === Testando API RPC do Erbium-Node ==="
echo ""

echo "9. 🔢 Número do bloco:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' | jq '.' 2>/dev/null || echo "RPC não disponível"
echo ""

echo "10. 🌐 Versão da rede:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"net_version","params":[],"id":2}' | jq '.' 2>/dev/null || echo "RPC não disponível"
echo ""

echo "11. ℹ️  Informações da chain:"
curl -s -X POST "$RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"erb_chainInfo","params":[],"id":3}' | jq '.' 2>/dev/null || echo "RPC não disponível"
echo ""

echo "✅ === Todos os testes concluídos ==="
echo ""
echo "💡 Dicas:"
echo "  - Instale jq para melhor formatação JSON: sudo apt-get install jq"
echo "  - Para WebSocket: instale websocat e execute: websocat ws://localhost:8546/ws"
echo "  - Verifique se o nó está rodando antes de executar os testes"
