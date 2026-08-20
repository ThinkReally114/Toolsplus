 1→import{r as g,w as v}from"./index-hpeq328-.js";const _=new Set(["FromRight","FromLeft"]),y=new Set(["NavigationTrigger_NavigatingTo","NavigationTrigger_NavigatingAway","NavigationTrigger_BackNavigatingTo","NavigationTrigger_BackNavigatingAway"]),c="NavigationTrigger_NavigatingTo",Y="NavigationTrigger_NavigatingAway",U="NavigationTrigger_BackNavigatingTo",K="NavigationTrigger_BackNavigatingAway",N=()=>({Type:"EntranceNavigationTransitionInfo"}),C=()=>({Type:"DrillInNavigationTransitionInfo"}),E=()=>({Type:"SuppressNavigationTransitionInfo"}),F=()=>({Type:"CommonNavigationTransitionInfo"}),A=()=>({Type:"ContinuumNavigationTransitionInfo"}),k=(n="FromRight")=>({Type:"SlideNavigationTransitionInfo",Effect:_.has(n)?n:"FromRight"}),d=null,s=n=>{if(!n)return d;const{Type:t}=n;return t==="EntranceNavigationTransitionInfo"?N():t==="DrillInNavigationTransitionInfo"?C():t==="SuppressNavigationTransitionInfo"?E():t==="CommonNavigationTransitionInfo"?F():t==="ContinuumNavigationTransitionInfo"?A():t==="SlideNavigationTransitionInfo"?k(n.Effect):d},L=(n,t=N())=>{if(!n)return s(t);try{return s(JSON.parse(n))}catch{return s(t)}},w=n=>JSON.stringify(s(n)),Z=(n,t)=>w(n)===w(t),M=(n=c)=>{const t=String(n??"").trim();if(y.has(t))return t;const o=`NavigationTrigger_${t}`;return y.has(o)?o:c},G=(n,t=c)=>{const o=s(n),i=M(t);return o?o.Type==="SlideNavigationTransitionInfo"?`SlideNavigationTransitionInfo ${o.Effect} ${i}`:`${o.Type} ${i}`:`DefaultNavigationTransitionInfo ${i}`},T="toolsplus-font",I="toolsplus-antialias",r="toolsplus-font-style",l={system:"system-ui, 'Segoe UI', 'Microsoft YaHei UI', sans-serif",segoe:"'Segoe UI', 'Microsoft YaHei UI', sans-serif",yahei:"'Microsoft YaHei UI', 'Microsoft YaHei', sans-serif",simsun:"'SimSun', 'NSimSun', serif",kaiti:"'KaiTi', 'STKaiti', cursive",consolas:"'Consolas', 'Cascadia Code', 'JetBrains Mono', monospace"},b={auto:"auto",antialiased:"antialiased","subpixel-antialiased":"subpixel-antialiased",none:"none"};function O(){const n=localStorage.getItem(T);return n&&n in l?n:"system"}function $(){const n=localStorage.getItem(I);return n&&n in b?n:"auto"}const m=g(O()),u=g($());function B(){let n=document.getElementById(r);return n||(n=document.createElement("style"),n.id=r,document.head.appendChild(n)),n}const D="'Segoe Fluent Icons', 'Segoe MDL2 Assets', 'WinUIOnWebIcons', 'Segoe UI Symbol'";function f(n){const t=l[n]||l.system,o=document.documentElement,i=document.body;o.style.setProperty("--app-font-family",t),o.style.setProperty("--ContentControlThemeFontFamily",t),i&&(i.style.setProperty("--app-font-family",t),i.style.setProperty("--ContentControlThemeFontFamily",t));const e=`
 2→html, body, #app, .app-root, .app-shell,
 3→.win-text-block, .win-text, .win-button, .win-button-content,
 4→.win-nav-item-content, .win-settings-card, .win-expander-content,
 5→.win-combo-box, .win-text-box, .win-radio-button, .win-check-box,
 6→.win-content-dialog-title, .win-content-dialog-body,
 7→.win-menu-flyout-item-text, .win-list-view-item-content,
 8→.win-breadcrumb-bar-item, .win-pivot-header, .win-info-bar-content,
 9→.win-grid-view-item-content, .win-hyperlink-button {
10→  font-family: ${t} !important;
11→}
12→.win-expander-header-icon, .win-settings-card-icon,
13→.win-expander-header-icon *, .win-settings-card-icon *,
14→.symbol-icon, .appbar-button-chevron,
15→.win-symbol-icon, .win-font-icon, .win-icon,
16→.win-pivot-header-icon, .win-info-bar-icon,
17→.win-menu-flyout-icon, .win-list-view-item-icon,
18→.win-breadcrumb-bar-icon, .win-command-bar-icon {
19→  font-family: ${D} !important;
20→}
21→`;B().textContent=e}function p(n){const t=document.documentElement,o=document.body,i=b[n]||"auto",e=n==="none"?"optimizeSpeed":"optimizeLegibility";t.style.setProperty("-webkit-font-smoothing",i),t.style.setProperty("--app-font-smoothing",i),t.style.textRendering=e,o&&(o.style.setProperty("-webkit-font-smoothing",i),o.style.setProperty("--app-font-smoothing",i),o.style.textRendering=e);const x=`
22→html, body, #app, .app-root, .app-shell,
23→.win-text-block, .win-text, .win-button, .win-button-content,
24→.win-nav-item-content, .win-settings-card, .win-expander-content,
25→.win-combo-box, .win-text-box, .win-radio-button, .win-check-box,
26→.win-content-dialog-title, .win-content-dialog-body,
27→.win-menu-flyout-item-text, .win-list-view-item-content,
28→.win-breadcrumb-bar-item, .win-pivot-header, .win-info-bar-content,
29→.win-grid-view-item-content, .win-hyperlink-button {
30→  -webkit-font-smoothing: ${i} !important;
31→  text-rendering: ${e} !important;
32→}
33→`;let a=document.getElementById(r+"-aa");a||(a=document.createElement("style"),a.id=r+"-aa",document.head.appendChild(a)),a.textContent=x}function H(n){m.value=n,localStorage.setItem(T,n),f(n)}function V(n){u.value=n,localStorage.setItem(I,n),p(n)}function q(){f(m.value),p(u.value)}v(m,n=>f(n));v(u,n=>p(n));const h="toolsplus-webview-zoom";function P(){const n=Number(localStorage.getItem(h));return isNaN(n)||n===0?100:Math.min(200,Math.max(50,n))}const S=g(P());function z(n){const t=Math.min(200,Math.max(50,n||100));S.value=t,localStorage.setItem(h,String(t));const o=t/100;document.documentElement.style.setProperty("--app-zoom",String(o));const i=document.querySelector(".app-shell");i&&(i.style.zoom=String(o),i.style.transform="",i.style.width="",i.style.height="")}function J(){z(S.value)}export{d as D,U as N,k as a,C as b,N as c,E as d,F as e,A as f,G as g,J as h,q as i,c as j,K as k,Y as l,z as m,s as n,Z as o,L as p,m as q,H as r,w as s,u as t,V as u,S as w};