<#
.SYNOPSIS
    BenderShot - Script de Nova Release e Build para Windows (PowerShell)

.DESCRIPTION
    Automatiza o fluxo completo de release no Windows:
    1. Valida o formato da versão e verifica se a tag já existe no Git.
    2. Atualiza a versão em Cargo.toml, packaging/windows/bendershot.iss e packaging/rpm/bendershot.spec.
    3. Sincroniza o Cargo.lock com cargo check.
    4. Compila o binário em modo release (cargo build --release).
    5. Localiza o Inno Setup Compiler (ISCC.exe) e compila o instalador Windows (BenderShot-Setup-x64.exe).
    6. Cria o commit e a tag anotada no Git.
    7. Envia o commit e a tag para o repositório remoto no GitHub.

.PARAMETER Version
    Número da versão (ex: 0.4.5 ou v0.4.5). Obrigatório.

.PARAMETER Message
    Mensagem descritiva opcional para anexar ao commit e à tag.

.PARAMETER SkipBuild
    Pula a compilação do Rust e a geração do instalador pelo Inno Setup.

.PARAMETER SkipPush
    Não envia o commit e a tag para o GitHub (mantém apenas localmente).

.EXAMPLE
    .\new-release.ps1 0.4.5
    .\new-release.ps1 0.4.5 "Correção na gravação de tela e novo atalho"
    .\new-release.ps1 0.4.5 -SkipPush
#>

