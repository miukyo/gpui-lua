<#
.SYNOPSIS
Rebuilds the pure-Rust Luau WebAssembly engine used by the docs playground.

.DESCRIPTION
Mirrors upstream `website/wasm.sh` from https://github.com/webc-site/ulua:
builds `ulua-web` for wasm32-unknown-unknown and runs wasm-bindgen over it, then
installs the artifacts into:

    docs/.vitepress/theme/ulua/   (bundled by the theme)
    docs/public/                  (served at /ulua_web_bg.wasm)

Requirements (installed automatically if missing):
    rustup with the wasm32-unknown-unknown target
    wasm-bindgen-cli, pinned to the version in ulua's Cargo.lock
    wasm-opt (optional, only used to shrink the release artifact)

.EXAMPLE
pwsh -File .vitepress/theme/ulua/build-wasm.ps1

.EXAMPLE
pwsh -File .vitepress/theme/ulua/build-wasm.ps1 -Tag ulua-web-v0.1.6
#>

[CmdletBinding()]
param(
    # Release tag of github.com/webc-site/ulua to build against.
    [string]$Tag = 'ulua-web-v0.1.6',

    # Emit an unoptimized build. Faster, but a much larger .wasm.
    [switch]$DebugBuild,

    # Skip wasm-opt even when it is installed.
    [switch]$NoWasmOpt,

    # Reuse an existing checkout instead of re-cloning into the temp dir.
    [string]$RepoDir
)

$ErrorActionPreference = 'Stop'

$themeUluaDir = $PSScriptRoot
$docsDir = (Resolve-Path (Join-Path $themeUluaDir '..' '..' '..')).Path
$publicDir = Join-Path $docsDir 'public'

function Invoke-Checked {
    param([string]$Command, [string[]]$Arguments, [string]$What)

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$What failed (exit $LASTEXITCODE): $Command $($Arguments -join ' ')"
    }
}

