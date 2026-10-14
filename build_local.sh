#!/usr/bin/env bash
# ==============================================================================
# BenderShot - Script de Build Local & Empacotamento
# Compila o binário em Rust e gera pacotes locais (.rpm, .deb, .AppImage, .tar.gz)
# ==============================================================================
set -euo pipefail

# ANSI Colors
BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"
DIM="\033[2m"
RESET="\033[0m"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="${ROOT_DIR}/target"
DIST_DIR="${ROOT_DIR}/packaging/dist"
BIN_NAME="bendershot"

# Flags
BUILD_MODE="release"
DO_PACKAGE=true
DO_TEST=false
DO_INSTALL=false

print_banner() {
    echo -e "${CYAN}${BOLD}"
    echo "=========================================================="
    echo "         BenderShot - Build & Packaging Local"
    echo "=========================================================="
    echo -e "${RESET}"
}

print_help() {
    print_banner
    echo "Uso: ./build_local.sh [OPÇÕES]"
    echo ""
    echo "Opções:"
    echo "  (sem argumentos)  Compila em modo release e gera todos os pacotes em packaging/dist"
    echo "  -q, --quick       Apenas compila o binário release com cargo (pula empacotamento)"
    echo "  -d, --debug       Compila em modo debug (em vez de release)"
    echo "  -t, --test        Executa a suíte de testes (cargo test) antes do build"
    echo "  -i, --install     Instala o binário compilado localmente em ~/.local/bin/bendershot"
    echo "  -h, --help        Exibe esta mensagem de ajuda"
    echo ""
    echo "Exemplos:"
    echo "  ./build_local.sh            # Build completo + empacotamento (.rpm, .deb, .AppImage)"
    echo "  ./build_local.sh --quick    # Build rápido do binário target/release/bendershot"
    echo "  ./build_local.sh -t -i      # Executa testes, compila e instala localmente"
    exit 0
}

# Parse command line options
while [[ $# -gt 0 ]]; do
    case "$1" in
        -q|--quick)
            DO_PACKAGE=false
            shift
            ;;
        -d|--debug)
            BUILD_MODE="debug"
            DO_PACKAGE=false
            shift
            ;;
        -t|--test)
            DO_TEST=true
            shift
            ;;
        -i|--install)
            DO_INSTALL=true
            shift
            ;;
        -h|--help)
            print_help
            ;;
        *)
            echo -e "${RED}Opção desconhecida: $1${RESET}" >&2
            echo "Execute './build_local.sh --help' para ver as opções disponíveis." >&2
            exit 1
            ;;
    esac
done

print_banner

# 1. Check prerequisites
echo -e "${BOLD}==> Verificando dependências de compilação...${RESET}"
if ! command -v cargo &>/dev/null; then
    echo -e "${RED}Erro: 'cargo' não foi encontrado no PATH. Instale o Rust (https://rustup.rs).${RESET}" >&2
    exit 1
fi
if ! command -v rustc &>/dev/null; then
    echo -e "${RED}Erro: 'rustc' não foi encontrado no PATH.${RESET}" >&2
    exit 1
fi

RUSTC_VER="$(rustc --version)"
CARGO_VER="$(cargo --version)"
PROJECT_VER="$(grep -m1 '^version' "${ROOT_DIR}/Cargo.toml" | cut -d '"' -f2)"

echo -e "  [✔] Rust:    ${GREEN}${RUSTC_VER}${RESET}"
echo -e "  [✔] Cargo:   ${GREEN}${CARGO_VER}${RESET}"
echo -e "  [✔] Versão:  ${GREEN}v${PROJECT_VER}${RESET} (modo: ${BUILD_MODE})"

# 2. Run tests if requested
if [ "${DO_TEST}" = true ]; then
    echo ""
    echo -e "${BOLD}==> Executando testes unitários e de integração...${RESET}"
    cargo test --manifest-path "${ROOT_DIR}/Cargo.toml"
    echo -e "${GREEN}✔ Todos os testes passaram com sucesso!${RESET}"
fi

# 3. Compile binary
echo ""
echo -e "${BOLD}==> Compilando BenderShot com Cargo (${BUILD_MODE})...${RESET}"
if [ "${BUILD_MODE}" = "release" ]; then
    cargo build --release --manifest-path "${ROOT_DIR}/Cargo.toml"
    BINARY="${TARGET_DIR}/release/${BIN_NAME}"
