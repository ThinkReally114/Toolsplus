Add-Type -AssemblyName System.Drawing
. "$PSScriptRoot\gen_icons.ps1" | Out-Null

# 生成 ICO 文件（包含多尺寸）
$icon32 = New-ToolboxBitmap 32
$icon64 = New-ToolboxBitmap 64
$icon128 = New-ToolboxBitmap 128
$icon256 = New-ToolboxBitmap 256

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)

# ICO Header
$reserved = 0
$type = 1
$count = 4
$bw.Write([uint16]$reserved)
$bw.Write([uint16]$type)
$bw.Write([uint16]$count)

# 各尺寸的目录条目（每个 16 字节）
$entries = @(
    @{ w = 32; bmp = $icon32 }
    @{ w = 64; bmp = $icon64 }
    @{ w = 128; bmp = $icon128 }
    @{ w = 256; bmp = $icon256 }
)

# 先写目录条目，再写图像数据
$dataOffset = 6 + ($count * 16)
$imageDataList = @()

foreach ($entry in $entries) {
    $pngMs = New-Object System.IO.MemoryStream
    $entry.bmp.Save($pngMs, [System.Drawing.Imaging.ImageFormat]::Png)
    $pngData = $pngMs.ToArray()
    $pngMs.Dispose()

    $w = if ($entry.w -ge 256) { 0 } else { $entry.w }
    $h = $w
    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0)  # color count
    $bw.Write([byte]0)  # reserved
    $bw.Write([uint16]1)  # planes
    $bw.Write([uint16]32)  # bpp
    $bw.Write([uint32]$pngData.Length)
    $bw.Write([uint32]$dataOffset)

    $imageDataList += , $pngData
    $dataOffset += $pngData.Length
}

# 写图像数据
foreach ($data in $imageDataList) {
    $bw.Write($data)
}

$bw.Flush()

# 保存到文件
$icoPath = "$PSScriptRoot\icon.ico"
[System.IO.File]::WriteAllBytes($icoPath, $ms.ToArray())

$ms.Dispose()
$bw.Dispose()
$icon32.Dispose()
$icon64.Dispose()
$icon128.Dispose()
$icon256.Dispose()

Write-Host "Generated icon.ico"
