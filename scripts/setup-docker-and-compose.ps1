param(
  [string]$ComposeDir = $PSScriptRoot
)

# =========================================================
# Windows: Install Docker Desktop + start + docker compose up
# Single-file PowerShell script
# =========================================================

$ErrorActionPreference = "Stop"

function Write-Info($msg)  { Write-Host "[INFO]  $msg" -ForegroundColor Cyan }
function Write-Warn($msg)  { Write-Host "[WARN]  $msg" -ForegroundColor Yellow }
function Write-Err($msg)   { Write-Host "[ERROR] $msg" -ForegroundColor Red }

function Assert-Admin {
  $currentUser = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
  if (-not $currentUser.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "管理者として実行してください（PowerShell を「管理者として実行」）"
  }
}

function Enable-WSL2 {
  Write-Info "WSL/仮想化機能を有効化します…（未有効の場合のみ）"

  # WSL
  dism.exe /online /enable-feature /featurename:Microsoft-Windows-Subsystem-Linux /all /norestart | Out-Null
  # Virtual Machine Platform
  dism.exe /online /enable-feature /featurename:VirtualMachinePlatform /all /norestart | Out-Null

  # WSL2 default
  try {
    wsl.exe --set-default-version 2 | Out-Null
  } catch {
    Write-Warn "wsl --set-default-version 2 が失敗しました。Windows が古い可能性があります。"
  }

  # Install WSL (Windows 10/11の環境により挙動が異なる)
  try {
    wsl.exe --status | Out-Null
  } catch {
    Write-Warn "WSL が未導入の可能性があるため wsl --install を試します。"
    try {
      wsl.exe --install | Out-Null
    } catch {
      Write-Warn "wsl --install が失敗しました。手動でWSLを導入してください。"
    }
  }
}

function Ensure-Winget {
  if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "winget が見つかりません。Microsoft Store の「App Installer」を入れてから再実行してください。"
  }
}

function Install-DockerDesktop {
  # 既に docker desktop が入っているか軽く確認（完全一致でなくても良い）
  $dockerExe = "${env:ProgramFiles}\Docker\Docker\Docker Desktop.exe"
  if (Test-Path $dockerExe) {
    Write-Info "Docker Desktop は既にインストールされています: $dockerExe"
    return
  }

  Write-Info "Docker Desktop を winget でインストールします…"
  Ensure-Winget

  # winget install
  # -e: exact
  # --accept-package-agreements / --accept-source-agreements: 同意の自動化
  winget install -e --id Docker.DockerDesktop --accept-package-agreements --accept-source-agreements

  if (-not (Test-Path $dockerExe)) {
    Write-Warn "Docker Desktop の実行ファイルが想定パスに見つかりませんでした。インストール結果を確認してください。"
  }
}

function Start-DockerDesktopAndWait {
  $dockerDesktopExe = "${env:ProgramFiles}\Docker\Docker\Docker Desktop.exe"
  if (-not (Test-Path $dockerDesktopExe)) {
    throw "Docker Desktop が見つかりません: $dockerDesktopExe"
  }

  Write-Info "Docker Desktop を起動します…"
  Start-Process -FilePath $dockerDesktopExe | Out-Null

  Write-Info "Docker Engine の起動を待ちます（docker info が通るまでループ）…"
  $maxSeconds = 300
  $start = Get-Date

  while ($true) {
    try {
      # docker CLI が PATH に無い場合があるので、候補も見る
      if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        $candidate = "${env:ProgramFiles}\Docker\Docker\resources\bin\docker.exe"
        if (Test-Path $candidate) {
          $env:PATH = "${env:ProgramFiles}\Docker\Docker\resources\bin;" + $env:PATH
        }
      }

      docker info | Out-Null
      Write-Info "Docker Engine 起動確認OK"
      break
    } catch {
      Start-Sleep -Seconds 3
      $elapsed = (Get-Date) - $start
      if ($elapsed.TotalSeconds -ge $maxSeconds) {
        throw "Docker Engine の起動待ちがタイムアウトしました。Docker Desktop の画面でエラーが出ていないか確認してください。"
      }
    }
  }
}

function Compose-Up($dir) {
  if (-not (Test-Path $dir)) {
    throw "ComposeDir が存在しません: $dir"
  }

  Set-Location $dir
  Write-Info "docker compose を実行します: $dir"

  # compose.yml / docker-compose.yml が無いと困るのでチェック
  $compose1 = Join-Path $dir "compose.yml"
  $compose2 = Join-Path $dir "docker-compose.yml"
  $compose3 = Join-Path $dir "docker-compose.yaml"
  $compose4 = Join-Path $dir "compose.yaml"

  if (-not (Test-Path $compose1) -and -not (Test-Path $compose2) -and -not (Test-Path $compose3) -and -not (Test-Path $compose4)) {
    Write-Warn "compose.yml / docker-compose.yml が見つかりません。ディレクトリを確認してください。"
  }

  # Docker Compose v2 (docker compose)
  docker compose up -d
  Write-Info "docker compose up -d 完了"

  Write-Info "起動状態:"
  docker compose ps
}

try {
  Assert-Admin
  Write-Info "開始: Docker Desktop インストール + compose 起動"

  Enable-WSL2
  Install-DockerDesktop

  Write-Info "必要に応じて再起動が必要です（WSL/仮想化機能を有効化した直後など）。"
  Write-Info "このまま続行して Docker を起動します。"

  Start-DockerDesktopAndWait
  Compose-Up $ComposeDir

  Write-Info "完了"
} catch {
  Write-Err $_.Exception.Message
  exit 1
}
