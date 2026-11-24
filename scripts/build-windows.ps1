param([Parameter(Mandatory = $true)][string]$WorkDir)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (![Environment]::Is64BitProcess -or $env:OS -ne 'Windows_NT') { throw 'Windows x64 build required' }
$WorkDir = [IO.Path]::GetFullPath($WorkDir)
$source = Get-Content (Join-Path $WorkDir 'source-paths.json') -Raw | ConvertFrom-Json
$prefix = Join-Path $WorkDir 'prefix'
$bin = Join-Path $WorkDir 'bin'
New-Item -ItemType Directory -Path $bin, $prefix | Out-Null

# Import the runner's installed MSVC/SDK only into this build process.
$vswhere = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or !$vs) { throw 'MSVC x64 tools not found' }
Import-Module (Join-Path $vs 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64'

function Invoke-Native {
    param([string]$Program, [string[]]$Arguments)
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE" }
}
function Build-CMake {
    param([string]$Name, [string]$SourcePath, [string[]]$Options = @())
    $build = Join-Path $WorkDir "build-$Name"
    $configure = @('-S', $SourcePath, '-B', $build, '-G', 'Ninja',
        '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_POLICY_VERSION_MINIMUM=3.5', "-DCMAKE_INSTALL_PREFIX=$prefix",
        "-DCMAKE_PREFIX_PATH=$prefix", '-DCMAKE_POLICY_DEFAULT_CMP0091=NEW',
        '-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded',
        '-DZLIB_USE_STATIC_LIBS=ON', '-DCMAKE_C_FLAGS=/utf-8', '-DCMAKE_CXX_FLAGS=/utf-8') + $Options
    Invoke-Native 'cmake' $configure
    Invoke-Native 'cmake' @('--build', $build, '--parallel', '4')
    Invoke-Native 'ctest' @('--test-dir', $build, '--output-on-failure', '-C', 'Release')
    Invoke-Native 'cmake' @('--install', $build)
}
Build-CMake 'zlib' $source.zlib @('-DZLIB_BUILD_EXAMPLES=ON')
Build-CMake 'libpng' $source.libpng @('-DPNG_SHARED=OFF', '-DPNG_STATIC=ON', '-DPNG_TESTS=ON', '-DPNG_TOOLS=OFF')
# Upstream supports scalar builds: no unpinned NASM download; final smoke guards quality.
Build-CMake 'mozjpeg' $source.mozjpeg @('-DENABLE_SHARED=OFF', '-DENABLE_STATIC=ON',
    '-DWITH_CRT_DLL=OFF', '-DWITH_SIMD=OFF', '-DPNG_SUPPORTED=ON', '-DWITH_TURBOJPEG=OFF')
foreach ($name in @('cjpeg', 'jpegtran')) {
    $candidate = Join-Path $WorkDir "build-mozjpeg/$name-static.exe"
    if (!(Test-Path $candidate)) { throw "Missing mozjpeg static executable: $candidate" }
    Copy-Item $candidate (Join-Path $bin "$name.exe")
}

# giflib has a C library with Windows branches but no upstream CMake recipe.
$gifCmake = Join-Path $WorkDir 'giflib-cmake'
New-Item -ItemType Directory -Path $gifCmake | Out-Null
$gifSource = $source.giflib.Replace('\', '/')
@"
cmake_minimum_required(VERSION 3.20)
project(yali_giflib C)
set(CMAKE_C_STANDARD 99)
add_library(gif STATIC
  "$gifSource/dgif_lib.c" "$gifSource/egif_lib.c" "$gifSource/gifalloc.c"
  "$gifSource/gif_err.c" "$gifSource/gif_font.c" "$gifSource/gif_hash.c"
  "$gifSource/openbsd-reallocarray.c")
target_include_directories(gif PUBLIC "$gifSource")
target_compile_definitions(gif PRIVATE _CRT_SECURE_NO_WARNINGS _CRT_NONSTDC_NO_DEPRECATE)
install(TARGETS gif ARCHIVE DESTINATION lib)
install(FILES "$gifSource/gif_lib.h" DESTINATION include)
"@ | Set-Content -Encoding utf8 (Join-Path $gifCmake 'CMakeLists.txt')
Build-CMake 'giflib' $gifCmake
Build-CMake 'libwebp' $source.libwebp @('-DBUILD_SHARED_LIBS=OFF',
    '-DWEBP_BUILD_VWEBP=OFF', '-DWEBP_BUILD_EXTRAS=OFF', '-DWEBP_BUILD_GIF2WEBP=ON')
foreach ($name in @('cwebp', 'gif2webp')) {
    Copy-Item (Join-Path $WorkDir "build-libwebp/$name.exe") (Join-Path $bin "$name.exe")
}

Push-Location (Join-Path $source.gifsicle 'src')
try {
    Invoke-Native 'nmake' @('/f', 'Makefile.w32', 'gifsicle.exe', 'CFLAGS=-I.. -I..\include -DHAVE_CONFIG_H -D_CONSOLE /W2 /O2 /MT /utf-8')
    Copy-Item 'gifsicle.exe' (Join-Path $bin 'gifsicle.exe')
} finally { Pop-Location }

$env:RUSTFLAGS = '-C target-feature=+crt-static'
Invoke-Native 'cargo' @('test', '--locked', '--release', '--manifest-path', (Join-Path $source.pngquant 'Cargo.toml'),
    '--features', 'static,z-static', '--target-dir', (Join-Path $WorkDir 'pngquant-target'))
Invoke-Native 'cargo' @('build', '--locked', '--release', '--manifest-path', (Join-Path $source.pngquant 'Cargo.toml'),
    '--features', 'static,z-static', '--target-dir', (Join-Path $WorkDir 'pngquant-target'))
Copy-Item (Join-Path $WorkDir 'pngquant-target/release/pngquant.exe') (Join-Path $bin 'pngquant.exe')
Invoke-Native 'cargo' @('vendor', '--locked', '--manifest-path', (Join-Path $source.pngquant 'Cargo.toml'), (Join-Path $WorkDir 'vendor'))

# libvips includes a Rust librsvg DLL: preserve its transitive corresponding source too.
$rsvg = $source.'vips-source-librsvg'
Invoke-Native 'cargo' @('vendor', '--locked', '--manifest-path', (Join-Path $rsvg 'Cargo.toml'), (Join-Path $WorkDir 'vendor-librsvg'))

# Official oxipng MSVC binary imports vcruntime140.dll. Copy only the licensed x64 redistributable.
$crt = Get-ChildItem (Join-Path $vs 'VC/Redist/MSVC') -Directory -Recurse |
    Where-Object { $_.FullName -match '[\\\\/]x64[\\\\/]Microsoft[.]VC14[0-9][.]CRT$' } |
    Sort-Object FullName -Descending | Select-Object -First 1
if (!$crt) { throw 'Visual Studio x64 redistributable CRT directory not found' }
Copy-Item (Join-Path $crt.FullName 'vcruntime140.dll') (Join-Path $bin 'vcruntime140.dll')
$licenseRoot = Join-Path $vs 'Licenses'
$licenseFiles = @(Get-ChildItem $licenseRoot -File -Recurse | Where-Object { $_.Extension -in @('.txt', '.rtf', '.html') })
if ($licenseFiles.Count -eq 0) { throw 'Visual Studio distribution license material not found; stop packaging' }
$msvcLicenses = Join-Path $WorkDir 'msvc-licenses'
New-Item -ItemType Directory -Path $msvcLicenses | Out-Null
foreach ($file in $licenseFiles) {
    $relative = [IO.Path]::GetRelativePath($licenseRoot, $file.FullName)
    $destination = Join-Path $msvcLicenses $relative
    New-Item -ItemType Directory -Force (Split-Path $destination -Parent) | Out-Null
    Copy-Item $file.FullName $destination
}
@{
    compiler = (& cl.exe 2>&1 | Out-String)
    crtVersion = (Get-Item (Join-Path $bin 'vcruntime140.dll')).VersionInfo.FileVersion
    crtSource = $crt.FullName
    mozjpegSIMD = $false
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $WorkDir 'build-tools.json')

# Narrow argv/fopen in upstream C tools must use UTF-8 for Chinese paths on Windows 11.
$manifest = Join-Path $WorkDir 'utf8.manifest'
@'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
    </windowsSettings>
  </application>
</assembly>
'@ | Set-Content -Encoding utf8 $manifest
foreach ($name in @('cjpeg', 'jpegtran', 'gifsicle', 'cwebp', 'gif2webp')) {
    $exe = Join-Path $bin "$name.exe"
    Invoke-Native 'mt.exe' @('-nologo', '-manifest', $manifest, "-outputresource:$exe;#1")
}
