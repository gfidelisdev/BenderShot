param (
    [string]$AppDir = $PSScriptRoot
)

Write-Host "Verificando instalacao do FFmpeg..."

if (Get-Command winget -ErrorAction SilentlyContinue) {
    Write-Host "Instalando FFmpeg via winget..."
    winget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements --silent
} else {
    Write-Host "Baixando FFmpeg portatil..."
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        $url = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'
        $tmpZip = Join-Path $env:TEMP 'bendershot_ffmpeg.zip'
        $unpacked = Join-Path $env:TEMP 'bendershot_ffmpeg_unpacked'

        Invoke-WebRequest -Uri $url -OutFile $tmpZip -UseBasicParsing
        Expand-Archive -Path $tmpZip -DestinationPath $unpacked -Force

        if (-not (Test-Path $AppDir)) {
            New-Item -ItemType Directory -Path $AppDir -Force | Out-Null
        }

        Get-ChildItem -Path $unpacked -Recurse -Filter 'ffmpeg.exe' | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $AppDir 'ffmpeg.exe') -Force
        }
        Get-ChildItem -Path $unpacked -Recurse -Filter 'ffprobe.exe' | ForEach-Object {
            Copy-Item $_.FullName (Join-Path $AppDir 'ffprobe.exe') -Force
        }

        Remove-Item -Recurse -Force $unpacked -ErrorAction SilentlyContinue
        Remove-Item -Force $tmpZip -ErrorAction SilentlyContinue
        Write-Host "FFmpeg configurado com sucesso em $AppDir"
    } catch {
        Write-Warning "Nao foi possivel baixar o FFmpeg automaticamente: $_"
    }
}
