# Builds, signs and publishes a shrink release on GitHub.
#
#   pwsh -ExecutionPolicy Bypass -File scripts/release.ps1
#
# Before running:
#   - bump "version" in package.json, src-tauri/Cargo.toml and src-tauri/tauri.conf.json
#   - write the release notes to docs/releases/<version>.md
#   - commit and push; the release is tagged at the pushed commit
#   - the updater signing key (from Vaultwarden) at $KeyPath, its password at $PasswordPath
param(
    [string]$KeyPath = "$env:USERPROFILE\.tauri\shrink-updater.key",
    [string]$PasswordPath = "$env:USERPROFILE\.tauri\shrink-updater.password",
    [string]$Repo = 'asunoaim/shrink'
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$version = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$tag = "v$version"
$notesFile = "docs/releases/$version.md"
if (-not (Test-Path $notesFile)) { throw "Write the release notes first: $notesFile" }
if (git status --porcelain) { throw 'Commit your changes first.' }
if (-not (Test-Path src-tauri/ffmpeg/ffmpeg.exe)) { throw 'Run scripts/fetch-ffmpeg.ps1 first.' }

# build + sign
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $KeyPath -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = (Get-Content $PasswordPath -Raw).Trim()
npx tauri build
if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }

$bundle = 'src-tauri/target/release/bundle/nsis'
$setupName = "shrink_${version}_x64-setup.exe"
$setup = Join-Path $bundle $setupName
$sig = "$setup.sig"

# the file installed copies read to find updates
$notes = Get-Content $notesFile -Raw
$latest = [ordered]@{
    version   = $version
    notes     = $notes
    pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    platforms = [ordered]@{
        'windows-x86_64' = [ordered]@{
            signature = (Get-Content $sig -Raw).Trim()
            url       = "https://github.com/$Repo/releases/download/$tag/$setupName"
        }
    }
}
$latestJson = Join-Path $bundle 'latest.json'
$latest | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8NoBOM $latestJson

# GPL: ship the source of the bundled ffmpeg alongside the binary
$ffVersion = ((Get-Content src-tauri/ffmpeg/VERSION.txt -Raw) -replace '^ffmpeg-([\d.]+)-.*$', '$1').Trim()
$ffSource = Join-Path $env:TEMP "ffmpeg-$ffVersion.tar.xz"
if (-not (Test-Path $ffSource)) {
    curl.exe -sSL -o $ffSource "https://ffmpeg.org/releases/ffmpeg-$ffVersion.tar.xz"
}
$ffReadme = Join-Path $env:TEMP 'ffmpeg-build-components.txt'
Copy-Item src-tauri/ffmpeg/README-ffmpeg.txt $ffReadme -Force

gh release create $tag --repo $Repo --title "shrink $version" --notes-file $notesFile `
    $setup $sig $latestJson $ffSource $ffReadme
if ($LASTEXITCODE -ne 0) { throw 'Publishing failed.' }
Write-Host "Published $tag"
