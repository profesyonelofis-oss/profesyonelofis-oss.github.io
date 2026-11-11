param([int]$x,[int]$y)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class F {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
}
"@
$p = Get-Process ollama_proje | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "PENCERE_YOK"; exit 1 }
$h = $p.MainWindowHandle
$fg = [F]::GetForegroundWindow()
$ft = [F]::GetWindowThreadProcessId($fg, [IntPtr]::Zero)
$tt = [F]::GetWindowThreadProcessId($h, [IntPtr]::Zero)
[F]::AttachThreadInput($ft, $tt, $true) | Out-Null
[F]::BringWindowToTop($h) | Out-Null
[F]::ShowWindow($h, 9) | Out-Null
[F]::SetForegroundWindow($h) | Out-Null
[F]::AttachThreadInput($ft, $tt, $false) | Out-Null
Start-Sleep -Milliseconds 500
$on = ([F]::GetForegroundWindow() -eq $h)
"on=$on hwnd=$h"
if (-not $on) { "ODAK_ALINAMADI"; exit 2 }
[F]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 150
[F]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[F]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 400
"TIKLANDI $x,$y"
