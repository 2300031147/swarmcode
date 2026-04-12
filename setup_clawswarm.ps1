# SwarmCode Installer
# Supports: GUI, TUI, Both, or Server-only modes
# Platforms: Windows (.exe via Tauri NSIS), Linux (.AppImage / .deb)

param(
    [ValidateSet("gui", "tui", "both", "server", "")]
    [string]$Mode = "",
    [switch]$Silent,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$CLAWSWARM_VERSION = "0.1.0"
$INSTALL_DIR = "$env:LOCALAPPDATA\SwarmCode"
$BIN_DIR     = "$INSTALL_DIR\bin"
$CONFIG_DIR  = "$env:USERPROFILE\.swarmcode"

function Write-Banner {
    Write-Host ""
    Write-Host "  ╔══════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "  ║        SwarmCode $CLAWSWARM_VERSION Installer         ║" -ForegroundColor Cyan
    Write-Host "  ║   AI Engineering Assistant — by SwarmCode  ║" -ForegroundColor Cyan
    Write-Host "  ╚══════════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
}

function Write-Step([string]$msg) { Write-Host "  > $msg" -ForegroundColor DarkCyan }
function Write-OK([string]$msg)   { Write-Host "  + $msg" -ForegroundColor Green }
function Write-Warn([string]$msg) { Write-Host "  ! $msg" -ForegroundColor Yellow }
function Write-Err([string]$msg)  { Write-Host "  x $msg" -ForegroundColor Red }

function Get-InstallMode {
    if ($Mode -ne "") { return $Mode }
    Write-Host ""
    Write-Host "  What would you like to install?" -ForegroundColor White
    Write-Host ""
    Write-Host "    [1] GUI  (Recommended) -- VS Code-like desktop app with full AI features" -ForegroundColor White
    Write-Host "    [2] TUI  -- Terminal UI, lightweight, works over SSH"                     -ForegroundColor White
    Write-Host "    [3] Both -- Install GUI + TUI"                                            -ForegroundColor White
    Write-Host "    [4] Server only -- Headless mode (no UI, API server)"                    -ForegroundColor White
    Write-Host ""
    $choice = Read-Host "  Enter choice [1-4]"
    switch ($choice.Trim()) {
        "1" { return "gui"    }
        "2" { return "tui"    }
        "3" { return "both"   }
        "4" { return "server" }
        default {
            Write-Warn "Invalid choice, defaulting to GUI."
            return "gui"
        }
    }
}

function Ensure-Dir([string]$path) {
    if (-not (Test-Path $path)) {
        New-Item -ItemType Directory -Force -Path $path | Out-Null
    }
}

function Add-ToPath([string]$dir) {
    $current = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($current -notlike "*$dir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$current;$dir", "User")
        Write-OK "Added $dir to user PATH"
    }
}

function Install-GUI {
    Write-Step "Installing SwarmCode GUI..."
    $guiExe = ".\swarm-gui\src-tauri\target\release\bundle\nsis\SwarmCode_${CLAWSWARM_VERSION}_x64-setup.exe"
    if (Test-Path $guiExe) {
        Write-Step "Running NSIS installer..."
        Start-Process -FilePath $guiExe -ArgumentList "/S" -Wait
        Write-OK "ClawSwarm GUI installed via NSIS."
    } else {
        $rawExe = ".\target\release\swarm-gui.exe"
        if (Test-Path $rawExe) {
            Ensure-Dir $BIN_DIR
            Copy-Item $rawExe "$BIN_DIR\swarmcode.exe" -Force
            Add-ToPath $BIN_DIR
            Write-OK "GUI binary installed to $BIN_DIR\swarmcode.exe"
            # Start Menu shortcut
            $shell  = New-Object -ComObject WScript.Shell
            $lnkDir = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\SwarmCode"
            Ensure-Dir $lnkDir
            $shortcut = $shell.CreateShortcut("$lnkDir\SwarmCode.lnk")
            $shortcut.TargetPath = "$BIN_DIR\swarmcode.exe"
            $shortcut.WorkingDirectory = $BIN_DIR
            $shortcut.Description = "SwarmCode AI Engineering Assistant"
            $shortcut.Save()
            Write-OK "Start Menu shortcut created."
        } else {
            Write-Warn "GUI binary not found. Run 'npx tauri build' from swarm-gui/ first."
        }
    }
}

function Install-TUI {
    Write-Step "Installing ClawSwarm TUI (swarm-master)..."
    $tuiExe = ".\target\release\swarm-master.exe"
    if (Test-Path $tuiExe) {
        Ensure-Dir $BIN_DIR
        Copy-Item $tuiExe "$BIN_DIR\claw.exe" -Force
        Add-ToPath $BIN_DIR
        Write-OK "TUI installed as: claw"
    } else {
        Write-Warn "TUI binary not found. Run 'cargo build --release -p swarm-master' first."
    }
}

function Install-Server {
    Write-Step "Installing ClawSwarm headless server..."
    $serverExe = ".\target\release\swarm-master.exe"
    if (Test-Path $serverExe) {
        Ensure-Dir $BIN_DIR
        Copy-Item $serverExe "$BIN_DIR\clawswarm-server.exe" -Force
        Add-ToPath $BIN_DIR
        Write-OK "Server installed as: clawswarm-server"
    } else {
        Write-Warn "Server binary not found. Build the project first."
    }
}

function Setup-Config {
    Write-Step "Setting up configuration directory..."
    Ensure-Dir $CONFIG_DIR
    $tplPath = "$CONFIG_DIR\providers.toml"
    if (-not (Test-Path $tplPath)) {
        Set-Content -Path $tplPath -Encoding UTF8 -Value @"
# SwarmCode Providers Configuration
# Add custom OpenAI-compatible providers below.
# Restart SwarmCode after editing.

# [[providers]]
# name     = "My Local Server"
# base_url = "http://localhost:8080/v1"
# models   = ["my-model"]
"@
        Write-OK "Created providers.toml at $tplPath"
    } else {
        Write-OK "providers.toml already exists."
    }
}

function Show-PostInstall([string]$mode) {
    Write-Host ""
    Write-Host "  ClawSwarm installed!" -ForegroundColor Green
    switch ($mode) {
        "gui"    { Write-Host "  Launch GUI : clawswarm  (or Start Menu -> ClawSwarm)" -ForegroundColor White }
        "tui"    { Write-Host "  Launch TUI : claw" -ForegroundColor White }
        "both"   { Write-Host "  GUI: clawswarm  |  TUI: claw" -ForegroundColor White }
        "server" { Write-Host "  Start: clawswarm-server --headless" -ForegroundColor White }
    }
    Write-Host ""
    Write-Host "  Config : $CONFIG_DIR\providers.toml" -ForegroundColor DarkGray
    Write-Host ""
    Write-Host "  Hosted API keys (optional):" -ForegroundColor DarkGray
    Write-Host "    GROQ_API_KEY / MISTRAL_API_KEY / TOGETHER_API_KEY / OPENAI_API_KEY" -ForegroundColor DarkGray
    Write-Host "  Local: run 'ollama serve' and ClawSwarm auto-detects Ollama." -ForegroundColor DarkGray
    Write-Host ""
}

# Main
if ($Help) {
    Write-Host "Usage: .\setup_clawswarm.ps1 [-Mode gui|tui|both|server] [-Silent]"
    exit 0
}

Write-Banner
$mode = Get-InstallMode
Setup-Config

switch ($mode) {
    "gui"    { Install-GUI    }
    "tui"    { Install-TUI    }
    "both"   { Install-GUI; Install-TUI }
    "server" { Install-Server }
}

Show-PostInstall $mode
