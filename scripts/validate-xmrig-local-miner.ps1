param(
    [string]$Version = "6.20.0",
    [string]$ArchiveUrl = "https://github.com/xmrig/xmrig/releases/download/v6.20.0/xmrig-6.20.0-msvc-win64.zip",
    [string]$ArchiveSha256 = "dd7fef5e3594eb18dd676e550e128d4b64cc5a469ff6954a677dc414265db468",
    [string]$ExecutableName = "xmrig.exe",
    [int]$Threads = 1,
    [int]$TelemetryTimeoutSeconds = 120,
    [switch]$KeepArtifacts
)

$ErrorActionPreference = "Stop"

function Invoke-CheckedCommand {
    param(
        [string]$FilePath,
        [string[]]$Arguments,
        [string]$FailureMessage
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FailureMessage (exit code $LASTEXITCODE)"
    }
}

function Wait-ForHealth {
    param(
        [string]$CliPath
    )

    $deadline = (Get-Date).AddSeconds(30)
    while ((Get-Date) -lt $deadline) {
        $healthOut = [System.IO.Path]::GetTempFileName()
        $healthErr = [System.IO.Path]::GetTempFileName()
        try {
            $health = Start-Process -FilePath $CliPath -ArgumentList @("health") -Wait -PassThru -WindowStyle Hidden -RedirectStandardOutput $healthOut -RedirectStandardError $healthErr
            if ($health.ExitCode -eq 0) {
                return
            }
        } finally {
            Remove-Item -Force -Path $healthOut, $healthErr -ErrorAction SilentlyContinue
        }
        Start-Sleep -Milliseconds 500
    }

    throw "daemon did not become healthy within 30 seconds"
}

function Assert-WindowsX64 {
    if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)) {
        throw "this validation script currently targets the Windows x64 XMRig archive"
    }
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -ne "X64") {
        throw "this validation script currently targets x86_64 hosts"
    }
}

Assert-WindowsX64

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$daemonPath = Join-Path $repoRoot "target\debug\bunker-miner-daemon.exe"
$cliPath = Join-Path $repoRoot "target\debug\bunker-miner-cli.exe"

Push-Location $repoRoot
try {
    Invoke-CheckedCommand -FilePath "cargo" -Arguments @("build", "--workspace") -FailureMessage "workspace build failed"

    if (-not (Test-Path $daemonPath)) {
        throw "daemon binary was not built at $daemonPath"
    }
    if (-not (Test-Path $cliPath)) {
        throw "CLI binary was not built at $cliPath"
    }

    $tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("bunker-miner-xmrig-validation-" + [guid]::NewGuid().ToString("N"))
    $configDir = Join-Path $tempRoot "config"
    $downloadDir = Join-Path $tempRoot "download"
    $extractDir = Join-Path $tempRoot "extract"
    $daemonOut = Join-Path $tempRoot "daemon.out.log"
    $daemonErr = Join-Path $tempRoot "daemon.err.log"
    $watchOut = Join-Path $tempRoot "watch.out.log"
    $watchErr = Join-Path $tempRoot "watch.err.log"

    New-Item -ItemType Directory -Force -Path $configDir, $downloadDir, $extractDir | Out-Null
    $archivePath = Join-Path $downloadDir ([System.IO.Path]::GetFileName(([Uri]$ArchiveUrl).AbsolutePath))

    Write-Host "Downloading $ArchiveUrl"
    Invoke-WebRequest -Uri $ArchiveUrl -OutFile $archivePath

    $actualArchiveSha256 = (Get-FileHash -Algorithm SHA256 -Path $archivePath).Hash.ToLowerInvariant()
    if ($actualArchiveSha256 -ne $ArchiveSha256.ToLowerInvariant()) {
        throw "archive SHA-256 mismatch: expected $ArchiveSha256 got $actualArchiveSha256"
    }

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $extractDir)
    $executables = Get-ChildItem -Path $extractDir -Recurse -File -Filter $ExecutableName
    if ($executables.Count -eq 0) {
        throw "archive did not contain $ExecutableName"
    }
    if ($executables.Count -gt 1) {
        throw "archive contained multiple $ExecutableName files"
    }

    $executableSha256 = (Get-FileHash -Algorithm SHA256 -Path $executables[0].FullName).Hash.ToLowerInvariant()
    $manifestPath = Join-Path $configDir "miner-manifest.toml"
    @"
schema_version = 1

