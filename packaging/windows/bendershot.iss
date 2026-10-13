#define MyAppName "BenderShot"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "BenderShot Team"
#define MyAppURL "https://github.com/gfidelis/BenderShot"
#define MyAppExeName "bendershot.exe"

[Setup]
AppId={{8B1D4F3A-7B42-4D9A-974F-4DE0B547C351}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=BenderShot-Setup-x64
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
brazilianportuguese.FFmpegTaskDesc=Baixar e configurar o FFmpeg automaticamente (Necessário para gravação e edição de vídeo)
english.FFmpegTaskDesc=Download and configure FFmpeg automatically (Required for screen recording and video editing)
brazilianportuguese.FFmpegGroupDesc=Componentes Adicionais:
english.FFmpegGroupDesc=Additional Components:
brazilianportuguese.FFmpegStatusMsg=Configurando FFmpeg para gravação de vídeo...
english.FFmpegStatusMsg=Setting up FFmpeg for screen recording...
brazilianportuguese.StartupTaskDesc=Iniciar o BenderShot em segundo plano automaticamente ao ligar o Windows (Recomendado)
english.StartupTaskDesc=Start BenderShot in background automatically on Windows logon (Recommended)
brazilianportuguese.StartupGroupDesc=Inicialização do Serviço:
english.StartupGroupDesc=Service Startup:

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startupservice"; Description: "{cm:StartupTaskDesc}"; GroupDescription: "{cm:StartupGroupDesc}"
Name: "installffmpeg"; Description: "{cm:FFmpegTaskDesc}"; GroupDescription: "{cm:FFmpegGroupDesc}"; Check: NeedsFFmpeg

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\..\CONTEXT.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{#MyAppName} (Segundo Plano)"; Filename: "{app}\{#MyAppExeName}"; Parameters: "daemon"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Registra o daemon para iniciar no logon do usuário do Windows
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "BenderShot"; ValueData: """{app}\{#MyAppExeName}"" daemon"; Flags: uninsdeletevalue; Tasks: startupservice

[Run]
; Baixa e configura o FFmpeg se a opção foi selecionada
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -NoProfile -Command ""if (Get-Command winget -ErrorAction SilentlyContinue) { Write-Host 'Instalando FFmpeg via winget...'; winget install --id Gyan.FFmpeg -e --accept-package-agreements --accept-source-agreements --silent } else { Write-Host 'Baixando FFmpeg...'; try { [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; $url = 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'; $tmp = Join-Path $env:TEMP 'ffmpeg.zip'; Invoke-WebRequest -Uri $url -OutFile $tmp; Expand-Archive -Path $tmp -DestinationPath (Join-Path $env:TEMP 'ffmpeg_unpacked') -Force; Get-ChildItem -Path (Join-Path $env:TEMP 'ffmpeg_unpacked') -Recurse -Filter 'ffmpeg.exe' | ForEach-Object { Copy-Item $_.FullName '{app}\ffmpeg.exe' -Force }; Get-ChildItem -Path (Join-Path $env:TEMP 'ffmpeg_unpacked') -Recurse -Filter 'ffprobe.exe' | ForEach-Object { Copy-Item $_.FullName '{app}\ffprobe.exe' -Force }; Remove-Item -Recurse -Force (Join-Path $env:TEMP 'ffmpeg_unpacked') -ErrorAction SilentlyContinue; Remove-Item -Force $tmp -ErrorAction SilentlyContinue } catch { Write-Warning 'Falha ao baixar FFmpeg.' } }"""; StatusMsg: "{cm:FFmpegStatusMsg}"; Tasks: installffmpeg; Flags: runhidden

; Inicia o daemon em segundo plano imediatamente após concluir a instalação
Filename: "{app}\{#MyAppExeName}"; Parameters: "daemon"; Description: "Iniciar o serviço BenderShot em segundo plano agora"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Finaliza o processo do daemon antes de desinstalar
Filename: "taskkill.exe"; Parameters: "/F /IM {#MyAppExeName}"; Flags: runhidden; RunOnceId: "StopBenderShot"

[Code]
var
  FFmpegAlreadyInstalled: Boolean;

function NeedsFFmpeg(): Boolean;
var
  ResultCode: Integer;
begin
  // 1. Verifica se o comando ffmpeg já existe no sistema no %PATH% via 'where ffmpeg'
  if Exec('cmd.exe', '/c where ffmpeg', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
  begin
    FFmpegAlreadyInstalled := (ResultCode = 0);
  end
  else
  begin
    FFmpegAlreadyInstalled := False;
  end;

  // 2. Também verifica se o executável já existe na pasta de instalação do BenderShot
  if FileExists(ExpandConstant('{app}\ffmpeg.exe')) then
  begin
    FFmpegAlreadyInstalled := True;
  end;

  // Retorna True se o FFmpeg NÃO estiver instalado, exibindo a opção de instalação automática
  Result := not FFmpegAlreadyInstalled;
end;
