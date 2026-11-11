param([int]$x,[int]$y)
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Ms { [DllImport("user32.dll")] public static extern bool SetCursorPos(int X,int Y); [DllImport("user32.dll")] public static extern void mouse_event(uint f,uint dx,uint dy,uint d,UIntPtr e); }
"@
[Ms]::SetCursorPos($x,$y) | Out-Null
Start-Sleep -Milliseconds 150
[Ms]::mouse_event(0x0002,0,0,0,[UIntPtr]::Zero)
Start-Sleep -Milliseconds 80
[Ms]::mouse_event(0x0004,0,0,0,[UIntPtr]::Zero)
