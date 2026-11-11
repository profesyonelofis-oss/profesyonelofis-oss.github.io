@echo off
cd /d "%~dp0"
set "EXE=target\debug\ollama_proje.exe"
if not exist "%EXE%" (
  echo EXE bulunamadi: %EXE%
  pause
  exit /b 1
)
start "" "%EXE%" --tedarik-panel-gui
exit