function Get-ToolPath {
    param([string]$Name)
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

if (-not (Get-ToolPath 'cargo')) {
    throw 'cargo not found on PATH. Install Rust from https://rustup.rs first.'
}

# --- 1. rustup + wasm32 target ------------------------------------------------
$rustup = Get-ToolPath 'rustup'
if (-not $rustup) {
    throw 'rustup not found on PATH. Install it from https://rustup.rs, then re-run this script.'
}

$installedTargets = & $rustup target list --installed
if ($installedTargets -notcontains 'wasm32-unknown-unknown') {
    Write-Host '==> Adding wasm32-unknown-unknown target'
    Invoke-Checked $rustup @('target', 'add', 'wasm32-unknown-unknown') 'rustup target add'
}

$cargo = (& $rustup which cargo 2>$null)
if (-not $cargo) { $cargo = 'cargo' } else { $cargo = $cargo.Trim() }
$rustc = (& $rustup which rustc 2>$null)
if ($rustc) { $env:RUSTC = $rustc.Trim() }

# --- 2. fetch the ulua sources at $Tag ---------------------------------------
if (-not $RepoDir) {
    $RepoDir = Join-Path ([System.IO.Path]::GetTempPath()) ("ulua-" + $Tag)
    if (Test-Path (Join-Path $RepoDir '.git')) {
        Write-Host "==> Reusing checkout at $RepoDir"
        Invoke-Checked 'git' @('-C', $RepoDir, 'fetch', '--tags', '--force') 'git fetch'
        Invoke-Checked 'git' @('-C', $RepoDir, 'checkout', '--force', $Tag) 'git checkout'
    }
    else {
        Write-Host "==> Cloning webc-site/ulua at $Tag into $RepoDir"
        if (Test-Path $RepoDir) { Remove-Item -Recurse -Force $RepoDir }
        Invoke-Checked 'git' @('clone', '--depth', '1', '--branch', $Tag,
            'https://github.com/webc-site/ulua.git', $RepoDir) 'git clone'
    }
}
else {
    $RepoDir = (Resolve-Path $RepoDir).Path
}

# --- 3. wasm-bindgen-cli, pinned to ulua's Cargo.lock -------------------------
$expectedBindgen = $null
$lockPath = Join-Path $RepoDir 'Cargo.lock'
if (Test-Path $lockPath) {
    $lines = Get-Content $lockPath
    for ($i = 0; $i -lt $lines.Count - 1; $i++) {
        if ($lines[$i] -match '^name = "wasm-bindgen"$') {
            if ($lines[$i + 1] -match '^version = "([^"]+)"$') {
                $expectedBindgen = $Matches[1]
                break
            }
        }
    }
}
if (-not $expectedBindgen) { $expectedBindgen = '0.2.129' }

$bindgen = Get-ToolPath 'wasm-bindgen'
$bindgenOk = $false
if ($bindgen) {
    $current = (& $bindgen --version 2>&1 | Out-String).Trim() -replace '^wasm-bindgen\s+', ''
    $bindgenOk = ($current -eq $expectedBindgen)
    if (-not $bindgenOk) {
        Write-Host "==> wasm-bindgen $current does not match $expectedBindgen, reinstalling"
    }
}
if (-not $bindgenOk) {
    Write-Host "==> Installing wasm-bindgen-cli $expectedBindgen (compiles from source, be patient)"
    Invoke-Checked $cargo @('install', 'wasm-bindgen-cli', '--version', $expectedBindgen, '--force') 'cargo install wasm-bindgen-cli'
    $bindgen = Get-ToolPath 'wasm-bindgen'
    if (-not $bindgen) { throw 'wasm-bindgen-cli installed but not found on PATH.' }
}

# --- 4. build ulua-web for wasm32 --------------------------------------------
$profile = if ($DebugBuild) { 'debug' } else { 'release' }
$buildArgs = @('build', '-p', 'ulua-web', '--target', 'wasm32-unknown-unknown', '--features', 'wasm')
if (-not $DebugBuild) { $buildArgs += '--release' }

Write-Host "==> Building ulua-web ($profile) from $RepoDir"
Push-Location $RepoDir
try {
    Invoke-Checked $cargo $buildArgs 'cargo build -p ulua-web'
}
finally {
    Pop-Location
}

$metadata = & $cargo metadata --format-version 1 --manifest-path (Join-Path $RepoDir 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$targetDir = ($metadata | ConvertFrom-Json).target_directory
$wasmSource = Join-Path $targetDir "wasm32-unknown-unknown/$profile/ulua_web.wasm"
if (-not (Test-Path $wasmSource)) { throw "Build produced no wasm at $wasmSource" }

# --- 5. wasm-bindgen -----------------------------------------------------------
$pkgDir = Join-Path ([System.IO.Path]::GetTempPath()) 'ulua-web-pkg'
if (Test-Path $pkgDir) { Remove-Item -Recurse -Force $pkgDir }
New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null

Write-Host '==> Generating JS glue with wasm-bindgen'
Invoke-Checked $bindgen @('--target', 'web', '--out-dir', $pkgDir, $wasmSource) 'wasm-bindgen'

$glueWasm = Join-Path $pkgDir 'ulua_web_bg.wasm'
if (-not (Test-Path $glueWasm)) { throw "wasm-bindgen produced no ulua_web_bg.wasm in $pkgDir" }

if ((-not $DebugBuild) -and (-not $NoWasmOpt)) {
    $wasmOpt = Get-ToolPath 'wasm-opt'
    if ($wasmOpt) {
        Write-Host '==> Optimizing with wasm-opt -O3'
        Invoke-Checked $wasmOpt @('-O3', $glueWasm, '-o', $glueWasm) 'wasm-opt'
    }
    else {
        Write-Host '==> wasm-opt not found, skipping size optimization'
    }
}

# --- 6. install the artifacts -------------------------------------------------
$glue = Join-Path $pkgDir 'ulua_web.js'
$optional = @('ulua_web_bg.js', 'ulua_web.d.ts')

Write-Host "==> Installing into $themeUluaDir"
Copy-Item $glue $themeUluaDir -Force
Copy-Item $glueWasm $themeUluaDir -Force
foreach ($name in $optional) {
    $src = Join-Path $pkgDir $name
    if (Test-Path $src) { Copy-Item $src $themeUluaDir -Force }
}

New-Item -ItemType Directory -Force -Path $publicDir | Out-Null
Copy-Item $glueWasm $publicDir -Force

Set-Content -Path (Join-Path $themeUluaDir 'VERSION') -Value $Tag -NoNewline -Encoding utf8

$sizeKb = [math]::Round((Get-Item $glueWasm).Length / 1KB, 1)
Write-Host ''
Write-Host "==> Done: $Tag ($profile)"
Write-Host "    theme/ulua/ulua_web_bg.wasm  $sizeKb KB"
Write-Host "    public/ulua_web_bg.wasm       $sizeKb KB"
Write-Host '    Run `npm run docs:build` to verify.'
