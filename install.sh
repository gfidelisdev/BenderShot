#!/usr/bin/env bash
# ==============================================================================
# enhanced-shot install.sh
# Universal Multi-Distro Interactive Installer & Shortcut Configurator
# ==============================================================================
set -euo pipefail

# ANSI Colors
BOLD="\033[1m"
GREEN="\033[32m"
CYAN="\033[36m"
YELLOW="\033[33m"
RED="\033[31m"
RESET="\033[0m"

APP_NAME="enhanced-shot"
APP_DISPLAY_NAME="Enhanced Shot"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Determine Binary & Asset paths (works whether run from repo or extracted tarball)
if [ -f "${SCRIPT_DIR}/${APP_NAME}" ]; then
    SRC_BIN="${SCRIPT_DIR}/${APP_NAME}"
    SRC_ASSETS="${SCRIPT_DIR}/assets"
elif [ -f "${SCRIPT_DIR}/target/release/${APP_NAME}" ]; then
    SRC_BIN="${SCRIPT_DIR}/target/release/${APP_NAME}"
    SRC_ASSETS="${SCRIPT_DIR}/packaging/assets"
elif command -v "${APP_NAME}" &>/dev/null; then
    SRC_BIN="$(command -v "${APP_NAME}")"
    SRC_ASSETS="${SCRIPT_DIR}/packaging/assets"
else
    echo -e "${RED}Erro: Binário '${APP_NAME}' não encontrado!${RESET}"
    echo "Execute a compilação primeiro ('cargo build --release') ou use o tarball de distribuição."
    exit 1
fi

print_header() {
    echo -e "${CYAN}${BOLD}"
    echo "=========================================================="
    echo "         ${APP_DISPLAY_NAME} - Instalador Multi-Distro"
    echo "      Captura e Anotação Profissional de Tela Wayland"
    echo "=========================================================="
    echo -e "${RESET}"
}

# Check install target locations
setup_paths() {
    if [ "${EUID}" -eq 0 ]; then
        INSTALL_MODE="system"
        BIN_DIR="/usr/local/bin"
        DESKTOP_DIR="/usr/local/share/applications"
        ICON_DIR="/usr/local/share/icons/hicolor/scalable/apps"
        SERVICE_DIR="/etc/systemd/user"
    else
        INSTALL_MODE="user"
        BIN_DIR="${HOME}/.local/bin"
        DESKTOP_DIR="${HOME}/.local/share/applications"
        ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
        SERVICE_DIR="${HOME}/.config/systemd/user"
    fi
}

install_files() {
    echo -e "${BOLD}==> Instalando arquivos (${INSTALL_MODE})...${RESET}"
    mkdir -p "${BIN_DIR}" "${DESKTOP_DIR}" "${ICON_DIR}" "${SERVICE_DIR}"

    # 1. Binary
    if [ "${SRC_BIN}" != "${BIN_DIR}/${APP_NAME}" ]; then
        cp -f "${SRC_BIN}" "${BIN_DIR}/${APP_NAME}"
        chmod 755 "${BIN_DIR}/${APP_NAME}"
        echo -e "  [✔] Binário: ${GREEN}${BIN_DIR}/${APP_NAME}${RESET}"
    fi

    # 2. Desktop file
    if [ -f "${SRC_ASSETS}/enhanced-shot.desktop" ]; then
        cp -f "${SRC_ASSETS}/enhanced-shot.desktop" "${DESKTOP_DIR}/${APP_NAME}.desktop"
        chmod 644 "${DESKTOP_DIR}/${APP_NAME}.desktop"
        echo -e "  [✔] Lançador: ${GREEN}${DESKTOP_DIR}/${APP_NAME}.desktop${RESET}"
    fi

    # 3. Icon
    if [ -f "${SRC_ASSETS}/enhanced-shot.svg" ]; then
        cp -f "${SRC_ASSETS}/enhanced-shot.svg" "${ICON_DIR}/${APP_NAME}.svg"
        chmod 644 "${ICON_DIR}/${APP_NAME}.svg"
        echo -e "  [✔] Ícone: ${GREEN}${ICON_DIR}/${APP_NAME}.svg${RESET}"
    fi

    # 4. Systemd unit
    if [ -f "${SRC_ASSETS}/enhanced-shot.service" ]; then
        cp -f "${SRC_ASSETS}/enhanced-shot.service" "${SERVICE_DIR}/${APP_NAME}.service"
        chmod 644 "${SERVICE_DIR}/${APP_NAME}.service"
        echo -e "  [✔] Serviço Daemon: ${GREEN}${SERVICE_DIR}/${APP_NAME}.service${RESET}"
    fi

    # Update desktop and icon database if available
    command -v update-desktop-database &>/dev/null && update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
    command -v gtk-update-icon-cache &>/dev/null && gtk-update-icon-cache -f -t "$(dirname "$(dirname "${ICON_DIR}")")" 2>/dev/null || true

    # PATH warning if user mode and ~/.local/bin not in PATH
    if [ "${INSTALL_MODE}" = "user" ]; then
        if [[ ":$PATH:" != *":${HOME}/.local/bin:"* ]]; then
            echo -e "${YELLOW}Aviso: '${HOME}/.local/bin' não está no seu PATH. Adicione ao seu ~/.bashrc ou ~/.zshrc:${RESET}"
            echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
        fi
    fi
}

