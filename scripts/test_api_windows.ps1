# Erbium Node API Testing Script for Windows PowerShell
# Usage: .\test_api_windows.ps1

$BASE_URL = "http://localhost:8080"
$RPC_URL = "http://localhost:8545"

Write-Host "=== Testando API REST do Erbium-Node ===" -ForegroundColor Green
Write-Host ""

# Testes GET
Write-Host "1. Informações do nó:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/node" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "2. Informações da chain:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/chain" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "3. Lista de blocos:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/blocks" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "4. Lista de transações:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/transactions" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "5. Lista de validadores:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/validators" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "6. Health check:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/health" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "7. Lista de bridges:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/bridges" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "8. Lista de transferências de bridge:" -ForegroundColor Yellow
try {
    $response = curl.exe -X GET "$BASE_URL/bridges/transfers" -ErrorAction Stop
    $response
} catch {
    Write-Host "Erro ao conectar: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "=== Testando API RPC do Erbium-Node ===" -ForegroundColor Green
Write-Host ""

Write-Host "9. Número do bloco:" -ForegroundColor Yellow
try {
    $response = curl.exe -X POST "$RPC_URL" `
      -H "Content-Type: application/json" `
      -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' -ErrorAction Stop
    $response
} catch {
    Write-Host "RPC não disponível: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "10. Versão da rede:" -ForegroundColor Yellow
try {
    $response = curl.exe -X POST "$RPC_URL" `
      -H "Content-Type: application/json" `
      -d '{"jsonrpc":"2.0","method":"net_version","params":[],"id":2}' -ErrorAction Stop
    $response
} catch {
    Write-Host "RPC não disponível: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "11. Informações da chain (RPC):" -ForegroundColor Yellow
try {
    $response = curl.exe -X POST "$RPC_URL" `
      -H "Content-Type: application/json" `
      -d '{"jsonrpc":"2.0","method":"erb_chainInfo","params":[],"id":3}' -ErrorAction Stop
    $response
} catch {
    Write-Host "RPC não disponível: $_" -ForegroundColor Red
}
Write-Host ""

Write-Host "=== Todos os testes concluídos ===" -ForegroundColor Green
Write-Host ""
Write-Host "Dicas:" -ForegroundColor Cyan
Write-Host "  - Para WebSocket: npm install -g wscat && wscat -c ws://localhost:8546/ws" -ForegroundColor White
Write-Host "  - Verifique se o nó está rodando antes de executar os testes" -ForegroundColor White
Write-Host "  - Use curl.exe (incluído no Windows 10+)" -ForegroundColor White
