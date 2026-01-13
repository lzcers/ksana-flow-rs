$ScriptRoot = $PSScriptRoot

# Function to check if a command exists
function Test-CommandExists {
    param ($Command)
    $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
}

# Determine package manager
$PackageManager = "npm"
if (Test-Path "$ScriptRoot\web\pnpm-lock.yaml") {
    if (Test-CommandExists pnpm) {
        $PackageManager = "pnpm"
    } else {
        Write-Warning "pnpm-lock.yaml found but pnpm not installed. Falling back to npm."
    }
}

Write-Host "Using package manager: $PackageManager" -ForegroundColor Cyan

# Start Backend
Write-Host "Starting Backend Server (cargo run -p server)..." -ForegroundColor Green
# Start cargo directly. -NoNewWindow streams output to current console.
$backend = Start-Process -FilePath "cargo" -ArgumentList "run", "-p", "server" -WorkingDirectory $ScriptRoot -NoNewWindow -PassThru

# Start Frontend
Write-Host "Starting Frontend ($PackageManager run dev)..." -ForegroundColor Green
# Wrap in powershell to handle shell command resolution (e.g. .cmd files) and execution
$frontend = Start-Process powershell -ArgumentList "-Command", "$PackageManager run dev" -WorkingDirectory "$ScriptRoot\web" -NoNewWindow -PassThru

Write-Host "Development environment started! Press Ctrl+C to stop." -ForegroundColor Yellow

try {
    # Wait for both processes. If one exits, we continue waiting for the other, 
    # but practically we want to keep the script running as long as either is alive 
    # or until user interrupts.
    Wait-Process -Id $backend.Id, $frontend.Id
}
finally {
    # Ensure processes are terminated when the script exits (e.g. via Ctrl+C)
    Write-Host "`nStopping processes..." -ForegroundColor Yellow
    Stop-Process -Id $backend.Id, $frontend.Id -ErrorAction SilentlyContinue
}
