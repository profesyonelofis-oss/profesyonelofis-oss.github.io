param([double]$ex = 250, [double]$ey = 57, [string]$yol = "")
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Drawing;
public class P2 {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
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
$h = [P2]::EnBuyuk($pidler)
$r = New-Object P2+RECT
[P2]::GetWindowRect($h, [ref]$r) | Out-Null
"rect $($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"
[P2]::ShowWindow($h, 5) | Out-Null
[P2]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 400
$x = [int]($r.Left + $ex)
$y = [int]($r.Top + $ey)
[P2]::SetCursorPos($x, $y) | Out-Null
Start-Sleep -Milliseconds 150
[P2]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
[P2]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
"tiklandi $x,$y"
