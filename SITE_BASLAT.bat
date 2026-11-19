@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo Saruhanli Oto Kurtarma - yerel onizleme
echo Adres: http://127.0.0.1:8907/
start "" cmd /c "timeout /t 2 >nul & start "" http://127.0.0.1:8907/"
python "site\sunucu.py" --port 8907
pause