configure_gnome_shortcut() {
    local keybinding="$1"
    local binding_name="$2"
    local command_to_run="${BIN_DIR}/${APP_NAME} region"

    if ! command -v gsettings &>/dev/null; then
        echo -e "${YELLOW}gsettings não disponível. Atalhos devem ser configurados manualmente.${RESET}"
        return
    fi

    # If keybinding is 'Print', we disable the GNOME native screenshot UI first
    if [ "${keybinding}" = "Print" ]; then
        echo -e "  [i] Desvinculando PrintScreen nativo do GNOME Shell..."
        gsettings set org.gnome.shell.keybindings show-screenshot-ui "[]" 2>/dev/null || true
    fi

    echo -e "  [✔] Configurando atalho global GNOME: ${BOLD}${keybinding}${RESET} -> '${command_to_run}'"

    # Locate or create a custom shortcut slot
    local custom_path="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/"
    local current_bindings
    current_bindings="$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "@as []")"

    if [[ "${current_bindings}" != *"${custom_path}"* ]]; then
        if [ "${current_bindings}" = "@as []" ] || [ "${current_bindings}" = "[]" ]; then
            gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['${custom_path}']"
        else
            # Append to array
            local new_bindings
            new_bindings="${current_bindings%]*}, '${custom_path}']"
            gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "${new_bindings}"
        fi
    fi

    # Write keybinding schema
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} name "${binding_name}"
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} command "${command_to_run}"
    gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} binding "${keybinding}"

    echo -e "  ${GREEN}Atalho registrado com sucesso no GNOME!${RESET}"
}

configure_daemon() {
    echo -e "${BOLD}==> Ativando serviço de segundo plano (Daemon)...${RESET}"
    if command -v systemctl &>/dev/null; then
        systemctl --user daemon-reload || true
        systemctl --user enable --now "${APP_NAME}.service" || true
        echo -e "  ${GREEN}Daemon ativado e em execução no painel superior!${RESET}"
    else
        echo -e "${YELLOW}systemctl não encontrado. Inicie manualmente executando: '${APP_NAME} daemon &'${RESET}"
    fi
}

