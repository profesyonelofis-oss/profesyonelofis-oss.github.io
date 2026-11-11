param([string]$metin)
Add-Type -AssemblyName System.Windows.Forms
Set-Clipboard -Value $metin
Start-Sleep -Milliseconds 250
[System.Windows.Forms.SendKeys]::SendWait("^v")
Start-Sleep -Milliseconds 350
"yapistirildi: $metin"
