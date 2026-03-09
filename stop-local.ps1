$ErrorActionPreference = 'SilentlyContinue'

function Stop-PortProcess {
    param([int]$Port)

    Get-NetTCPConnection -LocalPort $Port -State Listen | Select-Object -ExpandProperty OwningProcess -Unique | ForEach-Object {
        Stop-Process -Id $_ -Force
        Write-Host "Stopped process on port $Port (PID $_)"
    }
}

Stop-PortProcess -Port 3000
Stop-PortProcess -Port 5173
