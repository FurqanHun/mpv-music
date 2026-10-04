param (
    [switch]$Dev,
    [switch]$Update,
    [string]$Tag = "",
    [switch]$NoVerify
)

$RepoOwner = "FurqanHun"
$RepoName = "mpv-music"

if ($Update) {
    Write-Host "🎧 mpv-music Updater" -ForegroundColor Cyan
} else {
    Write-Host "🎧 mpv-music PowerShell Installer" -ForegroundColor Cyan
}

$DefaultInstallDir = Join-Path $env:LOCALAPPDATA "mpv-music"

if (Get-Command mpv-music -ErrorAction SilentlyContinue) {
    $ExistingPath = (Get-Command mpv-music).Source
    $InstallDir = Split-Path $ExistingPath
    if (-not $Update) {
        Write-Host "[OK] Using existing installation directory: $InstallDir" -ForegroundColor Green
    }
} else {
    if (-not $Update) {
        Write-Host "`nWhere would you like to install the binary?"
        $UserInput = Read-Host "Installation directory [$DefaultInstallDir]"
        if (-not [string]::IsNullOrWhiteSpace($UserInput)) {
            $InstallDir = $UserInput
        } else {
            $InstallDir = $DefaultInstallDir
        }
    } else {
        $InstallDir = $DefaultInstallDir
    }
}

# Expand environment variables and resolve path
$InstallDir = [System.Environment]::ExpandEnvironmentVariables($InstallDir)
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}

$InstalledBinary = Join-Path $InstallDir "mpv-music.exe"

$ArchPlatform = "x86_64-pc-windows-msvc"

# --- Fetch Release and Asset ---
if (-not [string]::IsNullOrWhiteSpace($Tag)) {
    if (-not $Update) {
        Write-Host "`n[INFO] Using provided tag: $Tag" -ForegroundColor Cyan
    } else {
        Write-Host "`n[INFO] Fetching update: $Tag" -ForegroundColor Cyan
    }
    $LatestTag = $Tag
} else {
    Write-Host "`n[INFO] Fetching release info..." -ForegroundColor Cyan
    
    # Use TLS 1.2
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    
    $ApiEndpoint = "https://furqanhun.github.io/mpv-music/latest.json"
    $Releases = Invoke-RestMethod -Uri $ApiEndpoint
    
    if ($Dev) {
        $LatestTag = $Releases.dev.tag_name
    } else {
        $LatestTag = $Releases.stable.tag_name
    }
    
    if ([string]::IsNullOrWhiteSpace($LatestTag)) {
        Write-Host "[ERROR] Failed to parse latest version from $ApiEndpoint." -ForegroundColor Red
        exit 1
    }
}

$AssetName = "mpv-music-$LatestTag-$ArchPlatform.zip"
$AssetUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$LatestTag/$AssetName"
$ChecksumUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$LatestTag/checksums.txt"

Write-Host "[OK] Target version: $ArchPlatform ($LatestTag)" -ForegroundColor Green

$TempDir = Join-Path -Path ([System.IO.Path]::GetTempPath()) -ChildPath ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $TempDir | Out-Null

$ZipPath = Join-Path $TempDir $AssetName
$ChecksumPath = Join-Path $TempDir "checksums.txt"

Write-Host "[INFO] Downloading binary..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $AssetUrl -OutFile $ZipPath -UseBasicParsing -ErrorAction Stop
} catch {
    Write-Host "[ERROR] Failed to download binary. Are you connected to the internet, and does this release exist?" -ForegroundColor Red
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
    exit 1
}

if (-not $NoVerify) {
    Write-Host "[INFO] Downloading checksums..." -ForegroundColor Cyan
    try {
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumPath -UseBasicParsing -ErrorAction Stop
        Write-Host "[INFO] Verifying checksum..." -ForegroundColor Cyan
        
        $ExpectedHashLine = Select-String -Path $ChecksumPath -Pattern $AssetName | Select-Object -ExpandProperty Line
        if (-not $ExpectedHashLine) {
            Write-Host "[ERROR] Checksum for $AssetName not found in checksums.txt!" -ForegroundColor Red
            Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
            exit 1
        }
        
        $ExpectedHash = $ExpectedHashLine.Split(' ')[0].Trim()
        
        $ActualHash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash
        
        if ($ActualHash -eq $ExpectedHash) {
            Write-Host "[OK] Checksum verified." -ForegroundColor Green
        } else {
            Write-Host "[ERROR] Checksum validation failed! The download may be corrupted." -ForegroundColor Red
            Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
            exit 1
        }
    } catch {
        Write-Host "[WARN] checksums.txt not found on release. Skipping validation." -ForegroundColor Yellow
    }
} else {
    Write-Host "[WARN] Checksum validation disabled via -NoVerify." -ForegroundColor Yellow
}

