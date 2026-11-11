param(
    [Parameter(Position=0)][string]$Action = "info",
    [Parameter(Position=1)][int]$X = 0,
    [Parameter(Position=2)][int]$Y = 0,
    [Parameter(Position=3)][string]$Path = ".gecici/gui_shot.png"
)

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class NexusWin {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h, int n);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
    [DllImport("user32.dll")] public static extern int GetWindowLong(IntPtr h, int index);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public struct POINT { public int X, Y; }
}
"@

Add-Type -AssemblyName System.Drawing

$p = Get-Process ollama_proje -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $p) { Write-Output "NO_PROCESS"; exit 1 }
$h = $p.MainWindowHandle
$wr = New-Object NexusWin+RECT
$cr = New-Object NexusWin+RECT
$origin = New-Object NexusWin+POINT
[NexusWin]::GetWindowRect($h, [ref]$wr) | Out-Null
[NexusWin]::GetClientRect($h, [ref]$cr) | Out-Null
[NexusWin]::ClientToScreen($h, [ref]$origin) | Out-Null
$clientDx = $origin.X - $wr.Left
$clientDy = $origin.Y - $wr.Top
$style = [NexusWin]::GetWindowLong($h, -20)

function Bring-App {
    [NexusWin]::ShowWindowAsync($h, 9) | Out-Null
    Start-Sleep -Milliseconds 600
    [NexusWin]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040) | Out-Null
    [NexusWin]::SetForegroundWindow($h) | Out-Null
    Start-Sleep -Milliseconds 1400
    [NexusWin]::GetWindowRect($h, [ref]$wr) | Out-Null
    [NexusWin]::GetClientRect($h, [ref]$cr) | Out-Null
    [NexusWin]::ClientToScreen($h, [ref]$origin) | Out-Null
    $script:clientDx = $origin.X - $wr.Left
    $script:clientDy = $origin.Y - $wr.Top
}

function Capture-App($dest) {
    $w = $wr.Right - $wr.Left
    $hh = $wr.Bottom - $wr.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $hh
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($wr.Left, $wr.Top, 0, 0, $bmp.Size)
    $g.Dispose()
    $bmp.Save((Join-Path (Get-Location) $dest), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

switch ($Action) {
    "info" {
        Write-Output "PID=$($p.Id) HWND=$h TITLE=$($p.MainWindowTitle)"
        Write-Output "WINDOW=$($wr.Left),$($wr.Top),$($wr.Right-$wr.Left),$($wr.Bottom-$wr.Top)"
        Write-Output "CLIENT=$($cr.Right),$($cr.Bottom) OFFSET=$clientDx,$clientDy TOPMOST=$([bool]($style -band 0x8))"
    }
    "click" {
        # X,Y are screenshot/window-relative; convert to client-relative.
        Bring-App
        $cx = $X - $script:clientDx
        $cy = $Y - $script:clientDy
        $lp = [IntPtr](($cy -shl 16) -bor ($cx -band 0xffff))
        [NexusWin]::PostMessage($h, 0x0201, [IntPtr]1, $lp) | Out-Null
        Start-Sleep -Milliseconds 80
        [NexusWin]::PostMessage($h, 0x0202, [IntPtr]0, $lp) | Out-Null
        Write-Output "CLICK window=$X,$Y client=$cx,$cy lparam=$lp"
    }
    "shot" {
        Bring-App
        Capture-App $Path
        Write-Output "SHOT=$Path"
    }
    "bring" {
        Bring-App
        Write-Output "BROUGHT"
    }
    default { Write-Output "UNKNOWN_ACTION" }
}
