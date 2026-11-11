param([string]$eylem = "tikla", [double]$fx = 0.5, [double]$fy = 0.5, [string]$yol = "")
Add-Type @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Drawing;
public class Pencere {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr z, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
  public delegate bool EnumProc(IntPtr h, IntPtr p);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public static IntPtr EnBuyuk(uint[] pidler) {
    IntPtr en = IntPtr.Zero; int alan = 0;
    EnumWindows(delegate(IntPtr h, IntPtr p) {
      if (!IsWindowVisible(h)) return true;
      uint pid; GetWindowThreadProcessId(h, out pid);
      if (Array.IndexOf(pidler, pid) < 0) return true;
      RECT r; if (!GetWindowRect(h, out r)) return true;
      int a = (r.Right - r.Left) * (r.Bottom - r.Top);
      if (a > alan) { alan = a; en = h; }
      return true;
    }, IntPtr.Zero);
    return en;
  }
}
"@ -ReferencedAssemblies System.Drawing
$pidler = @(Get-Process ollama_proje -ErrorAction SilentlyContinue | ForEach-Object { [uint32]$_.Id })
if ($pidler.Count -eq 0) { "PENCERE_YOK"; exit 1 }
$h = [Pencere]::EnBuyuk($pidler)
if ($h -eq [IntPtr]::Zero) { "PENCERE_YOK"; exit 1 }
[Pencere]::ShowWindow($h, 9) | Out-Null
[Pencere]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
[Pencere]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 120
[Pencere]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 500
$r = New-Object Pencere+RECT
[Pencere]::GetWindowRect($h, [ref]$r) | Out-Null
$gen = $r.Right - $r.Left
$yuk = $r.Bottom - $r.Top
if ($eylem -eq "konumlandir") {
  [Pencere]::SetWindowPos($h, [IntPtr]::Zero, 40, 40, 1456, 939, 0x0040) | Out-Null
  Start-Sleep -Milliseconds 400
  "konumlandirildi"
  exit 0
}
if ($eylem -eq "yakala") {
  $bmp = New-Object System.Drawing.Bitmap($gen, $yuk)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
  $bmp.Save($yol, [System.Drawing.Imaging.ImageFormat]::Jpeg)
  $g.Dispose(); $bmp.Dispose()
  "yakalandi $yol ($gen x $yuk) on=$([Pencere]::GetForegroundWindow() -eq $h)"
  exit 0
}
$x = [int]($r.Left + $gen * $fx)
$y = [int]($r.Top + $yuk * $fy)
[Pencere]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 180
[Pencere]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 90
[Pencere]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
"tiklandi $x,$y boyut=$gen x $yuk on=$([Pencere]::GetForegroundWindow() -eq $h)"