[[miners]]
name = "XMRig"
version = "$Version"
platform = "windows-x86_64"
executable = "$ExecutableName"
sha256 = "$executableSha256"
source_url = "$ArchiveUrl"
archive_sha256 = "$actualArchiveSha256"
"@ | Set-Content -Encoding ASCII -Path $manifestPath

    $oldConfigDir = $env:BUNKER_MINER_CONFIG_DIR
    $oldPassword = $env:BUNKER_MINER_CONFIG_PASSWORD
    $oldManifest = $env:BUNKER_MINER_MANIFEST_PATH
    $env:BUNKER_MINER_CONFIG_DIR = $configDir
    $env:BUNKER_MINER_CONFIG_PASSWORD = "xmrig-validation-password"
    Remove-Item Env:BUNKER_MINER_MANIFEST_PATH -ErrorAction SilentlyContinue

    $daemon = $null
    $watch = $null
    try {
        $daemon = Start-Process -FilePath $daemonPath -ArgumentList @("serve") -PassThru -WindowStyle Hidden -RedirectStandardOutput $daemonOut -RedirectStandardError $daemonErr
        Wait-ForHealth $cliPath

        Invoke-CheckedCommand -FilePath $cliPath -Arguments @("miner", "install", "--name", "XMRig", "--version", $Version, "--timeout-seconds", "180") -FailureMessage "miner install failed"

        $watch = Start-Process -FilePath $cliPath -ArgumentList @("watch", "--interval", "1") -PassThru -WindowStyle Hidden -RedirectStandardOutput $watchOut -RedirectStandardError $watchErr
        Start-Sleep -Milliseconds 750

        Invoke-CheckedCommand -FilePath $cliPath -Arguments @(
            "start",
            "--algorithm", "randomx",
            "--pool", "127.0.0.1:1",
            "--wallet", "validation-wallet-not-used",
            "--device", "$Threads",
            "--param", "xmrig_benchmark=1M",
            "--param", "print_time_seconds=1",
            "--timeout-seconds", "60"
        ) -FailureMessage "diagnostic XMRig start failed"

        $deadline = (Get-Date).AddSeconds($TelemetryTimeoutSeconds)
        $telemetryObserved = $false
        $observedTelemetryLine = $null
        while ((Get-Date) -lt $deadline) {
            if (Test-Path $watchOut) {
                $watchText = Get-Content -Raw -Path $watchOut
                foreach ($line in ($watchText -split "`r?`n")) {
                    if ($line -match "algorithm=randomx" -and $line -match "hashrate=([0-9]+(?:\.[0-9]+)?) (H/s|kH/s|MH/s)") {
                        if ([double]$Matches[1] -gt 0) {
                            $telemetryObserved = $true
                            $observedTelemetryLine = $line
                            break
                        }
                    }
                }
                if ($telemetryObserved) { break }
            }
            Start-Sleep -Seconds 1
        }

        if (-not $telemetryObserved) {
            throw "no randomx telemetry was observed within $TelemetryTimeoutSeconds seconds"
        }

        Invoke-CheckedCommand -FilePath $cliPath -Arguments @("stop", "--force", "--timeout-seconds", "10") -FailureMessage "miner stop failed"

        Write-Host "XMRig validation passed"
        Write-Host "Observed telemetry: $observedTelemetryLine"
        Write-Host "Archive SHA-256: $actualArchiveSha256"
        Write-Host "Executable SHA-256: $executableSha256"
        if ($KeepArtifacts) {
            Write-Host "Artifacts: $tempRoot"
        } else {
            Write-Host "Artifacts removed after validation: $tempRoot"
        }
    } finally {
        if ($watch -and -not $watch.HasExited) {
            Stop-Process -Id $watch.Id -Force
            Wait-Process -Id $watch.Id -Timeout 10 -ErrorAction SilentlyContinue
        }
        if ($daemon -and -not $daemon.HasExited) {
            Stop-Process -Id $daemon.Id -Force
            Wait-Process -Id $daemon.Id -Timeout 10 -ErrorAction SilentlyContinue
        }
        if ($null -eq $oldConfigDir) { Remove-Item Env:BUNKER_MINER_CONFIG_DIR -ErrorAction SilentlyContinue } else { $env:BUNKER_MINER_CONFIG_DIR = $oldConfigDir }
        if ($null -eq $oldPassword) { Remove-Item Env:BUNKER_MINER_CONFIG_PASSWORD -ErrorAction SilentlyContinue } else { $env:BUNKER_MINER_CONFIG_PASSWORD = $oldPassword }
        if ($null -eq $oldManifest) { Remove-Item Env:BUNKER_MINER_MANIFEST_PATH -ErrorAction SilentlyContinue } else { $env:BUNKER_MINER_MANIFEST_PATH = $oldManifest }

        if (-not $KeepArtifacts) {
            Remove-Item -Recurse -Force -LiteralPath $tempRoot -ErrorAction SilentlyContinue
        }
    }
} finally {
    Pop-Location
}
