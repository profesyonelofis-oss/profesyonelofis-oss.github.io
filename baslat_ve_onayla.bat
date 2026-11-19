@echo off
chcp 65001 >nul
rem NEXUS medya onay paneli - tek tik baslatici.
rem Cift tiklayinca paneli arka planda baslatir; panel hazir olur olmaz
rem varsayilan tarayici http://127.0.0.1:8080 adresini kendisi acar.
setlocal
cd /d "%~dp0"
set "EXE=%~dp0ollama_proje.exe"
if not exist "%EXE%" set "EXE=%~dp0target\release\ollama_proje.exe"
if not exist "%EXE%" (
  echo [HATA] ollama_proje.exe bulunamadi.
  echo Once derleyin: cargo build --release
  pause
  exit /b 1
)
start "" /min "%EXE%" --onizleme
rem Pencere simge durumunda kalir; kapatmak icin gorev cubugundan kapatmaniz yeter.
endlocal
