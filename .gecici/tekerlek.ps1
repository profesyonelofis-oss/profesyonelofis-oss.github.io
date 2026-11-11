param([int]$x = 1000, [int]$y = 700, [int64]$miktar = -720, [int]$adet = 2)
$d = [uint32](4294967296 + $miktar)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Teker {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
}
"@
[Teker]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 300
for ($i = 0; $i -lt $adet; $i++) {
  [Teker]::mouse_event(0x0800, 0, 0, $d, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 350
}
"tekerlek tamam"
