# 🚀 Enhanced Shot

Uma ferramenta de captura de tela de altíssimo desempenho, anotação vetorial e fluxo de trabalho em estilo **KDE Spectacle** com design nativo **GNOME Libadwaita** para **Linux Wayland** (Fedora / GNOME), desenvolvida 100% em **Rust**.

---

## ⚡ Por que Rust?

- **Inicialização Quase Instantânea**: Inicia e exibe o editor em menos de 30ms, sem o overhead de interpretadores Python ou runtimes pesados do Node.js/Electron.
- **Consumo Mínimo de Recursos**: Menos de 40 MB de RAM em uso contínuo.
- **Anotações Vetoriais Não-Destrutivas**: Elementos (setas, textos, números de passos) são objetos independentes no grafo da cena que podem ser desfeitos (`Ctrl+Z`) e ajustados antes da exportação final.
- **Wayland Nativo e Silencioso**: Utiliza o protocolo XDG Desktop Portal (`org.freedesktop.portal.Screenshot`) com captura instantânea sem invocar o seletor nativo do GNOME.

---

## 🛠️ Funcionalidades

- **Design GNOME Libadwaita**:
  - Headerbar moderna com botões em estilo "pill" agrupados e margens confortáveis.
  - Botão principal **"📋 Copiar"** destacado no azul oficial do GNOME (`#3584e4`).
  - **Alternador de Tema (Dark / Light)**: Alterne instantaneamente entre os modos Escuro e Claro.
- **Ferramentas de Anotação & Edição**:
  - **Seleção**, **↗ Seta**, **▢ Retângulo**, **◯ Elipse**, **⯏ Recorte**.
  - **Constraint Modifier (`Ctrl`)**: Segure `Ctrl` enquanto arrasta para travar a **Elipse em círculo perfeito** ou o **Retângulo em quadrado perfeito**.
  - **T Texto Inline no Canvas**: Clique diretamente no ponto da tela desejado para abrir o editor inline. Renderização TrueType com halo de contraste alto para ser legível em qualquer fundo.
  - **① Marcadores de Passos (Step Badges)**: Insere círculos numerados incrementais automaticamente (①, ②, ③...) com um **único clique**.
  - **░ Obfuscation (Blur & Pixelate)**: Ofuscação de caixas em tempo real diretamente sobre os pixels com suporte a desfazer (`Ctrl+Z`).
  - **🖐 Smudge Brush (Pincel de Borrão Livre)**: Pincel de arraste livre que esfrega e mistura os pixels de forma fluida (color bleed), ideal para mascarar assinaturas, linhas soltas ou detalhes irregulares.
  - **🔍 Lupa (Loupe) & Color Picker**: Ampliação em tempo real de 7x ao redor do cursor com leitura instantânea dos valores **HEX** (`#RRGGBB`) e **RGB** (`rgb(r, g, b)`).
  - **📌 Janela Flutuante (Pin to Desktop)**: Fixa o screenshot em uma janelinha flutuante Always-on-Top sem bordas para servir de referência visual.
- **Exportação & Clipboard**:
  - `Ctrl+C` ou botão "Copiar": Envia a imagem renderizada (PNG) direto para a área de transferência do Wayland e fecha a janela.
  - **Auto-Save**: Salva automaticamente uma cópia em `~/Pictures/Screenshots/screenshot_YYYY-MM-DD_HH-mm-ss.png`.
- **Top Panel Indicator (GNOME)**:
  - Ícone na barra superior via `StatusNotifierItem` (D-Bus SNI) com menu de captura rápida.

---

## 📦 Compilação e Instalação

```bash
# Compilar em modo release com otimizações máximas
cargo build --release

# Opcional: Instalar no sistema
sudo cp target/release/enhanced-shot /usr/local/bin/
```

---

## ⌨️ Comandos CLI

```bash
# Captura direta (padrão)
enhanced-shot

# Captura de tela inteira
enhanced-shot full

# Captura da janela ativa
enhanced-shot window

# Iniciar o daemon com ícone no painel superior do GNOME
enhanced-shot daemon

# Abrir uma imagem existente diretamente no editor
enhanced-shot edit /caminho/para/imagem.png
```

---

## 🏗️ Documentação e ADRs

- [Glossário do Domínio (CONTEXT.md)](./CONTEXT.md)
- [ADR 0001: Modelo Vetorial Não-Destrutivo](./docs/adr/0001-vector-annotation-model.md)
- [ADR 0002: Escolha de Stack de GUI: egui e winit](./docs/adr/0002-gui-stack-egui-winit.md)
- [ADR 0003: Estratégia de Captura no Wayland](./docs/adr/0003-wayland-capture-portal-strategy.md)
- [ADR 0004: Monólito Modular em Rust](./docs/adr/0004-single-crate-modular-monolith.md)
- [ADR 0005: Captura Silenciosa e Ofuscação no Buffer](./docs/adr/0005-silent-capture-and-direct-canvas-blur.md)
- [ADR 0006: Estilo Libadwaita e Texto Inline no Canvas](./docs/adr/0006-libadwaita-styling-and-inline-text.md)
- [ADR 0007: Pincel Livre de Smudge](./docs/adr/0007-freehand-smudge-brush.md)