[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Version,

    [Parameter(Position = 1)]
    [string]$Message,

    [Parameter()]
    [switch]$SkipBuild,

    [Parameter()]
    [switch]$SkipPush,

    [Parameter()]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Show-Banner {
    Write-Host "==========================================================" -ForegroundColor Cyan
    Write-Host "       BenderShot - Preparando Release (Windows)          " -ForegroundColor Cyan
    Write-Host "==========================================================" -ForegroundColor Cyan
}

function Show-Help {
    Show-Banner
    Write-Host "Uso: .\new-release.ps1 <versao> [mensagem_opcional] [-SkipBuild] [-SkipPush]" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Parâmetros:"
    Write-Host "  <versao>              Número da versão (ex: 0.4.5 ou v0.4.5)"
    Write-Host "  [mensagem_opcional]   Mensagem descritiva para o commit de release"
    Write-Host "  -SkipBuild            Pula a compilação do Rust e a criação do instalador"
    Write-Host "  -SkipPush             Cria o commit e tag localmente sem dar 'git push'"
    Write-Host "  -Help                 Exibe esta mensagem de ajuda"
    Write-Host ""
    Write-Host "Exemplos:"
    Write-Host "  .\new-release.ps1 0.4.5"
    Write-Host "  .\new-release.ps1 0.4.5 `"Correção na captura de tela e suporte a atalhos`""
    Write-Host "  .\new-release.ps1 0.4.5 -SkipPush"
}

if ($Help -or [string]::IsNullOrWhiteSpace($Version)) {
    if ([string]::IsNullOrWhiteSpace($Version) -and -not $Help) {
        Write-Host "Erro: Versão não informada!" -ForegroundColor Red
        Write-Host ""
        Show-Help
        exit 1
    }
    Show-Help
    exit 0
}

$RootDir = $PSScriptRoot
if (-not $RootDir) {
    $RootDir = (Get-Location).Path
}
Set-Location -Path $RootDir

# Normaliza versão (remove 'v' ou 'V' inicial)
$NewVer = $Version.Trim().TrimStart('v', 'V')
$TagName = "v$NewVer"

# Valida formato da versão (ex: 0.4.5 ou 1.0.0.1)
if ($NewVer -notmatch '^\d+(\.\d+)+$') {
    Write-Host "Erro: Formato de versão inválido '$NewVer'. Use números separados por pontos (ex: 0.4.5)." -ForegroundColor Red
    exit 1
}

$CurrentDate = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
if (-not [string]::IsNullOrWhiteSpace($Message)) {
    $CommitMsg = "Release $TagName ($CurrentDate): $Message"
} else {
    $CommitMsg = "Release $TagName ($CurrentDate)"
}

Show-Banner
Write-Host "Data/Hora:       $CurrentDate"
Write-Host "Nova Versão:     " -NoNewline; Write-Host $NewVer -ForegroundColor Green -NoNewline; Write-Host " (Tag: " -NoNewline; Write-Host $TagName -ForegroundColor Green -NoNewline; Write-Host ")"
Write-Host "Mensagem Commit: $CommitMsg"
Write-Host ""

# 1. Verifica pré-requisitos do Git
Write-Host "==> Verificando repositório Git..." -ForegroundColor Yellow
$null = git rev-parse --is-inside-work-tree 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Erro: Diretório atual não é um repositório Git!" -ForegroundColor Red
    exit 1
}

$existingTag = git tag -l $TagName
if ($existingTag) {
    Write-Host "Erro: A tag '$TagName' já existe no Git!" -ForegroundColor Red
    exit 1
}

# 2. Atualiza os arquivos de versão
Write-Host "==> Atualizando arquivos de versão..." -ForegroundColor Yellow

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

# Cargo.toml
$cargoPath = Join-Path $RootDir "Cargo.toml"
if (Test-Path $cargoPath) {
    $cargoText = [System.IO.File]::ReadAllText($cargoPath, [System.Text.Encoding]::UTF8)
    $newCargoText = [regex]::Replace($cargoText, '(?m)^version\s*=\s*"[^"]*"', "version = `"$NewVer`"")
    [System.IO.File]::WriteAllText($cargoPath, $newCargoText, $utf8NoBom)
    Write-Host "  [OK] Cargo.toml atualizado para $NewVer" -ForegroundColor Green
}

# packaging/windows/bendershot.iss (Inno Setup)
$issPath = Join-Path $RootDir "packaging\windows\bendershot.iss"
if (Test-Path $issPath) {
    $issText = [System.IO.File]::ReadAllText($issPath, [System.Text.Encoding]::UTF8)
    $newIssText = [regex]::Replace($issText, '(?m)#define\s+MyAppVersion\s+".*"', "#define MyAppVersion `"$NewVer`"")
    [System.IO.File]::WriteAllText($issPath, $newIssText, $utf8NoBom)
    Write-Host "  [OK] packaging/windows/bendershot.iss atualizado para $NewVer" -ForegroundColor Green
}

# packaging/rpm/bendershot.spec (Fedora / RPM)
$specPath = Join-Path $RootDir "packaging\rpm\bendershot.spec"
if (Test-Path $specPath) {
    $specText = [System.IO.File]::ReadAllText($specPath, [System.Text.Encoding]::UTF8)
    $newSpecText = [regex]::Replace($specText, '(?m)^Version:\s*.*', "Version:        $NewVer")
    [System.IO.File]::WriteAllText($specPath, $newSpecText, $utf8NoBom)
    Write-Host "  [OK] packaging/rpm/bendershot.spec atualizado para $NewVer" -ForegroundColor Green
}

# Sincroniza Cargo.lock
Write-Host "==> Sincronizando Cargo.lock via cargo check..." -ForegroundColor Yellow
$env:CARGO_TARGET_DIR = "$env:TEMP\bendershot-target"
cargo check --quiet
if ($LASTEXITCODE -ne 0) {
    Write-Host "Erro: 'cargo check' falhou ao sincronizar o Cargo.lock." -ForegroundColor Red
    exit 1
}
Write-Host "  [OK] Cargo.lock sincronizado com sucesso." -ForegroundColor Green

# 3. Compilação e Geração do Setup para Windows
if (-not $SkipBuild) {
    Write-Host ""
    Write-Host "==> Compilando BenderShot com Cargo (release)..." -ForegroundColor Cyan
    $env:CARGO_TARGET_DIR = "$env:TEMP\bendershot-target"
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Erro: Falha na compilação do Rust ('cargo build --release')." -ForegroundColor Red
        exit 1
    }

    $builtExe = Join-Path $env:TEMP "bendershot-target\release\bendershot.exe"
    $targetDir = Join-Path $RootDir "target\release"
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    if (Test-Path $builtExe) {
        Copy-Item $builtExe (Join-Path $targetDir "bendershot.exe") -Force
    }

    $exeTarget = Join-Path $targetDir "bendershot.exe"
    if (-not (Test-Path $exeTarget)) {
        Write-Host "Erro: Executável não encontrado em $exeTarget" -ForegroundColor Red
        exit 1
    }
    $exeSize = "{0:N2} MB" -f ((Get-Item $exeTarget).Length / 1MB)
    Write-Host "  [OK] Executável compilado: target\release\bendershot.exe ($exeSize)" -ForegroundColor Green

    # Localiza o Inno Setup Compiler (ISCC.exe)
    Write-Host "==> Localizando compilador do Inno Setup (ISCC.exe)..." -ForegroundColor Yellow
    $cmdIscc = Get-Command iscc.exe -ErrorAction SilentlyContinue
    $isccFromPath = if ($cmdIscc) { $cmdIscc.Source } else { $null }

    $isccCandidates = @(
        $isccFromPath,
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
    )

    $isccPath = $null
    foreach ($cand in $isccCandidates) {
        if ($cand -and (Test-Path $cand)) {
            $isccPath = $cand
            break
        }
    }

    if (-not $isccPath) {
        Write-Host "Erro: Compilador do Inno Setup (ISCC.exe) não foi encontrado!" -ForegroundColor Red
        Write-Host "Por favor, instale o Inno Setup via 'winget install JR.InnoSetup' ou baixe em https://jrsoftware.org/isdl.php" -ForegroundColor Yellow
        exit 1
    }

    Write-Host "  [OK] Compilador Inno Setup encontrado: $isccPath" -ForegroundColor Green

    # Garante que o diretório packaging/dist existe
    $distDir = Join-Path $RootDir "packaging\dist"
    if (-not (Test-Path $distDir)) {
        New-Item -ItemType Directory -Path $distDir -Force | Out-Null
    }

    Write-Host "==> Gerando instalador do Windows (BenderShot-Setup-x64.exe)..." -ForegroundColor Cyan
    & $isccPath "packaging\windows\bendershot.iss"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Erro: Falha ao compilar o instalador com o Inno Setup." -ForegroundColor Red
        exit 1
    }

    $setupFile = Join-Path $distDir "BenderShot-Setup-x64.exe"
    if (-not (Test-Path $setupFile)) {
        Write-Host "Erro: Arquivo do instalador $setupFile não foi gerado!" -ForegroundColor Red
        exit 1
    }

    $setupSize = "{0:N2} MB" -f ((Get-Item $setupFile).Length / 1MB)
    Write-Host "  [OK] Instalador Windows gerado com sucesso: packaging\dist\BenderShot-Setup-x64.exe ($setupSize)" -ForegroundColor Green
} else {
    Write-Host "==> Pulei a etapa de compilação (-SkipBuild ativado)." -ForegroundColor DarkYellow
}

# 4. Git add & commit
Write-Host ""
Write-Host "==> Adicionando arquivos modificados ao Git..." -ForegroundColor Yellow
git add Cargo.toml Cargo.lock packaging/windows/bendershot.iss packaging/rpm/bendershot.spec new-release.ps1 new-release.sh src/
git add -u

Write-Host "==> Criando commit: $CommitMsg" -ForegroundColor Yellow
git commit -m "$CommitMsg"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Aviso: Nenhum arquivo modificado para commit ou falha ao executar git commit." -ForegroundColor DarkYellow
}

# 5. Criação da Tag Git
Write-Host "==> Criando tag $TagName..." -ForegroundColor Yellow
git tag -a "$TagName" -m "Release $TagName ($CurrentDate)"
if ($LASTEXITCODE -ne 0) {
    Write-Host "Erro: Falha ao criar a tag $TagName." -ForegroundColor Red
    exit 1
}
Write-Host "  [OK] Tag $TagName criada." -ForegroundColor Green

# 6. Push para o repositório remoto
if (-not $SkipPush) {
    Write-Host ""
    Write-Host "==> Enviando commit e tag para o GitHub..." -ForegroundColor Cyan
    $currentBranch = (git rev-parse --abbrev-ref HEAD).Trim()
    
    Write-Host "  • Enviando branch $currentBranch..." -ForegroundColor Gray
    git push origin "$currentBranch"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Erro: Falha ao enviar a branch $currentBranch para o remoto." -ForegroundColor Red
        exit 1
    }

    Write-Host "  • Enviando tag $TagName..." -ForegroundColor Gray
    git push origin "$TagName"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Erro: Falha ao enviar a tag $TagName para o remoto." -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "==> Pulei o envio para o GitHub (-SkipPush ativado)." -ForegroundColor DarkYellow
    Write-Host "    Para enviar manualmente mais tarde execute:" -ForegroundColor DarkYellow
    $currentBranch = (git rev-parse --abbrev-ref HEAD).Trim()
    Write-Host "      git push origin $currentBranch" -ForegroundColor White
    Write-Host "      git push origin $TagName" -ForegroundColor White
}

Write-Host ""
Write-Host "==========================================================" -ForegroundColor Green
Write-Host " [OK] Release $TagName concluída com sucesso!             " -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Green
if (-not $SkipBuild) {
    Write-Host "  • Instalador Windows gerado em: packaging\dist\BenderShot-Setup-x64.exe" -ForegroundColor White
}
if (-not $SkipPush) {
    Write-Host "  • Commit e tag $TagName enviados para o GitHub (CI/Release acionados)" -ForegroundColor White
}
