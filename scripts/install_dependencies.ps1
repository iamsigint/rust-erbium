# Script para instalar dependências necessárias para compilar o Erbium-Node
# Execute este script como administrador

# Verificar se o Chocolatey está instalado
if (!(Get-Command choco -ErrorAction SilentlyContinue)) {
    Write-Host "Instalando Chocolatey..."
    Set-ExecutionPolicy Bypass -Scope Process -Force
    [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))
}

# Instalar Perl (necessário para OpenSSL)
Write-Host "Instalando Perl (necessário para OpenSSL)..."
choco install strawberryperl -y

# Instalar LLVM (necessário para algumas dependências)
Write-Host "Instalando LLVM..."
choco install llvm -y

# Instalar OpenSSL
Write-Host "Instalando OpenSSL..."
choco install openssl -y

# Configurar variáveis de ambiente para OpenSSL
$opensslPath = "C:\Program Files\OpenSSL-Win64"
if (Test-Path $opensslPath) {
    [Environment]::SetEnvironmentVariable("OPENSSL_DIR", $opensslPath, "Machine")
    Write-Host "Variável de ambiente OPENSSL_DIR configurada para $opensslPath"
}

# Atualizar PATH para incluir os binários do Perl e OpenSSL
$perlPath = "C:\Strawberry\perl\bin"
if (Test-Path $perlPath) {
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "Machine")
    if (!$currentPath.Contains($perlPath)) {
        [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$perlPath", "Machine")
        Write-Host "Adicionado $perlPath ao PATH"
    }
}

Write-Host "Instalação de dependências concluída!"
Write-Host "Por favor, reinicie seu terminal para que as alterações nas variáveis de ambiente tenham efeito."