Write-Host "[INFO] Extracting..." -ForegroundColor Cyan
Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

$ExtractedExe = Get-ChildItem -Path $TempDir -Filter "mpv-music.exe" -Recurse | Select-Object -First 1

if ($ExtractedExe) {
    if (Test-Path $InstalledBinary) {
        $OldBinary = "$InstalledBinary.old"
        if (Test-Path $OldBinary) {
            Remove-Item -Path $OldBinary -Force -ErrorAction SilentlyContinue
        }
        Rename-Item -Path $InstalledBinary -NewName "mpv-music.exe.old" -Force -ErrorAction SilentlyContinue
        
        $CleanupCmd = "Start-Sleep -Seconds 3; Remove-Item -Path '$OldBinary' -Force -ErrorAction SilentlyContinue"
        Start-Process -FilePath "powershell.exe" -WindowStyle Hidden -ArgumentList "-NoProfile", "-Command", $CleanupCmd
    }
    Move-Item -Path $ExtractedExe.FullName -Destination $InstalledBinary -Force
    Write-Host "[OK] Extracted and installed successfully." -ForegroundColor Green
} else {
    Write-Host "[ERROR] mpv-music.exe not found in the downloaded archive." -ForegroundColor Red
    Remove-Item -Path $TempDir -Recurse -Force
    exit 1
}

Remove-Item -Path $TempDir -Recurse -Force

if ($Update) {
    Write-Host "[OK] mpv-music successfully updated in $InstalledBinary" -ForegroundColor Green
} else {
    Write-Host "[OK] mpv-music installed to $InstalledBinary" -ForegroundColor Green
}

# --- Initial Configuration ---
if (-not $Update) {
    Write-Host "`n[INFO] Initial Setup" -ForegroundColor Cyan
    $SetupChoice = Read-Host "Would you like to add music directories now? [y/N]"
    
    if ($SetupChoice -match '^[yY]') {
        $CollectedPaths = @()
        while ($true) {
            Write-Host "`nEnter full path (or ENTER to finish):"
            $MusicPath = Read-Host ">"
            if ([string]::IsNullOrWhiteSpace($MusicPath)) {
                break
            }
            
            $CleanPath = $MusicPath.Trim().Trim('"').Trim("'")
            if (Test-Path $CleanPath -PathType Container) {
                $CollectedPaths += $CleanPath
                Write-Host "[QUEUED] $CleanPath" -ForegroundColor Green
            } else {
                Write-Host "[ERROR] Directory not found: $CleanPath" -ForegroundColor Red
            }
        }
        
        if ($CollectedPaths.Count -gt 0) {
            & $InstalledBinary --add-dir $CollectedPaths
        }
    }
}

# --- PATH Verification ---
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$SysPath = [Environment]::GetEnvironmentVariable("PATH", "Machine")

if (($UserPath -notmatch [regex]::Escape($InstallDir)) -and ($SysPath -notmatch [regex]::Escape($InstallDir))) {
    Write-Host "`n[WARNING] $InstallDir is not in your PATH." -ForegroundColor Yellow
    Write-Host "Adding it to your User PATH automatically..." -ForegroundColor Cyan
    
    $NewPath = "$UserPath;$InstallDir"
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    
    Write-Host "[OK] PATH updated! You may need to restart your terminal for the changes to take effect." -ForegroundColor Green
}

if ($Update) {
    Write-Host "`nUpdate complete! You are now running the latest version." -ForegroundColor Green
} else {
    Write-Host "`nInstallation complete! Run 'mpv-music' to start." -ForegroundColor Green
}