uninstall() {
    setup_paths
    echo -e "${RED}${BOLD}==> Desinstalando ${APP_DISPLAY_NAME}...${RESET}"

    # Stop daemon
    if command -v systemctl &>/dev/null; then
        systemctl --user disable --now "${APP_NAME}.service" 2>/dev/null || true
    fi

    # Restore GNOME Print key if customized
    if command -v gsettings &>/dev/null; then
        local custom_path="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/enhanced-shot/"
        local current_bindings
        current_bindings="$(gsettings get org.gnome.settings-daemon.plugins.media-keys custom-keybindings 2>/dev/null || echo "[]")"
        if [[ "${current_bindings}" == *"${custom_path}"* ]]; then
            gsettings reset org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} binding 2>/dev/null || true
            gsettings reset org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} command 2>/dev/null || true
            gsettings reset org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:${custom_path} name 2>/dev/null || true
            # Clean array
            local cleaned
            cleaned="$(echo "${current_bindings}" | sed "s|'${custom_path}', ||g" | sed "s|, '${custom_path}'||g" | sed "s|'${custom_path}'||g")"
            gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "${cleaned}" 2>/dev/null || true
        fi
        # Restore native GNOME PrintScreen
        gsettings set org.gnome.shell.keybindings show-screenshot-ui "['Print']" 2>/dev/null || true
    fi

    rm -f "${BIN_DIR}/${APP_NAME}"
    rm -f "${DESKTOP_DIR}/${APP_NAME}.desktop"
    rm -f "${ICON_DIR}/${APP_NAME}.svg"
    rm -f "${SERVICE_DIR}/${APP_NAME}.service"

    command -v update-desktop-database &>/dev/null && update-desktop-database "${DESKTOP_DIR}" 2>/dev/null || true
    command -v gtk-update-icon-cache &>/dev/null && gtk-update-icon-cache -f -t "$(dirname "$(dirname "${ICON_DIR}")")" 2>/dev/null || true

    echo -e "${GREEN}${APP_DISPLAY_NAME} removido com sucesso.${RESET}"
    exit 0
}

# --- CLI Arguments Handling ---
if [ "${1:-}" = "--uninstall" ] || [ "${1:-}" = "-u" ]; then
    uninstall
fi

print_header
setup_paths

echo "Destino da instalação: ${GREEN}${BIN_DIR}${RESET} (${INSTALL_MODE})"
echo ""
echo -e "${BOLD}Selecione o modo de operação desejado:${RESET}"
echo "  1) [Recomendado] Substituir a tecla PrintScreen padrão pelo Enhanced Shot"
echo "  2) Configurar um atalho de teclado personalizado (ex: <Ctrl><Shift>Print ou <Ctrl><Alt>s)"
echo "  3) Executar como Daemon no painel da bandeja (ícone na barra superior do GNOME/KDE)"
echo "  4) Apenas instalar (lançamento pelo menu de aplicativos ou linha de comando)"
echo "  5) Cancelar instalação"
echo ""

read -rp "Opção [1-5] (padrão: 1): " CHOICE
CHOICE="${CHOICE:-1}"

case "${CHOICE}" in
    1)
        install_files
        configure_gnome_shortcut "Print" "Enhanced Shot Captura"
        ;;
    2)
        echo ""
        echo "Exemplos de formato para o GNOME:"
        echo "  - <Ctrl><Shift>Print"
        echo "  - <Primary><Alt>s"
        echo "  - <Super>s"
        read -rp "Digite a combinação de teclas desejada: " USER_KEY
        if [ -n "${USER_KEY}" ]; then
            install_files
            configure_gnome_shortcut "${USER_KEY}" "Enhanced Shot Captura"
        else
            echo "Nenhum atalho digitado. Instalando apenas lançadores."
            install_files
        fi
        ;;
    3)
        install_files
        configure_daemon
        ;;
    4)
        install_files
        ;;
    5)
        echo "Instalação cancelada pelo usuário."
        exit 0
        ;;
    *)
        echo -e "${RED}Opção inválida!${RESET}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}${BOLD}✓ Instalação concluída com sucesso!${RESET}"
echo "Comandos disponíveis:"
echo "  - ${CYAN}${APP_NAME} region${RESET}   : Selecionar e anotar região"
echo "  - ${CYAN}${APP_NAME} full${RESET}     : Capturar e anotar tela inteira"
echo "  - ${CYAN}${APP_NAME} window${RESET}   : Capturar janela ativa"
echo "  - ${CYAN}${APP_NAME} daemon${RESET}   : Iniciar daemon de bandeja"
echo "  - ${CYAN}${APP_NAME} /caminho/foto.png${RESET} : Abrir imagem para edição"
echo ""
echo "Para desinstalar no futuro, execute: ${BOLD}./install.sh --uninstall${RESET}"
