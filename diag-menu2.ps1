$ErrorActionPreference = 'Stop'
$pages = $null
try { $pages = Invoke-RestMethod 'http://localhost:9222/json' -TimeoutSec 3 } catch {}
if (-not $pages) {
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
  Start-Process -FilePath 'D:\Work\tauri\src-tauri\target\release\toolsplus.exe'
  Start-Sleep -Seconds 8
  $pages = Invoke-RestMethod 'http://localhost:9222/json'
}
$pg = $pages | Where-Object { $_.type -eq 'page' } | Select-Object -First 1
Write-Output "attached: $($pg.url)"

$ws = [System.Net.WebSockets.ClientWebSocket]::new()
$ws.ConnectAsync($pg.webSocketDebuggerUrl, [Threading.CancellationToken]::None).Wait()
$buf = New-Object byte[] 4194304

function Send-Cdp($json) {
  $b = [Text.Encoding]::UTF8.GetBytes($json)
  $ws.SendAsync([ArraySegment[byte]]::new($b), [System.Net.WebSockets.WebSocketMessageType]::Text, $true, [Threading.CancellationToken]::None).Wait()
}
$cts = [System.Threading.CancellationTokenSource]::new()
$cts.CancelAfter(20000)
function Recv-UntilId([int]$id) {
  $marker = '"id":' + $id
  while ($true) {
    $r = $ws.ReceiveAsync([ArraySegment[byte]]::new($buf), $cts.Token).Result
    $m = [Text.Encoding]::UTF8.GetString($buf, 0, $r.Count)
    if ($m.Contains($marker)) { return $m }
  }
}

$js = @'
(() => {
  const out = {};
  const menu = document.querySelector('.win-nav-menu');
  if (!menu) return JSON.stringify({ error: 'no .win-nav-menu' });
  out.menuChildren = menu.children.length;
  out.menuHTMLLen = menu.innerHTML.length;
  out.menuHTMLHead = menu.innerHTML.substring(0, 400);
  const scroll = document.querySelector('.win-nav-left-scrollable');
  if (scroll) {
    out.scrollChildren = scroll.children.length;
    const cs = getComputedStyle(scroll);
    out.scrollStyle = 'w=' + scroll.offsetWidth + ' h=' + scroll.offsetHeight + ' disp=' + cs.display + ' overflow=' + cs.overflow;
  }
  const nav = document.querySelector('nav');
  if (nav) out.navClasses = nav.className;
  return JSON.stringify(out).substring(0, 2000);
})()
'@

$payload = '{"id":100,"method":"Runtime.evaluate","params":{"expression":' + ($js | ConvertTo-Json) + ',"returnByValue":true}}'
Send-Cdp $payload
$resp = Recv-UntilId 100
if ($resp.Length -gt 2500) { $resp.Substring(0, 2500) } else { $resp }
$ws.Dispose()
