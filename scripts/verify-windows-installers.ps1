#!/usr/bin/env pwsh
# Verify that the Stratum Windows installers install, launch and uninstall.
#
# Usage:
#   pwsh scripts/verify-windows-installers.ps1                  # inspect dist/packages
#   pwsh scripts/verify-windows-installers.ps1 -Dir <dir>
#   pwsh scripts/verify-windows-installers.ps1 -SkipLaunch      # install checks only
#
# The release workflow calls this on a fresh windows-latest runner after the
# installers are built (and signed, when WINDOWS_CERTIFICATE is set), so the
# artifacts are never "verified" by the machine that produced them:
#
#   .msi   msiexec /i /qn, then the installed shell is located, launched and
#          the product is removed again through its uninstall entry
#   .exe   NSIS installer in silent mode (/S), same launch probe, uninstaller
#          in silent mode
#
# The launch probe starts the installed binary with STRATUM_FIXTURE=1 and
# requires it to either stay alive for the whole probe window or exit cleanly
# (a headless runner may refuse to create a window; a crash is still fatal).
# The single-instance glue hands argv to a running instance, so the probe runs
# before any other copy of the shell is started.
#
# Exit code 0 means every installer installed, launched and uninstalled.
[CmdletBinding()]
param(
    [string]$Dir = 'dist/packages',
    [int]$ProbeSeconds = 15,
    [switch]$SkipLaunch
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Dir)) {
    Write-Error "artifact directory '$Dir' does not exist."
    exit 1
}
$Dir = (Resolve-Path $Dir).Path

$script:failed = 0
$script:results = @()

function Record-Result {
    param([string]$Name, [string]$Status, [string]$Detail)
    $script:results += ('{0,-34} {1,-6} {2}' -f $Name, $Status, $Detail)
    Write-Host ("==> {0}: {1} {2}" -f $Name, $Status, $Detail)
    if ($Status -eq 'FAIL') { $script:failed = 1 }
}

# The installers are per-user (MSI) or current-user (NSIS), so the payload lands
# under %LOCALAPPDATA%; the search covers Program Files too in case a template
# ever switches scope. The newest match wins: the probe must not pick up a stale
# copy from a previous install.
function Find-InstalledBinary {
    $roots = @($env:LOCALAPPDATA, $env:ProgramFiles, ${env:ProgramFiles(x86)}) |
        Where-Object { $_ -and (Test-Path $_) }
    $found = @()
    foreach ($root in $roots) {
        $found += Get-ChildItem -Path $root -Filter 'stratum.exe' -Recurse -File `
            -ErrorAction SilentlyContinue
    }
    if ($found.Count -eq 0) { return $null }
    return ($found | Sort-Object LastWriteTime -Descending | Select-Object -First 1)
}

function Find-UninstallEntry {
    $keys = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    foreach ($key in $keys) {
        $entry = Get-ItemProperty -Path $key -ErrorAction SilentlyContinue |
            Where-Object { $_.DisplayName -and $_.DisplayName -like 'Stratum*' } |
            Select-Object -First 1
        if ($entry) { return $entry }
    }
    return $null
}

function Wait-Removed {
    param([string]$Path, [int]$Seconds = 90)
    for ($i = 0; $i -lt $Seconds; $i++) {
        if (-not (Test-Path $Path)) { return $true }
        Start-Sleep -Seconds 1
    }
    return $false
}

function Uninstall-Product {
    param([string]$InstalledBinary)
    $entry = Find-UninstallEntry
    if (-not $entry) { throw 'no uninstall entry for Stratum was registered' }
    $command = if ($entry.QuietUninstallString) { $entry.QuietUninstallString } else { $entry.UninstallString }
    if (-not $command) { throw 'the uninstall entry carries no uninstall command' }

    # An MSI entry is `MsiExec.exe /X{GUID}`; an NSIS entry points at the
    # uninstaller. The NSIS uninstaller relaunches itself from %TEMP% and
    # returns immediately, so removal is confirmed by polling the payload.
    if ($command -match '\{[0-9A-Fa-f-]{36}\}') {
        $guid = $matches[0]
        Write-Host "    removing product $guid"
        Start-Process -FilePath 'msiexec.exe' `
            -ArgumentList @('/x', $guid, '/qn', '/norestart') -Wait | Out-Null
    } else {
        $uninstaller = $command.Trim()
        if ($uninstaller.StartsWith('"')) {
            $uninstaller = $uninstaller.Substring(1, $uninstaller.IndexOf('"', 1) - 1)
        } elseif ($uninstaller -match '^(\S+\.exe)') {
            $uninstaller = $matches[1]
        }
        Write-Host "    running $uninstaller /S"
        Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait | Out-Null
    }

    if (-not (Wait-Removed -Path $InstalledBinary)) {
        throw "the installed binary $InstalledBinary was still present after uninstalling"
    }
    if (Find-UninstallEntry) {
        throw 'the uninstall entry was still registered after uninstalling'
    }
}

