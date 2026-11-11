Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class W {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$p = Get-Process ollama_proje | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { "PENCERE_YOK"; exit 1 }
$h = $p.MainWindowHandle
if ([W]::IsIconic($h)) { [W]::ShowWindow($h, 9) | Out-Null; Start-Sleep -Milliseconds 600 }
$r = New-Object W+RECT
[W]::GetWindowRect($h, [ref]$r) | Out-Null
$w = $r.Right - $r.Left; $hh = $r.Bottom - $r.Top
function Ortalama($bmp, $w, $hh) {
  $toplam = 0.0; $say = 0
  for ($x = 10; $x -lt $w; $x += 60) { for ($y = 10; $y -lt $hh; $y += 60) { $c = $bmp.GetPixel($x, $y); $toplam += ($c.R + $c.G + $c.B) / 3; $say++ } }
  return [int]($toplam / $say)
}
$bmp = New-Object System.Drawing.Bitmap $w, $hh
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [W]::PrintWindow($h, $hdc, 2)
$g.ReleaseHdc($hdc)
$parlak = Ortalama $bmp $w $hh
"printwindow=$ok rect=${w}x${hh} ortalama=$parlak on=$([W]::GetForegroundWindow() -eq $h)"
if ($parlak -lt 12) {
  $bmp.Dispose()
  [W]::SetForegroundWindow($h) | Out-Null
  Start-Sleep -Milliseconds 900
  $bmp = New-Object System.Drawing.Bitmap $w, $hh
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
  $parlak = Ortalama $bmp $w $hh
  "ekran_yakalama ortalama=$parlak on=$([W]::GetForegroundWindow() -eq $h)"
}
$bmp.Save("D:\ollama_proje\.gecici\ekran.png", [System.Drawing.Imaging.ImageFormat]::Png)
$kw = 1100; $kh = [int]($hh * $kw / $w)
$kb = New-Object System.Drawing.Bitmap $kw, $kh
$kg = [System.Drawing.Graphics]::FromImage($kb)
$kg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$kg.DrawImage($bmp, 0, 0, $kw, $kh)
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
$ep = New-Object System.Drawing.Imaging.EncoderParameters 1
$ep.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 85
$kb.Save("D:\ollama_proje\.gecici\ekran_kucuk.jpg", $codec, $ep)
$bmp.Dispose(); $kb.Dispose()
"HAZIR"
