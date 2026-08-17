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
  const el = document.querySelector('.win-nav-left-panel');
  if (!el) return 'no left panel';
  let inst = el.__vueParentComponent;
  let depth = 0;
  const chain = [];
  while (inst && depth < 8) {
    const name = inst.type?.name || inst.type?.__name || (typeof inst.type === 'object' ? 'anon' : String(inst.type).substring(0,30));
    const props = inst.props || {};
    const propKeys = Object.keys(props).filter(k => /menu/i.test(k));
    const menuInfo = propKeys.map(k => {
      const v = props[k];
      const isArr = Array.isArray(v);
      return k + '=' + (isArr ? '[' + v.length + ' items]' : typeof v);
    });
    chain.push(depth + ': ' + name + ' | ' + (menuInfo.join(', ') || '(no menu props)'));
    if (propKeys.length && Array.isArray(props[propKeys[0]]) && propKeys[0] === 'MenuItems') {
      out.receivedMenuItems = props.MenuItems.map(i => ({
        tag: i.Tag, content: i.Content, childCount: Array.isArray(i.MenuItems) ? i.MenuItems.length : 0
      }));
    }
    inst = inst.parent;
    depth++;
  }
  out.chain = chain;
  return JSON.stringify(out).substring(0, 3000);
})()
'@

$payload = '{"id":80,"method":"Runtime.evaluate","params":{"expression":' + ($js | ConvertTo-Json) + ',"returnByValue":true}}'
Send-Cdp $payload
$resp = Recv-UntilId 80
if ($resp.Length -gt 3500) { $resp.Substring(0, 3500) } else { $resp }
$ws.Dispose()
