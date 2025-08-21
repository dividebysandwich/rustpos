# dev.ps1
$ErrorActionPreference = "Stop"

# Build the project
& .\build.ps1

# Change to rustpos directory
Set-Location rustpos

# Start rustpos in background
$rustposProcess = Start-Process -FilePath ".\rustpos.exe" -PassThru -NoNewWindow

# Cleanup function
function Cleanup {
    Write-Host "Stopping development server..."
    if ($rustposProcess -and !$rustposProcess.HasExited) {
        try {
            $rustposProcess.Kill()
            $rustposProcess.WaitForExit(5000)  # Wait up to 5 seconds
        }
        catch {
            Write-Warning "Could not stop process gracefully"
        }
    }
    exit
}

# Register cleanup for Ctrl+C
Register-EngineEvent PowerShell.Exiting -Action { Cleanup }

# Handle Ctrl+C
try {
    Write-Host "Development server running. Press Ctrl+C to stop..."
    # Wait for the process to exit or user interrupt
    while (!$rustposProcess.HasExited) {
        Start-Sleep -Milliseconds 100
    }
}
catch {
    # This will catch Ctrl+C and other interruptions
    Cleanup
}
finally {
    Cleanup
}
