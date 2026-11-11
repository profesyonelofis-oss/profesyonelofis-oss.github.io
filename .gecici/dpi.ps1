Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class D {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr p);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
[D]::SetProcessDPIAware() | Out-Null
Start-Sleep -Milliseconds 200
$p = Get-Process ollama_proje | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$h = $p.MainWindowHandle
$fg = [D]::GetForegroundWindow()
$ft = [D]::GetWindowThreadProcessId($fg, [IntPtr]::Zero)
$tt = [D]::GetWindowThreadProcessId($h, [IntPtr]::Zero)
[D]::AttachThreadInput($ft, $tt, $true) | Out-Null
[D]::BringWindowToTop($h) | Out-Null
[D]::ShowWindow($h, 9) | Out-Null
[D]::SetForegroundWindow($h) | Out-Null
[D]::AttachThreadInput($ft, $tt, $false) | Out-Null
Start-Sleep -Milliseconds 600
$r = New-Object D+RECT
[D]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
"rect=$($r.Left),$($r.Top) ${w}x${hh} on=$([D]::GetForegroundWindow() -eq $h)"
# 2 Medya sekmesi: pencere genişliğinin %10'u, yüksekliğinin %5.9'u
$cx = $r.Left + [int]($w * 0.100)
$cy = $r.Top + [int]($hh * 0.059)
[D]::SetCursorPos($cx, $cy) | Out-Null
Start-Sleep -Milliseconds 250
[D]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 120
[D]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 900
"tiklandi=$cx,$cy"
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[D]::PrintWindow($h, $hdc, 2) | Out-Null
$g.ReleaseHdc($hdc)
$bmp.Save("D:\ollama_proje\.gecici\ekran.png", [System.Drawing.Imaging.ImageFormat]::Png)
$kw = 1100; $kh = [int]($hh * $kw / $w)
$kb = New-Object System.Drawing.Bitmap $kw, $kh
$kg = [System.Drawing.Graphics]::FromImage($kb)
$kg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$kg.DrawImage($bmp, 0, 0, $kw, $kh)
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
$ep = New-Object System.Drawing.Imaging.EncoderParameters 1
$ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 88
$kb.Save("D:\ollama_proje\.gecici\ekran_kucuk.jpg", $codec, $ep)
$bmp.Dispose(); $kb.Dispose()
"HAZIR"
