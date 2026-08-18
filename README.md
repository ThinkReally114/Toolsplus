# ToolsPlus

轻量、快速的 Windows 工具集成箱，基于 [WinUI on Web](http://github.com/Furry-Xiyi/WinUIonWeb) 项目构建。

**技术栈：Tauri v2 + Vue 3 + Rust**

**本项目完全开源免费，欢迎加入官方 QQ 讨论群反馈：831977964**

> 运行时会请求管理员权限，这是扫描系统垃圾、读取硬件信息等功能的必要权限，请同意。

---

## 运行环境要求

| 依赖 | 最低版本 | 说明 |
|---|---|---|
| Windows | 10 1809 (build 17763) | 仅 x64，依赖 WinRT API 集 |
| WebView2 Runtime | 最新版 | Win11 自带，Win10 可能需手动安装 |
| VC++ Redistributable | 2015-2022 (x64) | 提供 `VCRUNTIME140.dll` |

### 构建环境（额外要求）

| 工具 | 实测版本 | 用途 |
|---|---|---|
| Node.js | v24.19.0 | 前端构建 |
| npm | 11.17.0 | 依赖管理 |
| Rust | 1.97.1 (stable-msvc) | 后端编译 |
| VS 2022 BuildTools | MSVC 14.44 | C/C++ 工具链 |
| Windows SDK | 10.0.26100 | 系统头文件/库 |
| Tauri CLI | v2 | 打包框架 |
| WinUI on Web | submodule | UI 组件库 |

---

## 快速开始

### 1. 克隆仓库（含子模块）

```powershell
git clone --recursive <repo-url>
# 若已克隆但未带子模块：
git submodule update --init --recursive
```

### 2. 安装前端依赖

```powershell
npm install --legacy-peer-deps
```

> `--legacy-peer-deps` 是为了满足 WinUI on Web 子模块的 tsconfig references 依赖。

### 3. 开发模式（热更新）

```powershell
# 确保 Rust 在 PATH（新终端会话需执行）
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

npm run tauri dev
```

### 4. 构建发布版

```powershell
# 在仓库根目录执行
npm run tauri build
```

该命令会自动完成：

1. **`npm run build`**（beforeBuildCommand）
   - `vue-tsc --noEmit`：TypeScript 类型检查
   - `vite build`：前端打包到 `dist/`
2. **`cargo build --release`**：Rust 编译（目标 `x86_64-pc-windows-msvc`）
3. **`build.rs`**：嵌入 manifest（UAC 提权、DPI 感知、Common Controls 6.0）
4. **打包安装包**：生成 NSIS + MSI

---

## 📁 构建产物

```
src-tauri/target/release/
├─ toolsplus.exe                          # 主程序（约 10.4 MB，可直接运行）
├─ toolsplus.pdb                          # 调试符号
├─ toolsplus_lib.dll                      # 动态库
└─ bundle/
   ├─ nsis/ToolsPlus_0.1.0_x64-setup.exe  # NSIS 安装包
   └─ msi/ToolsPlus_0.1.0_x64_en-US.msi   # MSI 安装包
```

> ⏱️ 首次编译约 5~12 分钟（需编译所有 Rust crate 依赖），增量编译会快很多