else
    cargo build --manifest-path "${ROOT_DIR}/Cargo.toml"
    BINARY="${TARGET_DIR}/debug/${BIN_NAME}"
fi

if [ ! -f "${BINARY}" ]; then
    echo -e "${RED}Erro: Binário ${BINARY} não foi gerado!${RESET}" >&2
    exit 1
fi

BIN_SIZE=$(ls -lh "${BINARY}" | awk '{print $5}')
echo -e "${GREEN}✔ Compilação concluída:${RESET} ${BINARY} (${BIN_SIZE})"

# 4. Packaging if enabled
if [ "${DO_PACKAGE}" = true ]; then
    echo ""
    echo -e "${BOLD}==> Gerando pacotes de distribuição (.rpm, .deb, .AppImage, .tar.gz)...${RESET}"
    "${ROOT_DIR}/packaging/package.sh"
fi

# 5. Local installation if requested
if [ "${DO_INSTALL}" = true ]; then
    echo ""
    echo -e "${BOLD}==> Instalando localmente no sistema do usuário...${RESET}"
    USER_BIN="${HOME}/.local/bin"
    USER_APPS="${HOME}/.local/share/applications"
    USER_ICONS="${HOME}/.local/share/icons/hicolor"
    
    mkdir -p "${USER_BIN}" "${USER_APPS}" "${USER_ICONS}/scalable/apps" "${USER_ICONS}/256x256/apps" "${USER_ICONS}/64x64/apps"
    
    # Copy binary and symlink
    cp -f "${BINARY}" "${USER_BIN}/${BIN_NAME}"
    chmod 755 "${USER_BIN}/${BIN_NAME}"
    ln -sf "${BIN_NAME}" "${USER_BIN}/enhanced-shot"
    echo -e "  [✔] Binário instalado em: ${GREEN}${USER_BIN}/${BIN_NAME}${RESET}"

    # Copy icons
    if [ -f "${ROOT_DIR}/packaging/assets/bendershot.svg" ]; then
        cp -f "${ROOT_DIR}/packaging/assets/bendershot.svg" "${USER_ICONS}/scalable/apps/"
        cp -f "${ROOT_DIR}/packaging/assets/bendershot.png" "${USER_ICONS}/256x256/apps/"
        cp -f "${ROOT_DIR}/packaging/assets/bendershot_64.png" "${USER_ICONS}/64x64/apps/bendershot.png"
    fi

    # Copy desktop launcher
    if [ -f "${ROOT_DIR}/packaging/assets/bendershot.desktop" ]; then
        cp -f "${ROOT_DIR}/packaging/assets/bendershot.desktop" "${USER_APPS}/"
    fi

    # Update caches
    if command -v update-desktop-database &>/dev/null; then
        update-desktop-database "${USER_APPS}" &>/dev/null || true
    fi
    if command -v gtk-update-icon-cache &>/dev/null; then
        gtk-update-icon-cache -f -t "${USER_ICONS}" &>/dev/null || true
    fi

    # Auto setup permissions and restart service if active
    "${USER_BIN}/${BIN_NAME}" auto-setup || true
    if systemctl --user is-active bendershot.service &>/dev/null; then
        systemctl --user restart bendershot.service || true
        echo -e "  [✔] Serviço daemon do usuário reiniciado."
    fi
    echo -e "${GREEN}✔ Instalação local concluída com sucesso!${RESET}"
fi

# 6. Summary
echo ""
echo -e "${GREEN}${BOLD}==========================================================${RESET}"
echo -e "${GREEN}${BOLD}              BUILD CONCLUÍDO COM SUCESSO!                ${RESET}"
echo -e "${GREEN}${BOLD}==========================================================${RESET}"
echo -e "Binário gerado: ${CYAN}${BINARY}${RESET} (${BIN_SIZE})"

if [ "${DO_PACKAGE}" = true ] && [ -d "${DIST_DIR}" ]; then
    echo ""
    echo -e "${BOLD}Artefatos disponíveis em ${DIST_DIR}:${RESET}"
    ls -lh "${DIST_DIR}" | awk 'NR>1 {printf "  %-40s %s\n", $9, $5}'
fi
echo ""
