# Downloads the ffmpeg build that ships inside the installer into src-tauri/ffmpeg/.
# The binaries are not committed to git; run this once before `npm run tauri build`.
# Source: gyan.dev "release essentials" (GPL build: x264, NVENC, AMF, QSV).
# Pinned to one version so local and GitHub builds ship the same ffmpeg.
param([string]$Version = '9.0.2')
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root 'src-tauri\ffmpeg'
$url = "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-$Version-essentials_build.zip"
$zip = Join-Path $env:TEMP 'shrink-ffmpeg-essentials.zip'
$tmp = Join-Path $env:TEMP 'shrink-ffmpeg-essentials'

Write-Host "Downloading $url"
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
Expand-Archive -Path $zip -DestinationPath $tmp

$build = Get-ChildItem $tmp -Directory | Select-Object -First 1
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item (Join-Path $build.FullName 'bin\ffmpeg.exe') $dest -Force
Copy-Item (Join-Path $build.FullName 'bin\ffprobe.exe') $dest -Force
Copy-Item (Join-Path $build.FullName 'LICENSE') (Join-Path $dest 'LICENSE-ffmpeg.txt') -Force
Copy-Item (Join-Path $build.FullName 'README.txt') (Join-Path $dest 'README-ffmpeg.txt') -Force
Set-Content -Path (Join-Path $dest 'VERSION.txt') -Value "$($build.Name) from $url"

Remove-Item -Recurse -Force $tmp, $zip
Get-ChildItem $dest | Format-Table Name, Length
