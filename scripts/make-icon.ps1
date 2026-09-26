# Renders assets/icon.html to assets/icon.png (1024x1024, transparent corners)
# and regenerates every app icon size in src-tauri/icons/.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$edge = "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe"
$html = "file:///" + (Join-Path $root 'assets\icon.html').Replace('\', '/')
$png = Join-Path $root 'assets\icon.png'
& $edge --headless=new --disable-gpu --hide-scrollbars --window-size=1024,1024 --default-background-color=00000000 `
    --virtual-time-budget=6000 --screenshot="$png" $html | Out-Null
Start-Sleep -Seconds 2
Set-Location $root
npx tauri icon $png
