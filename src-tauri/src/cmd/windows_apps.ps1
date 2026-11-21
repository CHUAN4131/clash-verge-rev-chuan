$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$apps = [Collections.Generic.Dictionary[string, object]]::new([StringComparer]::OrdinalIgnoreCase)

function Add-App([string]$name, [string]$path) {
    $path = [Environment]::ExpandEnvironmentVariables($path.Trim().Trim('"'))
    if ($path -notlike '*.exe' -or -not [IO.Path]::IsPathRooted($path) -or -not [IO.File]::Exists($path)) {
        return
    }
    $path = [IO.Path]::GetFullPath($path)
    if ($apps.ContainsKey($path)) {
        return
    }
    if (-not $name) {
        try {
            $name = [Diagnostics.FileVersionInfo]::GetVersionInfo($path).FileDescription
        } catch {
            $name = ''
        }
    }
    if (-not $name) {
        $name = [IO.Path]::GetFileNameWithoutExtension($path)
    }
    $apps.Add($path, @{name = $name; path = $path})
}

function Get-PackageExecutable($item) {
    $directory = $item.ExtendedProperty('System.AppUserModel.PackageInstallPath')
    $applicationId = ($item.Path -split '!', 2)[1]
    if (-not $directory -or -not $applicationId) {
        return
    }
    try {
        [xml]$manifest = [IO.File]::ReadAllText((Join-Path $directory 'AppxManifest.xml'))
    } catch {
        return
    }
    $application = $manifest.Package.Applications.Application |
        Where-Object { $_.Id -eq $applicationId } | Select-Object -First 1
    if ($application.Executable) {
        Join-Path $directory $application.Executable
    }
}

$folder = (New-Object -ComObject Shell.Application).Namespace('shell:AppsFolder')
if (-not $folder) {
    throw 'Windows Apps folder is unavailable.'
}
foreach ($item in $folder.Items()) {
    $target = $item.ExtendedProperty('System.Link.TargetParsingPath')
    if (-not $target) {
        $target = Get-PackageExecutable $item
    }
    Add-App $item.Name $target
}

$shell = New-Object -ComObject WScript.Shell
foreach ($desktop in 'Desktop', 'AllUsersDesktop') {
    $directory = $shell.SpecialFolders.Item($desktop)
    if ($directory) {
        foreach ($link in Get-ChildItem -LiteralPath $directory -File -Filter '*.lnk' -ErrorAction SilentlyContinue) {
            try {
                $target = $shell.CreateShortcut($link.FullName).TargetPath
            } catch {
                continue
            }
            Add-App $link.BaseName $target
        }
    }
}

$appPaths = @(
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\App Paths'
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\App Paths'
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\App Paths'
)
foreach ($root in $appPaths) {
    foreach ($entry in Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue) {
        Add-App '' ($entry.GetValue(''))
    }
}

foreach ($process in Get-Process) {
    if ($process.MainWindowHandle -ne 0) {
        Add-App '' $process.Path
    }
}

ConvertTo-Json -InputObject @($apps.Values) -Compress
