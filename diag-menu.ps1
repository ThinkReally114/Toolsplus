$ErrorActionPreference = 'Stop'
Stop-Process -Name toolsplus -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9222'
Start-Process -FilePath 'D:\Work\tauri\src-tauri\target\release\toolsplus.exe'
Start-Sleep -Seconds 9
$pages = Invoke-RestMethod 'http://localhost:9222/json'
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
(async () => {
  const out = {};
  const panel = document.querySelector('.win-nav-left-panel');
  out.leftPanelExists = !!panel;
  if (panel) {
    out.leftPanelClasses = panel.className;
    out.leftPanelChildren = panel.children.length;
    out.itemCount = panel.querySelectorAll('.win-nav-item').length;
    out.items = [...panel.querySelectorAll('.win-nav-item')].slice(0, 12).map(el => {
      const label = el.querySelector('.label');
      return (el.className.split(' ').slice(0,2).join('.') || '?') + ' => ' + (label ? label.textContent.trim().substring(0, 20) : '(nolabel)');
    });
    const cs = getComputedStyle(panel);
    out.panelStyle = 'w=' + panel.offsetWidth + ' h=' + panel.offsetHeight + ' disp=' + cs.display + ' vis=' + cs.visibility;
    const list = panel.querySelector('.win-nav-menu-list, [class*="list"], [class*="menu"]');
    if (list) out.menuListHTMLLen = list.innerHTML.length;
  }
  out.declinedFlag = localStorage.getItem('toolsplus-admin-declined');
  out.localStorageKeys = Object.keys(localStorage);
  out.tauriInternals = typeof window.__TAURI_INTERNALS__;
  try {
    const admin = await window.__TAURI_INTERNALS__.invoke('is_admin');
    out.isAdmin = admin;
  } catch (e) { out.isAdminErr = String(e); }
  return JSON.stringify(out).substring(0, 3500);
})()
'@

$payload = '{"id":70,"method":"Runtime.evaluate","params":{"expression":' + ($js | ConvertTo-Json) + ',"awaitPromise":true,"returnByValue":true}}'
Send-Cdp $payload
$resp = Recv-UntilId 70
if ($resp.Length -gt 4000) { $resp.Substring(0, 4000) } else { $resp }
$ws.Dispose()
