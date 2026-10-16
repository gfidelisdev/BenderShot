# 🚀 BenderShot

Uma ferramenta moderna de altíssimo desempenho para **captura de tela, gravação de vídeo, anotação vetorial e edição rápida**, inspirada na agilidade do **KDE Spectacle** com design refinado **GNOME Libadwaita**, desenvolvida 100% em **Rust**.

Originalmente concebida para **Linux Wayland**, o BenderShot agora é **multiplataforma**, com suporte nativo de primeira classe tanto para **Linux** (Wayland / X11) quanto para **Windows** (10 / 11, x86_64).

---

## ⚡ Por que Rust?

- **Inicialização Quase Instantânea**: Inicia e exibe o editor em menos de 30ms, sem o overhead de interpretadores Python ou runtimes pesados do Electron/Web.
- **Consumo Mínimo de Recursos**: Menos de 40 MB de RAM em uso contínuo com aceleração de hardware via OpenGL (Glow).
- **Anotações Vetoriais Não-Destrutivas**: Elementos (setas, formas geométricas, curvas bézier, textos, números de passos) são objetos independentes no grafo da cena com histórico canônico de desfazer/refazer (`Ctrl+Z` / `Ctrl+Y`) antes da exportação final.
- **Integração Nativa por Plataforma**:
  - **Linux**: Protocolo XDG Desktop Portal (`org.freedesktop.portal.Screenshot`) silencioso e sem prompts intrusivos, com pré-autorização no `PermissionStore`, gravação via GNOME Screencast e bandeja via D-Bus SNI.
  - **Windows**: Captura nativa multi-monitor e de janelas via GDI/xcap, atalhos globais, integração no tray/inicialização e gravação via FFmpeg (`gdigrab`).

---

## 🛠️ Funcionalidades

### 🎨 Interface & Navegação
- **Design GNOME Libadwaita**:
  - Headerbar moderna com botões pill agrupados, margens confortáveis e botão principal **"📋 Copiar"** destacado no azul oficial do GNOME (`#3584e4`).
  - **Alternador de Tema (Dark / Light)**: Alterne instantaneamente entre os modos Escuro e Claro.
- **Zoom Focal do Canvas & Navegação**:
  - **Zoom com `Ctrl` + Scroll**: Zoom focal centrado no cursor do mouse, variando de 10% a 2000%.
  - **Pan / Movimentação do Viewport**: Arraste livre usando o botão do meio do mouse, <kbd>Espaço</kbd> + botão esquerdo ou ferramenta Pan (<kbd>H</kbd>).
  - **Floating HUD**: Pill flutuante no canto inferior para controle rápido de zoom (`-`, `100%`, `+`, `Ajustar/Fit`).
  - Atalhos rápidos: <kbd>Ctrl+0</kbd> (Ajustar à Janela) e <kbd>Ctrl+1</kbd> (100% Tamanho Real).

### ✏️ Ferramentas de Anotação & Vetores
- **Seleção & Pós-Criação Inteligente**:
  - Seleção, redimensionamento por alças e movimentação de qualquer camada ou objeto.
  - **Seleção Automática Pós-Criação**: Ao concluir o desenho de uma forma (seta, retângulo, elipse, polígono, bézier, texto), a ferramenta de Seleção é ativada automaticamente para ajustes finos sem cliques adicionais.
- **Formas Geométricas Avançadas**:
  - **↗ Seta**, **▢ Retângulo**, **◯ Elipse**, **⬡ Polígonos Regulares** (3 a 12 lados com controle por scroll) e **〰 Curvas Bézier Cúbicas** com alças tangenciais.
  - **Constraint Modifier (`Ctrl`)**: Segure `Ctrl` durante o desenho para travar proporções perfeitas (círculo, quadrado, polígono regular).
- **T Texto Inline no Canvas**: Clique onde desejar para digitar. Renderização TrueType de alta nitidez com halo de contraste para legibilidade em qualquer fundo.
- **① Marcadores de Passos (Step Badges)**: Insere círculos numerados incrementais automaticamente (①, ②, ③...) com um único clique.
- **░ Ofuscação & Proteção de Dados**:
  - Caixas de **Pixelate** e **Blur** aplicadas em tempo real sobre pixels sensíveis.
  - **🖐 Smudge Brush (Pincel de Borrão Livre)**: Esfrega e mistura os pixels de forma fluida (color bleed), ideal para ocultar assinaturas, textos ou detalhes irregulares.
- **🔍 Lupa (Loupe) & Color Picker**: Ampliação de 7x ao redor do cursor com leitura instantânea dos valores **HEX** (`#RRGGBB`) e **RGB** (`rgb(r, g, b)`).
- **📌 Janela Flutuante (Pin to Screen)**: Fixa qualquer recorte em uma janela flutuante Always-on-Top sem bordas para servir de referência durante o trabalho.

