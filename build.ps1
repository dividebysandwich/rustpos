# Set error handling
$ErrorActionPreference = "Stop"

Write-Host "Installing tools..."

# Check if trunk is installed, install if not
try {
    Get-Command trunk -ErrorAction Stop | Out-Null
} catch {
    cargo install trunk
}

rustup target add wasm32-unknown-unknown

Write-Host "Building frontend..."
Set-Location frontend
trunk build --release  #--public-url /
Set-Location ..

Write-Host "Copying frontend files to backend..."
if (!(Test-Path "rustpos/data")) {
    New-Item -ItemType Directory -Path "rustpos/data" -Force
}

Copy-Item -Path "backend/data/*" -Destination "rustpos/data" -Recurse -Force

if (Test-Path "rustpos/static") {
    Remove-Item -Path "rustpos/static" -Recurse -Force
}

Copy-Item -Path "frontend/dist" -Destination "rustpos/static" -Recurse
Copy-Item -Path "backend/data/logo_site.png" -Destination "rustpos/static/"

Write-Host "Frontend files copied to rustpos/static"

Write-Host "Building backend..."
Set-Location backend
cargo build --release
Set-Location ..

Copy-Item -Path "target/release/rustpos-backend.exe" -Destination "rustpos/rustpos.exe"

Write-Host "Build complete!"
Write-Host "Binary location: rustpos/rustpos.exe"
Write-Host "Run with: cd rustpos && ./rustpos.exe"
