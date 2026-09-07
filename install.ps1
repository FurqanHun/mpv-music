param (
    [switch]$Dev,
    [switch]$Update,
    [string]$Tag = ""
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
    $AssetUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/$LatestTag/mpv-music-$LatestTag-$ArchPlatform.zip"
} else {
    Write-Host "`n[INFO] Fetching release info..." -ForegroundColor Cyan
    
    # Use TLS 1.2
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    
    if ($Dev) {
        $ApiEndpoint = "https://api.github.com/repos/$RepoOwner/$RepoName/releases"
        $Releases = Invoke-RestMethod -Uri $ApiEndpoint
        $LatestRelease = $Releases[0]
    } else {
        $ApiEndpoint = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
        $LatestRelease = Invoke-RestMethod -Uri $ApiEndpoint
    }

    $LatestTag = $LatestRelease.tag_name
    
    $AssetUrl = ""
    foreach ($asset in $LatestRelease.assets) {
        if ($asset.name -match $ArchPlatform) {
            $AssetUrl = $asset.browser_download_url
            break
        }
    }
}

if (-not [string]::IsNullOrWhiteSpace($AssetUrl)) {
    Write-Host "[OK] Found pre-compiled binary for $ArchPlatform ($LatestTag)" -ForegroundColor Green
    
    $TempDir = Join-Path -Path ([System.IO.Path]::GetTempPath()) -ChildPath ([guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $TempDir | Out-Null
    
    $ZipPath = Join-Path $TempDir "mpv-music.zip"
    
    Write-Host "[INFO] Downloading..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $AssetUrl -OutFile $ZipPath -UseBasicParsing
    
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
} else {
    Write-Host "[ERROR] No pre-compiled binary found for your system." -ForegroundColor Red
    exit 1
}

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
