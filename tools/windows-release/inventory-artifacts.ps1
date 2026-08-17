# Inventory currently produced Windows release artifacts.
# Run from the Sammy repository root. Does not install, sign, or rebuild.
# Signing material must never be printed.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not (Test-Path (Join-Path $root 'src-tauri\tauri.conf.json'))) {
    $root = (Get-Location).Path
}

$artifacts = @(
    @{ Kind = 'EXE';  Rel = 'target\release\sammy.exe' },
    @{ Kind = 'CLI';  Rel = 'target\release\sammy-cli.exe' },
    @{ Kind = 'MSI';  Rel = 'target\release\bundle\msi\Sammy_0.1.0_x64_en-US.msi' },
    @{ Kind = 'NSIS'; Rel = 'target\release\bundle\nsis\Sammy_0.1.0_x64-setup.exe' }
)

Write-Output "Repository: $root"
Write-Output "Windows: $([System.Environment]::OSVersion.VersionString)"
Write-Output ''

foreach ($item in $artifacts) {
    $path = Join-Path $root $item.Rel
    Write-Output "=== $($item.Kind) ==="
    Write-Output "Path: $path"
    if (-not (Test-Path $path)) {
        Write-Output 'Present: no'
        Write-Output ''
        continue
    }
    $info = Get-Item $path
    $hash = (Get-FileHash -Path $path -Algorithm SHA256).Hash
    $sig = Get-AuthenticodeSignature -FilePath $path
    Write-Output "Filename: $($info.Name)"
    Write-Output "Size: $($info.Length)"
    Write-Output "LastWriteTime: $($info.LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss zzz'))"
    Write-Output "SHA-256: $hash"
    Write-Output "Authenticode: $($sig.Status)"
    if ($sig.SignerCertificate) {
        Write-Output "Signer: $($sig.SignerCertificate.Subject)"
        Write-Output "Timestamp: $($sig.TimeStamperCertificate.Subject)"
    }
    $version = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($path)
    if ($version.FileVersion) {
        Write-Output "FileVersion: $($version.FileVersion)"
        Write-Output "ProductVersion: $($version.ProductVersion)"
        Write-Output "ProductName: $($version.ProductName)"
        Write-Output "CompanyName: $($version.CompanyName)"
    }
    Write-Output ''
}