### 🎬 Gravação & Edição de Vídeo
- **Captura de Vídeo**: Gravação de tela integrada (Linux Wayland via GNOME Screencast / Windows via FFmpeg).
- **Linha do Tempo (Video Timeline)**:
  - Scrubber com régua de tempo, controle de reprodução/pausa e inspetor temporal.
  - **Spans de Anotação**: Defina exatamente o intervalo de tempo (`start_ms` a `end_ms`) em que cada anotação é exibida no vídeo.
  - **Transição Orgânica Write-On**: Efeito de desenho orgânico progressivo no surgimento da anotação com fade-out na saída.
  - **Exportação de Vídeo**: Renderiza anotações sobre os quadros e exporta para MP4, WebM ou GIF animado.

---

## 📦 Compilação e Instalação

### 🐧 Linux (Fedora, Debian, Ubuntu, Arch, etc.)

O repositório inclui o script [`build_local.sh`](./build_local.sh) para compilação e empacotamento automatizado:

```bash
# 1. Build completo e geração de pacotes (.rpm, .deb, .AppImage, .tar.gz em packaging/dist/)
./build_local.sh

# 2. Build rápido apenas do binário release
./build_local.sh --quick

# 3. Build e instalação local imediata (~/.local/bin/bendershot + ícones e autostart)
./build_local.sh --install

# 4. Executar suíte de testes antes do build
./build_local.sh --test
```

Instalação manual simples via Cargo:
```bash
cargo build --release
sudo cp target/release/bendershot /usr/local/bin/
```

### 🪟 Windows (10 / 11)

```cmd
:: 1. Compilar binário release otimizado
cargo build --release

:: Opcional: Gerar instalador oficial BenderShot-Setup-x64.exe via Inno Setup
iscc packaging\windows\bendershot.iss
```

O instalador para Windows configura automaticamente:
- Atalho global no sistema e inicialização com o Windows (`HKCU\...\Run`).
- Ícone na bandeja do sistema (System Tray).
- Detecção e configuração dinâmica do FFmpeg para gravação de vídeo.

---

## ⌨️ Comandos CLI

Disponíveis tanto no terminal do Linux (`bendershot`) quanto no Prompt/PowerShell do Windows (`bendershot.exe`):

```bash
# Captura de região (padrão interativo)
bendershot region

# Captura de tela inteira
bendershot full

# Captura da janela ativa
bendershot window

# Iniciar ou parar gravação de vídeo
bendershot record

# Iniciar o daemon residente em segundo plano (com ícone na bandeja)
bendershot daemon

# Abrir uma imagem ou vídeo diretamente no editor
bendershot edit /caminho/para/arquivo.png

# Assistente de configuração inicial interativo (Wizard)
bendershot wizard

# Configuração automática silenciosa de atalhos e permissões
bendershot auto-setup
```

---

## 🏗️ Arquitetura, Portabilidade e ADRs

- [Porting BenderShot to Windows (PORTING.md)](./PORTING.md)
- [Glossário do Domínio e Conceitos (CONTEXT.md)](./CONTEXT.md)
- [ADR 0001: Modelo Vetorial Não-Destrutivo](./docs/adr/0001-vector-annotation-model.md)
- [ADR 0002: Escolha de Stack de GUI: egui e winit](./docs/adr/0002-gui-stack-egui-winit.md)
- [ADR 0003: Estratégia de Captura no Wayland](./docs/adr/0003-wayland-capture-portal-strategy.md)
- [ADR 0004: Monólito Modular em Rust](./docs/adr/0004-single-crate-modular-monolith.md)
- [ADR 0005: Captura Silenciosa e Ofuscação no Buffer](./docs/adr/0005-silent-capture-and-direct-canvas-blur.md)
- [ADR 0006: Estilo Libadwaita e Texto Inline no Canvas](./docs/adr/0006-libadwaita-styling-and-inline-text.md)
- [ADR 0007: Pincel Livre de Smudge](./docs/adr/0007-freehand-smudge-brush.md)
- [ADR 0008: Painel de Camadas e Undo Stack Canônico](./docs/adr/0008-layers-panel-and-canonical-undo-stack.md)
- [ADR 0009: Decorações CSD e Linha do Tempo de Vídeo](./docs/adr/0009-csd-decorations-and-video-timeline.md)
- [ADR 0010: Efeito Write-on Orgânico e Geometrias Avançadas](./docs/adr/0010-bendershot-organic-write-on-and-advanced-vector-tools.md)
- [ADR 0011: Zoom Focal no Canvas, Pan e Pós-Seleção](./docs/adr/0011-canvas-zoom-pan-and-post-creation-selection.md)
- [ADR 0012: Pré-Autorização no Portal XDG e Script de Build Local](./docs/adr/0012-portal-permission-pre-authorization-and-build-local-workflow.md)
