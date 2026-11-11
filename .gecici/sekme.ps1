param([double]$fx,[double]$fy)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Ms4 {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X,int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,UIntPtr e);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$p = Get-Process ollama_proje | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "PENCERE_YOK"; exit 1 }
$h = $p.MainWindowHandle
[Ms4]::ShowWindow($h, 9) | Out-Null
# ALT: ön plan hakkı verir, SetForegroundWindow engeli aşılır.
[Ms4]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
[Ms4]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 120
[Ms4]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 450
$on = ([Ms4]::GetForegroundWindow() -eq $h)
$r = New-Object Ms4+RECT
[Ms4]::GetWindowRect($h, [ref]$r) | Out-Null
$gen = $r.Right - $r.Left
$yuk = $r.Bottom - $r.Top
$kose = New-Object Ms4+POINT
$kose.X = 0; $kose.Y = 0
[Ms4]::ClientToScreen($h, [ref]$kose) | Out-Null
if ($gen -lt 200) { $kose.X = $r.Left; $kose.Y = $r.Top; $gen = $r.Right - $r.Left; $yuk = $r.Bottom - $r.Top }
$x = [int]($kose.X + $gen * $fx)
$y = [int]($kose.Y + $yuk * $fy)
[Ms4]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 200
[Ms4]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 90
[Ms4]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
"tiklandi $x,$y on=$on boyut=$gen x $yuk kose=$($kose.X),$($kose.Y)"
