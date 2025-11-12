#!/bin/bash
# Script para instalar dependências necessárias para compilar o Erbium-Node no Linux
# Execute este script como sudo: sudo ./scripts/install_dependencies.sh

# Verificar se está rodando como root
if [ "$EUID" -ne 0 ]; then
  echo "Por favor, execute este script como root (sudo ./scripts/install_dependencies.sh)"
  exit 1
fi

# Detectar distribuição Linux
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    VER=$VERSION_ID
else
    echo "Não foi possível detectar a distribuição Linux"
    exit 1
fi

echo "Instalando dependências para $OS $VER..."

# Instalar dependências comuns
if [[ "$OS" == *"Ubuntu"* ]] || [[ "$OS" == *"Debian"* ]]; then
    apt-get update
    apt-get install -y build-essential pkg-config libssl-dev clang llvm cmake perl
elif [[ "$OS" == *"Fedora"* ]]; then
    dnf install -y gcc gcc-c++ make pkgconfig openssl-devel clang llvm cmake perl
elif [[ "$OS" == *"CentOS"* ]] || [[ "$OS" == *"Red Hat"* ]]; then
    yum install -y gcc gcc-c++ make pkgconfig openssl-devel clang llvm cmake perl
elif [[ "$OS" == *"Arch"* ]]; then
    pacman -Sy --noconfirm base-devel openssl clang llvm cmake perl
else
    echo "Distribuição não suportada: $OS"
    echo "Por favor, instale manualmente: gcc, g++, make, pkg-config, libssl-dev, clang, llvm, cmake, perl"
    exit 1
fi

# Verificar se o Rust está instalado
if ! command -v rustc &> /dev/null; then
    echo "Rust não encontrado. Instalando..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Rust já está instalado"
fi

echo "Instalação de dependências concluída!"
echo "Agora você pode compilar o Erbium-Node com 'cargo build'"
