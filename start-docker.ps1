$ErrorActionPreference = 'Stop'
$workspaceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host 'Docker Desktop is not installed or not running.' -ForegroundColor Red
    Write-Host 'Install/start Docker Desktop first, then run this script again.' -ForegroundColor Yellow
    Write-Host 'Official download: https://www.docker.com/products/docker-desktop/' -ForegroundColor Cyan
    exit 1
}

Set-Location $workspaceRoot

docker compose up --build
