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
  const out = { hasVueApp: false };
  const appEl = document.getElementById('app');
  const app = appEl && appEl.__vue_app__;
  if (!app) return JSON.stringify({ error: 'no __vue_app__ on #app', keys: appEl ? Object.keys(appEl).filter(k=>k.startsWith('__')) : [] });
  out.hasVueApp = true;
  const visit = (inst, depth) => {
    if (!inst || depth > 6) return;
    const name = inst.type && (inst.type.name || inst.type.__name) || 'anon';
    const props = inst.props || {};
    const menuProps = Object.keys(props).filter(k => /menu/i.test(k));
    if (menuProps.length || /Nav/i.test(name)) {
      const desc = menuProps.map(k => {
        const v = props[k];
        return k + ':' + (Array.isArray(v) ? v.length + 'items' : typeof v);
      }).join(' ');
      out['d' + depth + '_' + name] = desc || '(nav, no menu props)';
      if (Array.isArray(props.MenuItems)) {
        out.menuItemsDetail = props.MenuItems.map(i => ({ Tag: i.Tag, Content: String(i.Content).substring(0,10), children: Array.isArray(i.MenuItems) ? i.MenuItems.length : -1, iconType: typeof i.Icon }));
      }
      if (Array.isArray(props.FooterMenuItems)) {
        out.footerDetail = props.FooterMenuItems.map(i => ({ Tag: i.Tag, iconType: typeof i.Icon }));
      }
    }
    let child = inst.subTree;
    const walkSubtree = (vnode, d) => {
      if (!vnode || d > 10) return;
      if (vnode.component) visit(vnode.component, depth);
      if (Array.isArray(vnode.children)) vnode.children.forEach(c => walkSubtree(c, d+1));
      else if (vnode.children && vnode.children.default) {}
      if (vnode.component) {
        let sub = vnode.component.subTree;
        walkSubtree(sub, d+1);
      }
      if (vnode.shapeFlag && (vnode.shapeFlag & 16) && vnode.children && !Array.isArray(vnode.children)) {
        for (const slot of Object.values(vnode.children)) {
          if (typeof slot === 'function') {}
        }
      }
    };
    walkSubtree(inst.subTree, 0);
  };
  visit(app._instance, 0);
  return JSON.stringify(out).substring(0, 3000);
})()
'@

$payload = '{"id":90,"method":"Runtime.evaluate","params":{"expression":' + ($js | ConvertTo-Json) + ',"returnByValue":true}}'
Send-Cdp $payload
$resp = Recv-UntilId 90
if ($resp.Length -gt 3500) { $resp.Substring(0, 3500) } else { $resp }
$ws.Dispose()
