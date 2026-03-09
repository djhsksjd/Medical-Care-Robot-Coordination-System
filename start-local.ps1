$ErrorActionPreference = 'Stop'

$workspaceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$backendPath = Join-Path $workspaceRoot 'COMP_2432project'
$frontendPath = Join-Path $workspaceRoot 'frontend'

function Stop-PortProcess {
    param([int]$Port)

    $connections = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
    if ($connections) {
        $connections | Select-Object -ExpandProperty OwningProcess -Unique | ForEach-Object {
            try {
                Stop-Process -Id $_ -Force -ErrorAction Stop
                Write-Host "Stopped process on port $Port (PID $_)"
            } catch {
                Write-Warning "Could not stop PID $_ on port $Port: $($_.Exception.Message)"
            }
        }
    }
}

Stop-PortProcess -Port 3000
Stop-PortProcess -Port 5173

Start-Process powershell -ArgumentList @(
    '-NoExit',
    '-Command',
    "Set-Location '$backendPath'; cargo run"
)

Start-Sleep -Seconds 2

Start-Process powershell -ArgumentList @(
    '-NoExit',
    '-Command',
    "Set-Location '$frontendPath'; npm run dev"
)

Write-Host 'Backend starting on http://localhost:3000'
Write-Host 'Frontend starting on http://localhost:5173'
Write-Host 'Open http://localhost:5173 in your browser.'
