# Nexus Enterprise - Windows başlangıcına ekle / çıkar
#
# Kullanım:
#   powershell -ExecutionPolicy Bypass -File araclar\baslangica_ekle.ps1
#   powershell -ExecutionPolicy Bypass -File araclar\baslangica_ekle.ps1 -Kaldir
#
# Kısayol, shell:startup klasörüne yazılır; hedef release .exe, çalışma dizini
# exe'nin klasörüdür (uygulama .env ve defter dosyalarını oradan okur).

param([switch]$Kaldir)

$ErrorActionPreference = 'Stop'

$exe = Join-Path $PSScriptRoot '..\target\release\ollama_proje.exe'
$exe = (Resolve-Path $exe).Path
$baslangic = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Startup'
$kisayol = Join-Path $baslangic 'Nexus Enterprise.lnk'

if ($Kaldir) {
    if (Test-Path $kisayol) {
        Remove-Item $kisayol -Force
        "Kaldirildi: $kisayol"
    } else {
        "Kisayol zaten yok: $kisayol"
    }
    exit 0
}

if (-not (Test-Path $baslangic)) {
    New-Item -ItemType Directory -Path $baslangic -Force | Out-Null
}

$kabuk = New-Object -ComObject WScript.Shell
$lnk = $kabuk.CreateShortcut($kisayol)
$lnk.TargetPath = $exe
$lnk.WorkingDirectory = Split-Path $exe -Parent
$lnk.IconLocation = "$exe,0"
$lnk.Description = 'Nexus Enterprise - Isletme Kesif ve Demo Otomasyonu'
$lnk.WindowStyle = 1
$lnk.Save()

"Eklendi: $kisayol"
"Hedef   : $exe"
"Calisma : $(Split-Path $exe -Parent)"