function Invoke-LaunchProbe {
    param([string]$Binary)
    $stdout = Join-Path $env:TEMP 'stratum-launch.out.log'
    $stderr = Join-Path $env:TEMP 'stratum-launch.err.log'
    Remove-Item $stdout, $stderr -ErrorAction SilentlyContinue
    $env:STRATUM_FIXTURE = '1'
    $env:STRATUM_LOCALE = 'en'

    $process = Start-Process -FilePath $Binary -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $exited = $process.WaitForExit($ProbeSeconds * 1000)
    $log = @()
    foreach ($file in @($stdout, $stderr)) {
        if (Test-Path $file) { $log += Get-Content -Path $file -ErrorAction SilentlyContinue }
    }
    $logText = ($log -join "`n")
    $windowNote = if ($logText -match '(?i)failed to open the Stratum shell') {
        ' (no window on this runner)'
    } else { '' }

    if (-not $exited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        if ($logText -match '(?i)panic') {
            throw "the shell panicked while running:`n$logText"
        }
        return "stayed up for ${ProbeSeconds}s$windowNote"
    }

    $code = $process.ExitCode
    if ($logText -match '(?i)panic') {
        throw "the shell panicked (exit $code):`n$logText"
    }
    if ($code -ne 0) {
        throw "the shell exited with $code before the probe window:`n$logText"
    }
    return "exited cleanly (exit 0)${windowNote}"
}

function Test-Installer {
    param([string]$File, [string]$Kind)
    $name = Split-Path $File -Leaf
    Write-Host "==> $name ($Kind)"

    $installed = $null
    try {
        if ($Kind -eq 'msi') {
            $log = Join-Path $env:TEMP 'stratum-msi-install.log'
            Remove-Item $log -ErrorAction SilentlyContinue
            $install = Start-Process -FilePath 'msiexec.exe' `
                -ArgumentList @('/i', "`"$File`"", '/qn', '/norestart', '/l*v', "`"$log`"") `
                -Wait -PassThru
            if ($install.ExitCode -notin @(0, 3010)) {
                throw "msiexec exited with $($install.ExitCode) (log: $log)"
            }
        } else {
            $install = Start-Process -FilePath $File -ArgumentList '/S' -Wait -PassThru
            if ($install.ExitCode -ne 0) {
                throw "the NSIS installer exited with $($install.ExitCode)"
            }
        }

        # The silent installer returns before the file system settles on slow
        # runners, so the payload is polled instead of assumed.
        for ($i = 0; $i -lt 30 -and -not $installed; $i++) {
            $installed = Find-InstalledBinary
            if (-not $installed) { Start-Sleep -Seconds 1 }
        }
        if (-not $installed) { throw 'the installer did not place stratum.exe on disk' }
        Write-Host "    installed: $($installed.FullName)"

        $launchDetail = 'launch check skipped'
        if (-not $SkipLaunch) {
            $launchDetail = "installed + $(Invoke-LaunchProbe -Binary $installed.FullName)"
        } else {
            $launchDetail = 'installed'
        }

        Uninstall-Product -InstalledBinary $installed.FullName
        Record-Result -Name $name -Status 'PASS' -Detail "$launchDetail + uninstalled"
    } catch {
        Record-Result -Name $name -Status 'FAIL' -Detail $_.Exception.Message
        # Leave the machine clean for the next artifact even after a failure.
        if ($installed) {
            try { Uninstall-Product -InstalledBinary $installed.FullName } catch { }
        }
    }
}

$artifacts = @(Get-ChildItem -Path $Dir -File | Where-Object { $_.Extension -in @('.msi', '.exe') })
if ($artifacts.Count -eq 0) {
    Write-Error "no .msi or .exe installer found in $Dir"
    exit 1
}

foreach ($file in $artifacts) {
    $kind = if ($file.Extension -eq '.msi') { 'msi' } else { 'nsis' }
    Test-Installer -File $file.FullName -Kind $kind
}

Write-Host ''
Write-Host '==> results'
foreach ($line in $script:results) { Write-Host "    $line" }

if ($script:failed -ne 0) {
    Write-Host ''
    Write-Host '==> FAILED'
    exit 1
}

Write-Host ''
Write-Host '==> all Windows installers verified'
