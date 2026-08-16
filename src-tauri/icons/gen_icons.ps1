Add-Type -AssemblyName System.Drawing

$blue = [System.Drawing.Color]::FromArgb(255, 0, 120, 212)
$transparent = [System.Drawing.Color]::FromArgb(0, 0, 0, 0)

function New-ToolboxBitmap([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceOver

    $brush = New-Object System.Drawing.SolidBrush($blue)

    $script:s = $size
    function P([float]$x, [float]$y) {
        return New-Object System.Drawing.PointF(($x / 24 * $script:s), ($y / 24 * $script:s))
    }

    # 主体盒子：2,8 -> 22,20
    $body = @((P 2 8), (P 22 8), (P 22 20), (P 2 20))
    $g.FillPolygon($brush, $body)

    # 顶部把手：9,6 -> 15,8
    $handle = @((P 9 6), (P 15 6), (P 15 8), (P 9 8))
    $g.FillPolygon($brush, $handle)

    # 挖出两条槽（用透明色 SourceCopy 模式）
    $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
    $clearBrush = New-Object System.Drawing.SolidBrush($transparent)
    $slot1 = @((P 6 10), (P 8 10), (P 8 18), (P 6 18))
    $slot2 = @((P 16 10), (P 18 10), (P 18 18), (P 16 18))
    $g.FillPolygon($clearBrush, $slot1)
    $g.FillPolygon($clearBrush, $slot2)

    $brush.Dispose()
    $clearBrush.Dispose()
    $g.Dispose()
    return $bmp
}

# 生成各尺寸
$files = @{
    '32x32.png' = 32
    '128x128.png' = 128
    '128x128@2x.png' = 256
    'Square30x30Logo.png' = 30
    'Square44x44Logo.png' = 44
    'Square71x71Logo.png' = 71
    'Square89x89Logo.png' = 89
    'Square107x107Logo.png' = 107
    'Square142x142Logo.png' = 142
    'Square150x150Logo.png' = 150
    'Square284x284Logo.png' = 284
    'Square310x310Logo.png' = 310
    'StoreLogo.png' = 50
    'icon.png' = 256
}

foreach ($kv in $files.GetEnumerator()) {
    $bmp = New-ToolboxBitmap $kv.Value
    $bmp.Save($kv.Key, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Host "Generated $($kv.Key) ($($kv.Value)x$($kv.Value))"
}

Write-Host "All PNG icons generated!"
