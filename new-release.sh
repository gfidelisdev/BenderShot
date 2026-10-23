#!/usr/bin/env bash
# ==============================================================================
# BenderShot - Script de Nova Release e Build
# Uso: ./new-release.sh <versao> [mensagem_opcional]
# Exemplo: ./new-release.sh 0.4.4
# ==============================================================================
set -euo pipefail

BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"
RESET="\033[0m"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${ROOT_DIR}"

if [ $# -lt 1 ]; then
    echo -e "${RED}${BOLD}Erro: Versão não informada!${RESET}"
    echo "Uso: $0 <versão> [mensagem opcional]"
    echo "Exemplo: $0 0.4.4"
    exit 1
fi

INPUT_VER="$1"
# Remove o prefixo 'v' se foi passado por engano (ex: v0.4.4 -> 0.4.4)
NEW_VER="${INPUT_VER#v}"
TAG_NAME="v${NEW_VER}"

# Valida formato de versão simples (ex: 0.4.4 ou 0.4.4.1)
if ! [[ "${NEW_VER}" =~ ^[0-9]+(\.[0-9]+)+$ ]]; then
    echo -e "${RED}${BOLD}Erro: Formato de versão inválido '${NEW_VER}'. Use números separados por pontos (ex: 0.4.4).${RESET}"
    exit 1
fi

CURRENT_DATE="$(date '+%Y-%m-%d %H:%M:%S')"
CUSTOM_MSG="${2:-}"
if [ -n "${CUSTOM_MSG}" ]; then
    COMMIT_MSG="Release ${TAG_NAME} (${CURRENT_DATE}): ${CUSTOM_MSG}"
else
    COMMIT_MSG="Release ${TAG_NAME} (${CURRENT_DATE})"
fi

echo -e "${CYAN}${BOLD}==========================================================${RESET}"
echo -e "${CYAN}${BOLD}       BenderShot - Preparando Release ${TAG_NAME}        ${RESET}"
echo -e "${CYAN}${BOLD}==========================================================${RESET}"
echo -e "Data/Hora: ${BOLD}${CURRENT_DATE}${RESET}"
echo -e "Nova Versão: ${GREEN}${BOLD}${NEW_VER}${RESET} (Tag: ${GREEN}${BOLD}${TAG_NAME}${RESET})"
echo -e "Mensagem de Commit: ${BOLD}${COMMIT_MSG}${RESET}"
echo ""

# 1. Verifica se a tag já existe no git
if git rev-parse "${TAG_NAME}" >/dev/null 2>&1; then
    echo -e "${RED}${BOLD}Erro: A tag ${TAG_NAME} já existe no git!${RESET}"
    exit 1
fi

# 2. Atualiza os arquivos de versão
echo -e "${YELLOW}==> Atualizando arquivos de versão...${RESET}"

# Cargo.toml
if [ -f "Cargo.toml" ]; then
    sed -i "s/^version = \".*\"/version = \"${NEW_VER}\"/" Cargo.toml
    echo -e "  ✓ Cargo.toml atualizado para ${NEW_VER}"
fi

# packaging/rpm/bendershot.spec (Fedora / RPM)
if [ -f "packaging/rpm/bendershot.spec" ]; then
    sed -i "s/^Version: .*/Version:        ${NEW_VER}/" packaging/rpm/bendershot.spec
    echo -e "  ✓ packaging/rpm/bendershot.spec atualizado para ${NEW_VER}"
fi

# packaging/windows/bendershot.iss (Inno Setup)
if [ -f "packaging/windows/bendershot.iss" ]; then
    sed -i "s/#define MyAppVersion \".*\"/#define MyAppVersion \"${NEW_VER}\"/" packaging/windows/bendershot.iss
    echo -e "  ✓ packaging/windows/bendershot.iss atualizado para ${NEW_VER}"
fi

# Atualiza Cargo.lock sincronizando a versão da crate
echo -e "${YELLOW}==> Sincronizando Cargo.lock...${RESET}"
cargo check --quiet

# 3. Compilação e Empacotamento Local
echo ""
echo -e "${CYAN}${BOLD}==> Executando build e empacotamento local (./build_local.sh)...${RESET}"
if [ -x "./build_local.sh" ]; then
    ./build_local.sh
else
    bash ./build_local.sh
fi

# 4. Git add & commit
echo ""
echo -e "${YELLOW}==> Adicionando alterações ao git...${RESET}"
git add Cargo.toml Cargo.lock packaging/rpm/bendershot.spec packaging/windows/bendershot.iss src/ new-release.sh new-release.ps1

# Adiciona arquivos modificados no repositório caso haja outros pendentes do código
git add -u

echo -e "${YELLOW}==> Criando commit: ${COMMIT_MSG}${RESET}"
git commit -m "${COMMIT_MSG}"

# 5. Criação da Tag Git
echo -e "${YELLOW}==> Criando tag ${TAG_NAME}...${RESET}"
git tag -a "${TAG_NAME}" -m "Release ${TAG_NAME} (${CURRENT_DATE})"

# 6. Push para o repositório remoto
echo ""
echo -e "${CYAN}${BOLD}==> Enviando commit e tag para o GitHub...${RESET}"
CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
git push origin "${CURRENT_BRANCH}"
git push origin "${TAG_NAME}"

echo ""
echo -e "${GREEN}${BOLD}==========================================================${RESET}"
echo -e "${GREEN}${BOLD} ✔ Release ${TAG_NAME} concluída com sucesso!             ${RESET}"
echo -e "${GREEN}${BOLD}==========================================================${RESET}"
echo -e "  • Binários locais gerados em: ${BOLD}packaging/dist/${RESET}"
echo -e "  • Commit e tag ${TAG_NAME} enviados para o GitHub (CI acionado)"
