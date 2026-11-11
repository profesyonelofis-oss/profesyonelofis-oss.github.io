Add-Type -AssemblyName System.Drawing
$src = [System.Drawing.Image]::FromFile("D:\ollama_proje\.gecici\ekran.png")
$w = 1100
$h = [int]($src.Height * $w / $src.Width)
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.DrawImage($src, 0, 0, $w, $h)
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
$p = New-Object System.Drawing.Imaging.EncoderParameters 1
$p.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 82
$bmp.Save("D:\ollama_proje\.gecici\ekran_kucuk.jpg", $codec, $p)
$src.Dispose(); $bmp.Dispose()
Write-Output "KUCULTULDU ${w}x${h}"
