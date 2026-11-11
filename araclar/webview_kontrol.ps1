$src = @"
using System;
using System.Text;
using System.Runtime.InteropServices;
public class Win32Enum {
  public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr hWnd, EnumProc cb, IntPtr lParam);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr hWnd, StringBuilder sb, int max);
}
"@
Add-Type -TypeDefinition $src -ErrorAction SilentlyContinue
$proc = Get-Process ollama_proje -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { Write-Output "UYGULAMA_YOK"; exit 1 }
$siniflar = New-Object System.Collections.Generic.List[string]
$cb = [Win32Enum+EnumProc]{
  param($h, $l)
  $sb = New-Object System.Text.StringBuilder 256
  [void][Win32Enum]::GetClassName($h, $sb, 256)
  $siniflar.Add($sb.ToString())
  return $true
}
[void][Win32Enum]::EnumChildWindows($proc.MainWindowHandle, $cb, [IntPtr]::Zero)
Write-Output ("PENCERE=" + $proc.MainWindowTitle)
$siniflar | Group-Object | ForEach-Object { "ALT_PENCERE " + $_.Name + " x" + $_.Count }
