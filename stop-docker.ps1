$ErrorActionPreference = 'Stop'
$workspaceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host 'Docker Desktop is not installed or not running.' -ForegroundColor Red
    exit 1
}

Set-Location $workspaceRoot

docker compose down
