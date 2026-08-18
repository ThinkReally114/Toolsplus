// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::command;

/// 扫描结果项
#[derive(serde::Serialize)]
struct JunkItem {
    key: String,
    path: String,
    size: u64,
}

/// 清理目标项（从前端传入）
#[derive(serde::Deserialize)]
struct CleanTarget {
    key: String,
    path: String,
    size: u64,
}

/// 递归计算目录大小（字节数），遇到无权限或不存在返回 0
fn dir_size(path: &Path) -> u64 {
    const MAX_DEPTH: u32 = 32;
    const MAX_ENTRIES: usize = 5_000_000;
    if !path.exists() {
        return 0;
    }
    let total = AtomicU64::new(0);
    let visited = AtomicU64::new(0);
    let stack = vec![path.to_path_buf()];
    let mut current = stack;
    let mut depth_stack = vec![0u32];
    let mut top_subdirs: Vec<std::path::PathBuf> = Vec::new();
    while let Some(dir) = current.pop() {
        let depth = depth_stack.pop().unwrap_or(0);
        if depth > MAX_DEPTH {
            continue;
        }
        if visited.load(Ordering::Relaxed) > MAX_ENTRIES as u64 {
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            visited.fetch_add(1, Ordering::Relaxed);
            let p = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                if depth == 0 {
                    top_subdirs.push(p);
                } else {
                    current.push(p);
                    depth_stack.push(depth + 1);
                }
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    total.fetch_add(meta.len(), Ordering::Relaxed);
                }
            }
        }
    }

    if top_subdirs.is_empty() {
        return total.load(Ordering::Relaxed);
    }

    std::thread::scope(|s| {
        for subdir in &top_subdirs {
            let total = &total;
            let visited = &visited;
            s.spawn(move || {
                let local_total = AtomicU64::new(0);
                let local_visited = AtomicU64::new(0);
                let stack = vec![subdir.clone()];
                let mut current = stack;
                let mut depth_stack = vec![1u32];
                while let Some(dir) = current.pop() {
                    let depth = depth_stack.pop().unwrap_or(0);
                    if depth > MAX_DEPTH {
                        continue;
                    }
                    if visited.load(Ordering::Relaxed) + local_visited.load(Ordering::Relaxed)
                        > MAX_ENTRIES as u64
                    {
                        break;
                    }
                    let entries = match fs::read_dir(&dir) {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    for entry in entries.flatten() {
                        local_visited.fetch_add(1, Ordering::Relaxed);
                        let p = entry.path();
                        let ft = match entry.file_type() {
                            Ok(t) => t,
                            Err(_) => continue,
                        };
                        if ft.is_dir() {
                            current.push(p);
                            depth_stack.push(depth + 1);
                        } else if ft.is_file() {
                            if let Ok(meta) = entry.metadata() {
                                local_total.fetch_add(meta.len(), Ordering::Relaxed);
                            }
                        }
                    }
                }
                total.fetch_add(local_total.load(Ordering::Relaxed), Ordering::Relaxed);
                visited.fetch_add(local_visited.load(Ordering::Relaxed), Ordering::Relaxed);
            });
        }
    });

    total.load(Ordering::Relaxed)
}

/// 展开环境变量并返回规范化的路径字符串
fn expand_path(raw: &str) -> String {
    // 简单展开 %TEMP% / %LOCALAPPDATA% / %SYSTEMROOT% 等
    let mut out = raw.to_string();
    for (var, val) in [
        ("TEMP", std::env::var("TEMP").unwrap_or_default()),
        ("TMP", std::env::var("TMP").unwrap_or_default()),
        (
            "LOCALAPPDATA",
            std::env::var("LOCALAPPDATA").unwrap_or_default(),
        ),
        ("APPDATA", std::env::var("APPDATA").unwrap_or_default()),
        (
            "SYSTEMROOT",
            std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".to_string()),
        ),
        (
            "USERPROFILE",
            std::env::var("USERPROFILE").unwrap_or_default(),
        ),
    ] {
        if val.is_empty() {
            continue;
        }
        let from = format!("%{}%", var);
        out = out.replace(&from, &val);
    }
    out
}

/// 扫描 C 盘常见可回收垃圾（异步：放到线程池避免阻塞 UI）
#[command]
async fn scan_junk() -> Vec<JunkItem> {
    tauri::async_runtime::spawn_blocking(|| {
        let targets: [(&str, &str); 5] = [
            ("temp", "%TEMP%"),
            ("windowsTemp", "%SYSTEMROOT%\\Temp"),
            ("recycle", "C:\\$Recycle.Bin"),
            ("prefetch", "%SYSTEMROOT%\\Prefetch"),
            ("logs", "%SYSTEMROOT%\\Logs"),
        ];

        let expanded: Vec<(String, String)> = targets
            .iter()
            .map(|(key, raw)| (key.to_string(), expand_path(raw)))
            .collect();

        std::thread::scope(|s| {
            let handles: Vec<_> = expanded
                .iter()
                .map(|(key, path)| {
                    let key = key.clone();
                    let path = path.clone();
                    s.spawn(move || {
                        let size = dir_size(std::path::Path::new(&path));
                        (key, path, size)
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok())
                .map(|(key, path, size)| JunkItem { key, path, size })
                .collect()
        })
    })
    .await
    .unwrap_or_default()
}

/// 递归删除目录下所有内容（保留目录本身），返回是否无错误完成
fn clean_dir_contents(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return false,
    };

    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
    let mut dir_paths: Vec<std::path::PathBuf> = Vec::new();
    let mut all_ok = true;

    for entry in entries.flatten() {
        let p = entry.path();
        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => {
                all_ok = false;
                continue;
            }
        };
        if ft.is_dir() {
            dir_paths.push(p);
        } else if ft.is_file() {
            file_paths.push(p);
        }
    }

    for p in file_paths {
        if fs::remove_file(&p).is_err() {
            all_ok = false;
        }
    }

    if !dir_paths.is_empty() {
        let ok_flags = std::sync::Mutex::new(vec![false; dir_paths.len()]);
        std::thread::scope(|s| {
            for (i, p) in dir_paths.iter().enumerate() {
                let ok_flags = &ok_flags;
                s.spawn(move || {
                    let ok = fs::remove_dir_all(p).is_ok();
                    ok_flags.lock().unwrap()[i] = ok;
                });
            }
        });
        if ok_flags.lock().unwrap().iter().any(|&ok| !ok) {
            all_ok = false;
        }
    }

    all_ok
}

/// 清空回收站（调用系统 SHEmptyRecycleBinW，无需管理员权限）
fn empty_recycle_bin() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{
        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
    };
    unsafe {
        SHEmptyRecycleBinW(
            None,
            PCWSTR::null(),
            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
        )
    }
    .is_ok()
}

/// 清理选中的垃圾项（异步：放到线程池避免阻塞 UI）
#[command]
async fn clean_junk(targets: Vec<CleanTarget>) -> u64 {
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::scope(|s| {
            let handles: Vec<_> = targets
                .into_iter()
                .map(|t| {
                    s.spawn(move || -> u64 {
                        let path = Path::new(&t.path);
                        if t.key == "recycle" {
                            if empty_recycle_bin() {
                                t.size
                            } else {
                                0
                            }
                        } else if clean_dir_contents(path) {
                            t.size
                        } else {
                            0
                        }
                    })
                })
                .collect();

            handles
                .into_iter()
                .filter_map(|h| h.join().ok())
                .sum()
        })
    })
    .await
    .unwrap_or(0)
}

/// 硬件信息
#[derive(serde::Serialize)]
struct HardwareInfo {
    motherboard: String,
    cpu: String,
    gpu: String,
    ram_total: String,
    ram_used: String,
    ram_speed: String,
    ram_manufacturer: String,
    gpu_vram_total: String,
    gpu_driver: String,
    disks: Vec<DiskInfo>,
}

#[derive(serde::Serialize, Clone)]
struct DiskInfo {
    name: String,
    model: String,
    total: String,
    free: String,
    disk_type: String,
    interface: String,
}

/// 格式化字节数为人类可读
fn fmt_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}

static SYS: std::sync::LazyLock<std::sync::Mutex<sysinfo::System>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(sysinfo::System::new()));

static BOARD_INFO: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static GPU_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();

#[derive(Clone, Default)]
struct GpuLiveStats {
    usage: Option<f32>,
    vram_total: Option<u64>,
    vram_used: Option<u64>,
}

static GPU_LIVE: std::sync::LazyLock<std::sync::Mutex<GpuLiveStats>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(GpuLiveStats::default()));

static GPU_WORKER: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
    std::thread::spawn(|| loop {
        let (usage, vram_total, vram_used) = query_gpu_stats();
        if let Ok(mut c) = GPU_LIVE.lock() {
            c.usage = usage;
            c.vram_total = vram_total;
            c.vram_used = vram_used;
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
});

/// 查询硬件信息：主板/CPU/显卡/RAM
#[tauri::command]
fn get_hardware_info() -> HardwareInfo {
    let mut sys = SYS.lock().unwrap();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpus = sys.cpus();
    let cpu = if !cpus.is_empty() {
        format!("{} ({} 核心)", cpus[0].brand(), cpus.len())
    } else {
        "未知".to_string()
    };

    let ram_total = fmt_bytes(sys.total_memory());
    let ram_used = fmt_bytes(sys.used_memory());
    drop(sys);

    let motherboard = BOARD_INFO.get_or_init(query_motherboard).clone();
    let gpu = GPU_NAME.get_or_init(query_gpu).clone();
    let (ram_speed, ram_manufacturer) = query_ram_detail();
    let (gpu_vram_total, gpu_driver) = query_gpu_detail();
    let disks = query_disks();

    HardwareInfo {
        motherboard,
        cpu,
        gpu,
        ram_total,
        ram_used,
        ram_speed,
        ram_manufacturer,
        gpu_vram_total,
        gpu_driver,
        disks,
    }
}

/// 通过 WMI COM API 查询主板信息（原生，无需 PowerShell）
fn query_motherboard() -> String {
    use std::collections::HashMap;
    use std::time::Duration;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
                .ok()?;
            let results: Vec<HashMap<String, wmi::Variant>> = wmi
                .raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard")
                .ok()?;
            let row = results.first()?;
            let mfg = variant_to_string(row.get("Manufacturer"));
            let product = variant_to_string(row.get("Product"));
            Some(if mfg.is_empty() && product.is_empty() {
                "未知".to_string()
            } else if mfg.is_empty() {
                product
            } else if product.is_empty() {
                mfg
            } else {
                format!("{} {}", mfg, product)
            })
        })
        .join()
        .ok()
        .flatten()
    });
    result.unwrap_or_else(|| {
        let script = "Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue | ForEach-Object { \"$($_.Manufacturer)|$($_.Product)\" }";
        let output = run_powershell(script, Duration::from_secs(10));
        let raw = parse_powershell_output(output);
        let line = raw.lines().next().unwrap_or("").trim().to_string();
        let (mfg, product) = match line.split_once('|') {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None => (String::new(), String::new()),
        };
        if mfg.is_empty() && product.is_empty() {
            "未知".to_string()
        } else if mfg.is_empty() {
            product
        } else if product.is_empty() {
            mfg
        } else {
            format!("{} {}", mfg, product)
        }
    })
}

/// 通过 WMI COM API 查询显卡信息（原生，无需 PowerShell）
fn query_gpu() -> String {
    use std::collections::HashMap;
    use std::time::Duration;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
                .ok()?;
            let results: Vec<HashMap<String, wmi::Variant>> = wmi
                .raw_query("SELECT Name FROM Win32_VideoController WHERE Name IS NOT NULL")
                .ok()?;
            let names: Vec<String> = results
                .iter()
                .filter_map(|row| {
                    let n = variant_to_string(row.get("Name"));
                    if n.is_empty() { None } else { Some(n) }
                })
                .collect();
            Some(if names.is_empty() {
                "未知".to_string()
            } else {
                names.join(" + ")
            })
        })
        .join()
        .ok()
        .flatten()
    });
    result.unwrap_or_else(|| {
        let output = run_powershell(
            "Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Where-Object { $_.Name } | Select-Object -ExpandProperty Name",
            Duration::from_secs(10),
        );
        let raw = parse_powershell_output(output);
        let names: Vec<String> = raw
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        if names.is_empty() {
            "未知".to_string()
        } else {
            names.join(" + ")
        }
    })
}

/// 查询显卡详情：显存总量（DXGI 原生 API）、驱动版本（WMI）
fn query_gpu_detail() -> (String, String) {
    use std::collections::HashMap;
    use std::time::Duration;
    let vram = if let Some(bytes) = query_gpu_vram_dxgi() {
        fmt_bytes(bytes)
    } else {
        "未知".to_string()
    };

    let driver = {
        let result = std::thread::scope(|s| {
            s.spawn(|| {
                let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
                    .ok()?;
                let results: Vec<HashMap<String, wmi::Variant>> = wmi
                    .raw_query("SELECT DriverVersion FROM Win32_VideoController WHERE Name IS NOT NULL")
                    .ok()?;
                let row = results.first()?;
                let d = variant_to_string(row.get("DriverVersion"));
                Some(if d.is_empty() { "未知".to_string() } else { d })
            })
            .join()
            .ok()
            .flatten()
        });
        result.unwrap_or_else(|| {
            let script = "Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Where-Object { $_.Name } | Select-Object -First 1 -ExpandProperty DriverVersion";
            let output = run_powershell(script, Duration::from_secs(10));
            let raw = parse_powershell_output(output);
            let d = raw.trim().to_string();
            if d.is_empty() { "未知".to_string() } else { d }
        })
    };

    (vram, driver)
}

/// 将 WMI Variant 值转为 String
fn variant_to_string(v: Option<&wmi::Variant>) -> String {
    use wmi::Variant;
    match v {
        Some(Variant::String(s)) => s.trim().to_string(),
        Some(Variant::I1(n)) => n.to_string(),
        Some(Variant::I2(n)) => n.to_string(),
        Some(Variant::I4(n)) => n.to_string(),
        Some(Variant::I8(n)) => n.to_string(),
        Some(Variant::UI1(n)) => n.to_string(),
        Some(Variant::UI2(n)) => n.to_string(),
        Some(Variant::UI4(n)) => n.to_string(),
        Some(Variant::UI8(n)) => n.to_string(),
        Some(Variant::R4(n)) => n.to_string(),
        Some(Variant::R8(n)) => n.to_string(),
        Some(Variant::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// 通过 DXGI 枚举显卡获取真实显存（WMI AdapterRAM 为 32 位字段，4GB 以上会溢出）
fn query_gpu_vram_dxgi() -> Option<u64> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    };
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
    for i in 0..8 {
        let adapter = match unsafe { factory.EnumAdapters1(i) } {
            Ok(a) => a,
            Err(_) => break,
        };
        let desc = unsafe { adapter.GetDesc1() }.ok()?;
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let name = String::from_utf16_lossy(&desc.Description);
        if name.trim_end_matches('\0').trim().is_empty() {
            continue;
        }
        if desc.DedicatedVideoMemory > 0 {
            return Some(desc.DedicatedVideoMemory as u64);
        }
    }
    None
}

/// 查询内存条信息：速率、品牌（原生 WMI COM API）
fn query_ram_detail() -> (String, String) {
    use std::collections::HashMap;
    use std::time::Duration;
    let result = std::thread::scope(|s| {
        s.spawn(|| {
            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
                .ok()?;
            let results: Vec<HashMap<String, wmi::Variant>> = wmi
                .raw_query("SELECT Speed, Manufacturer FROM Win32_PhysicalMemory")
                .ok()?;
            let row = results.first()?;
            let speed_raw = variant_to_string(row.get("Speed"));
            let mfg = variant_to_string(row.get("Manufacturer"));
            let speed = speed_raw
                .parse::<u64>()
                .map(|s| format!("{} MHz", s))
                .unwrap_or_else(|_| {
                    if speed_raw.is_empty() {
                        "未知".to_string()
                    } else {
                        speed_raw.clone()
                    }
                });
            let brand = if mfg.is_empty() {
                "未知".to_string()
            } else {
                mfg.trim_matches(char::is_control).trim().to_string()
            };
            Some((speed, brand))
        })
        .join()
        .ok()
        .flatten()
    });
    result.unwrap_or_else(|| {
        let script = "Get-CimInstance Win32_PhysicalMemory -ErrorAction SilentlyContinue | Select-Object -First 1 | ForEach-Object { \"{0}|{1}\" -f $_.Speed, $_.Manufacturer }";
        let output = run_powershell(script, Duration::from_secs(10));
        let raw = parse_powershell_output(output);
        let line = raw.lines().next().unwrap_or("").trim().to_string();
        let mut parts = line.split('|');
        let speed_raw = parts.next().unwrap_or("").trim().to_string();
        let mfg = parts.next().unwrap_or("").trim().to_string();
        let speed = speed_raw
            .parse::<u64>()
            .map(|s| format!("{} MHz", s))
            .unwrap_or_else(|_| if speed_raw.is_empty() { "未知".to_string() } else { speed_raw });
        let brand = if mfg.is_empty() { "未知".to_string() } else { mfg.trim_matches(char::is_control).trim().to_string() };
        (speed, brand)
    })
}

/// 查询磁盘信息（sysinfo 主路径 + WMI 原生 API 补充型号/接口）
fn query_disks() -> Vec<DiskInfo> {
    use std::collections::HashMap;
    use sysinfo::Disks;
    let mut result = Vec::new();
    let disks = Disks::new_with_refreshed_list();
    for d in disks.list() {
        let name = d.name().to_string_lossy().to_string();
        let total = fmt_bytes(d.total_space());
        let free = fmt_bytes(d.available_space());
        result.push(DiskInfo {
            name,
            model: String::new(),
            total,
            free,
            disk_type: String::new(),
            interface: String::new(),
        });
    }
    if result.is_empty() {
        let mut result_ext: Vec<DiskInfo> = Vec::new();
        let wmi_list = std::thread::scope(|s| {
            s.spawn(|| {
                let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
                    .ok()?;
                let results: Vec<HashMap<String, wmi::Variant>> = wmi
                    .raw_query("SELECT Model, Size, InterfaceType FROM Win32_DiskDrive")
                    .ok()?;
                let mut list = Vec::new();
                for row in results {
                    let model = variant_to_string(row.get("Model"));
                    let size_raw = variant_to_string(row.get("Size"));
                    let iface = variant_to_string(row.get("InterfaceType"));
                    let total = size_raw
                        .parse::<u64>()
                        .map(fmt_bytes)
                        .unwrap_or_else(|_| "未知".to_string());
                    list.push(DiskInfo {
                        name: model.clone(),
                        model,
                        total,
                        free: "—".to_string(),
                        disk_type: String::new(),
                        interface: iface,
                    });
                }
                Some(list)
            })
            .join()
            .ok()
            .flatten()
        });
        if let Some(list) = wmi_list {
            result_ext.extend(list);
        }
        if result_ext.is_empty() {
            let script = "Get-CimInstance Win32_DiskDrive -ErrorAction SilentlyContinue | ForEach-Object { \"{0}|{1}|{2}\" -f $_.Model, $_.Size, $_.InterfaceType }";
            let output = run_powershell(script, std::time::Duration::from_secs(10));
            let raw = parse_powershell_output(output);
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() { continue; }
                let mut parts = line.split('|');
                let model = parts.next().unwrap_or("").trim().to_string();
                let size_raw = parts.next().unwrap_or("").trim().to_string();
                let iface = parts.next().unwrap_or("").trim().to_string();
                let total = size_raw.parse::<u64>().map(fmt_bytes).unwrap_or_else(|_| "未知".to_string());
                result_ext.push(DiskInfo {
                    name: model.clone(),
                    model,
                    total,
                    free: "—".to_string(),
                    disk_type: String::new(),
                    interface: iface,
                });
            }
        }
        result_ext
    } else {
        result
    }
}

/// 带超时执行 PowerShell 命令，避免查询卡死
fn run_powershell(
    script: &str,
    timeout: std::time::Duration,
) -> Result<std::process::Output, std::io::Error> {
    use std::io::ErrorKind;
    use std::process::{Command, Stdio};
    use std::time::Instant;

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut child = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;

    let start = Instant::now();
    let mut timed_out = false;
    while child.try_wait()?.is_none() {
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    if timed_out {
        return Err(std::io::Error::new(ErrorKind::TimedOut, "powershell timeout"));
    }
    child.wait_with_output()
}

/// 解析 PowerShell 命令输出为字符串
fn parse_powershell_output(output: Result<std::process::Output, std::io::Error>) -> String {
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            if stdout.trim().is_empty() {
                String::from_utf8_lossy(&o.stderr).to_string()
            } else {
                stdout
            }
        }
        Err(_) => String::new(),
    }
}

/// 性能监测数据
#[derive(serde::Serialize)]
struct PerformanceStats {
    cpu_usage: f32,
    cpu_per_core: Vec<f32>,
    cpu_freq: u64,
    cpu_cores: usize,
    cpu_name: String,
    ram_total: u64,
    ram_used: u64,
    ram_usage: f32,
    gpu_name: String,
    gpu_usage: Option<f32>,
    gpu_vram_total: Option<u64>,
    gpu_vram_used: Option<u64>,
}

/// 查询性能监测数据
#[tauri::command]
fn get_performance_stats() -> PerformanceStats {
    std::sync::LazyLock::force(&GPU_WORKER);

    let mut sys = SYS.lock().unwrap();
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpus = sys.cpus();
    let cpu_usage = if !cpus.is_empty() {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    } else {
        0.0
    };
    let cpu_per_core: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let cpu_freq = cpus.first().map(|c| c.frequency()).unwrap_or(0);
    let cpu_cores = cpus.len();
    let cpu_name = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();

    let ram_total = sys.total_memory();
    let ram_used = sys.used_memory();
    let ram_usage = if ram_total > 0 {
        ram_used as f32 / ram_total as f32 * 100.0
    } else {
        0.0
    };
    drop(sys);

    let gpu_name = GPU_NAME.get_or_init(query_gpu).clone();
    let gpu = GPU_LIVE.lock().map(|c| c.clone()).unwrap_or_default();

    PerformanceStats {
        cpu_usage,
        cpu_per_core,
        cpu_freq,
        cpu_cores,
        cpu_name,
        ram_total,
        ram_used,
        ram_usage,
        gpu_name,
        gpu_usage: gpu.usage,
        gpu_vram_total: gpu.vram_total,
        gpu_vram_used: gpu.vram_used,
    }
}

/// 通过 PowerShell 性能计数器查询 GPU 使用率和显存
fn query_gpu_stats() -> (Option<f32>, Option<u64>, Option<u64>) {
    let script = r#"
$e = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -ErrorAction SilentlyContinue)
$m = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory -ErrorAction SilentlyContinue)
$avg = 0.0
if ($e.Count -gt 0) { $avg = ($e | Measure-Object UtilizationPercentage -Average).Average }
$sum = 0
if ($m.Count -gt 0) { $sum = ($m | Measure-Object DedicatedUsage -Sum).Sum }
$vram = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Select-Object -ExpandProperty AdapterRAM)
$total = $null
if ($vram.Count -gt 0) { $total = $vram[0] }
Write-Output ("{0:F2}|{1}|{2}" -f $avg, $sum, $total)
"#;
    let output = run_powershell(script, std::time::Duration::from_secs(10));
    let raw = parse_powershell_output(output);
    let line = raw.lines().next().unwrap_or("").trim();
    let mut parts = line.split('|');
    let usage = parts.next().and_then(|s| s.parse::<f32>().ok());
    let vram_used = parts.next().and_then(|s| s.parse::<u64>().ok());
    let vram_total = parts.next().and_then(|s| s.parse::<u64>().ok());

    (usage, vram_total, vram_used)
}

/// 进程信息
#[derive(serde::Serialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory: u64,
    memory_percent: f32,
    status: String,
    icon: Option<String>,
    is_self: bool,
    is_related: bool,
    is_system: bool,
    parent_pid: Option<u32>,
}

/// exe 图标缓存：相同路径只提取一次
static ICON_CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Option<String>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// 从 exe 路径提取图标（16x16 PNG base64）
fn extract_exe_icon(exe_path: &std::path::Path) -> Option<String> {
    if let Ok(cache) = ICON_CACHE.lock() {
        if let Some(v) = cache.get(exe_path) {
            return v.clone();
        }
    }
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_SMALLICON, SHFILEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

    let path_wide: Vec<u16> = exe_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut shfi = SHFILEINFOW::default();
    let flags = SHGFI_ICON | SHGFI_SMALLICON;
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(path_wide.as_ptr()),
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            flags,
        )
    };

    if result == 0 || shfi.hIcon.is_invalid() {
        return None;
    }

    let hicon = shfi.hIcon;
    let png = icon_to_png_base64(hicon);
    unsafe { let _ = DestroyIcon(hicon); }
    if let Ok(mut cache) = ICON_CACHE.lock() {
        cache.insert(exe_path.to_path_buf(), png.clone());
    }
    png
}

/// 将 HICON 转换为 PNG base64
fn icon_to_png_base64(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<String> {
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteObject, DeleteDC, GetDIBits,
        BITMAPINFO, BITMAPINFOHEADER, DIB_USAGE, RGBQUAD,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};

    let mut icon_info = ICONINFO::default();
    unsafe { GetIconInfo(hicon, &mut icon_info).ok()? };

    let hdc = unsafe { CreateCompatibleDC(None) };
    if hdc.is_invalid() {
        unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
        unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }
        return None;
    }

    let mut bi = BITMAPINFOHEADER::default();
    bi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bi.biWidth = 16;
    bi.biHeight = -16;
    bi.biPlanes = 1;
    bi.biBitCount = 32;
    bi.biCompression = 0;

    let mut bmi = BITMAPINFO {
        bmiHeader: bi,
        bmiColors: [RGBQUAD::default(); 1],
    };

    let mut pixels: Vec<u8> = vec![0u8; (16 * 16 * 4) as usize];
    let rows = unsafe {
        GetDIBits(
            hdc,
            icon_info.hbmColor,
            0,
            16,
            Some(pixels.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_USAGE(0),
        )
    };

    unsafe { let _ = DeleteDC(hdc); }
    unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
    unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }

    if rows == 0 {
        return None;
    }

    let png = rgba_to_png(16, 16, &pixels)?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Some(format!("data:image/png;base64,{}", b64))
}

/// 简易 RGBA → PNG 编码（无依赖实现）
fn rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::new();

    let mut filtered: Vec<u8> = Vec::with_capacity((width * height * 4 + height) as usize);
    let mut row_start = 0;
    for _y in 0..height {
        filtered.push(0u8);
        for x in 0..width {
            let i = row_start + (x * 4) as usize;
            let b = rgba[i];
            let g = rgba[i + 1];
            let r = rgba[i + 2];
            let a = rgba[i + 3];
            filtered.push(r);
            filtered.push(g);
            filtered.push(b);
            filtered.push(a);
        }
        row_start += (width * 4) as usize;
    }

    let raw = &filtered;
    let zlib = flate_write(raw)?;
    let crc_table = build_crc_table();

    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    write_chunk(&mut out, b"IHDR", {
        let mut ihdr = Vec::with_capacity(13);
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.push(8);
        ihdr.push(6);
        ihdr.push(0);
        ihdr.push(0);
        ihdr.push(0);
        ihdr
    }, &crc_table);
    write_chunk(&mut out, b"IDAT", zlib, &crc_table);
    write_chunk(&mut out, b"IEND", Vec::new(), &crc_table);

    Some(out)
}

fn build_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xedb88320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        table[n as usize] = c;
    }
    table
}

fn crc32(data: &[u8], table: &[u32; 256]) -> u32 {
    let mut crc = 0xffffffff;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
    }
    crc ^ 0xffffffff
}

fn write_chunk(out: &mut Vec<u8>, type_: &[u8; 4], data: Vec<u8>, table: &[u32; 256]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(type_);
    out.extend_from_slice(&data);
    let crc = crc32(&out[start..], table);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn flate_write(data: &[u8]) -> Option<Vec<u8>> {
    // DEFLATE stored（无压缩）块：BFINAL=1, BTYPE=00 占首字节（含 5 bit 对齐），后跟 LEN/NLEN
    if data.len() > u16::MAX as usize {
        return None;
    }
    let mut zlib = Vec::with_capacity(data.len() + 16);
    zlib.push(0x78);
    zlib.push(0x01);
    zlib.push(0x01);
    let len = data.len() as u16;
    zlib.extend_from_slice(&len.to_le_bytes());
    zlib.extend_from_slice(&(!len).to_le_bytes());
    zlib.extend_from_slice(data);
    let adler = adler32(data);
    zlib.extend_from_slice(&adler.to_be_bytes());
    Some(zlib)
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// 调用 Windows SCM API 枚举所有正在运行的服务进程 PID
fn query_service_pids() -> std::collections::HashSet<u32> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Services::{
        CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW,
        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL, SERVICE_WIN32,
    };

    let mut pids: std::collections::HashSet<u32> = std::collections::HashSet::new();

    let scm = unsafe {
        OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)
    };
    let Ok(scm) = scm else { return pids; };

    let mut bytes_needed: u32 = 0;
    let mut services_returned: u32 = 0;
    let mut resume_handle: u32 = 0;

    let _ = unsafe {
        EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            PCWSTR::null(),
        )
    };

    if bytes_needed == 0 {
        unsafe { let _ = CloseServiceHandle(scm); };
        return pids;
    }

    let mut buffer: Vec<u8> = vec![0u8; bytes_needed as usize];
    let ok = unsafe {
        EnumServicesStatusExW(
            scm,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(buffer.as_mut_slice()),
            &mut bytes_needed,
            &mut services_returned,
            Some(&mut resume_handle),
            PCWSTR::null(),
        )
    };

    if ok.is_ok() && services_returned > 0 {
        let entries_ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
        for i in 0..services_returned as usize {
            let entry = unsafe { &*entries_ptr.add(i) };
            let pid = entry.ServiceStatusProcess.dwProcessId;
            if pid != 0 {
                pids.insert(pid);
            }
        }
    }

    unsafe { let _ = CloseServiceHandle(scm); };

    pids
}

/// 查询进程列表（不含图标，图标由 process_icons 异步补充）
#[tauri::command]
fn list_processes() -> Vec<ProcessInfo> {
    use sysinfo::ProcessesToUpdate;

    let mut sys = SYS.lock().unwrap();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let total_mem = sys.total_memory();
    let self_pid = std::process::id();

    let mut related_pids: std::collections::HashSet<u32> = std::collections::HashSet::new();
    related_pids.insert(self_pid);
    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
    queue.push_back(self_pid);
    while let Some(parent_pid) = queue.pop_front() {
        for (pid, p) in sys.processes() {
            let pid_u32 = pid.as_u32();
            if related_pids.contains(&pid_u32) {
                continue;
            }
            if p.parent().map(|pp| pp.as_u32() == parent_pid).unwrap_or(false) {
                related_pids.insert(pid_u32);
                queue.push_back(pid_u32);
            }
        }
    }

    let service_pids = query_service_pids();

    let mut entries: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, p)| {
            let mem = p.memory();
            let mem_percent = if total_mem > 0 {
                mem as f32 / total_mem as f32 * 100.0
            } else {
                0.0
            };
            let status = match p.status() {
                sysinfo::ProcessStatus::Run => "running",
                sysinfo::ProcessStatus::Sleep => "sleeping",
                sysinfo::ProcessStatus::Stop => "stopped",
                _ => "unknown",
            }
            .to_string();
            let pid_u32 = pid.as_u32();
            let is_self = pid_u32 == self_pid;
            let is_related = !is_self && related_pids.contains(&pid_u32);
            let is_system = !is_self && !is_related && service_pids.contains(&pid_u32);
            ProcessInfo {
                pid: pid_u32,
                name: p.name().to_string_lossy().to_string(),
                cpu_usage: p.cpu_usage(),
                memory: mem,
                memory_percent: mem_percent,
                status,
                icon: None,
                is_self,
                is_related,
                is_system,
                parent_pid: p.parent().map(|pp| pp.as_u32()),
            }
        })
        .collect();
    drop(sys);

    entries.sort_by(|a, b| b.memory.cmp(&a.memory));

    entries
}

/// 进程图标条目
#[derive(serde::Serialize)]
struct ProcessIcon {
    pid: u32,
    icon: Option<String>,
}

/// 批量提取进程图标（多线程并行，带路径级缓存）
#[tauri::command]
fn process_icons(pids: Vec<u32>) -> Vec<ProcessIcon> {
    let paths: Vec<(u32, std::path::PathBuf)> = {
        let sys = SYS.lock().unwrap();
        pids.into_iter()
            .filter_map(|pid| {
                sys.process(sysinfo::Pid::from_u32(pid))
                    .and_then(|p| p.exe().map(|e| (pid, e.to_path_buf())))
            })
            .collect()
    };

    let mut result: Vec<ProcessIcon> = Vec::with_capacity(paths.len());
    let mut pending: Vec<(u32, std::path::PathBuf)> = Vec::new();

    for (pid, path) in paths {
        let cached = ICON_CACHE
            .lock()
            .ok()
            .and_then(|c| c.get(&path).cloned());
        match cached {
            Some(v) => result.push(ProcessIcon { pid, icon: v }),
            None => pending.push((pid, path)),
        }
    }

    if !pending.is_empty() {
        let worker_count = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(2);
        let chunk_size = pending.len().div_ceil(worker_count);
        let mut handles = Vec::new();
        for chunk in pending.chunks(chunk_size) {
            let chunk = chunk.to_vec();
            handles.push(std::thread::spawn(move || {
                chunk
                    .into_iter()
                    .map(|(pid, path)| ProcessIcon {
                        pid,
                        icon: extract_exe_icon(&path),
                    })
                    .collect::<Vec<_>>()
            }));
        }
        for h in handles {
            if let Ok(mut part) = h.join() {
                result.append(&mut part);
            }
        }
    }

    result
}

/// 结束进程
#[tauri::command]
fn kill_process(pid: u32) -> Result<String, String> {
    let sys = SYS.lock().unwrap();
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
        if process.kill() {
            Ok(format!("进程 {} 已结束", pid))
        } else {
            Err(format!("无法结束进程 {}", pid))
        }
    } else {
        Err(format!("未找到进程 {}", pid))
    }
}

/// 获取当前前台窗口对应的进程 PID
#[tauri::command]
fn get_foreground_window_pid() -> Result<u32, String> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return Err("无法获取前台窗口".to_string());
    }
    let mut pid: u32 = 0;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
    }
    if pid == 0 {
        return Err("无法获取窗口进程 PID".to_string());
    }
    Ok(pid)
}

/// 冻结进程（NtSuspendProcess）
#[tauri::command]
fn suspend_process(pid: u32) -> Result<String, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

    type NtSuspendProcess = unsafe extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32;

    let h = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) }
        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtSuspendProcess")) }
        .ok_or_else(|| "找不到 NtSuspendProcess".to_string())?;
    let func: NtSuspendProcess = unsafe { std::mem::transmute(addr) };
    let status = unsafe { func(h) };
    unsafe { let _ = CloseHandle(h); };
    if status < 0 {
        return Err(format!("NtSuspendProcess 返回 0x{:X}", status as u32));
    }
    Ok(format!("进程 {} 已冻结", pid))
}

/// 恢复进程（NtResumeProcess）
#[tauri::command]
fn resume_process(pid: u32) -> Result<String, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

    type NtResumeProcess = unsafe extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32;

    let h = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) }
        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtResumeProcess")) }
        .ok_or_else(|| "找不到 NtResumeProcess".to_string())?;
    let func: NtResumeProcess = unsafe { std::mem::transmute(addr) };
    let status = unsafe { func(h) };
    unsafe { let _ = CloseHandle(h); };
    if status < 0 {
        return Err(format!("NtResumeProcess 返回 0x{:X}", status as u32));
    }
    Ok(format!("进程 {} 已恢复", pid))
}

/// 查询进程的 PPL 保护级别
#[tauri::command]
fn get_ppl_protection(pid: u32) -> Result<String, String> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    type NtQueryInformationProcess = unsafe extern "system" fn(
        HANDLE,
        u32,
        *mut std::ffi::c_void,
        u32,
        *mut u32,
    ) -> i32;

    const PROCESS_PROTECTION_INFORMATION: u32 = 61;
    #[repr(C)]
    #[derive(Default)]
    struct ProcessProtection {
        protection_level: u16,
        _reserved1: u16,
        _reserved2: u32,
    }

    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtQueryInformationProcess")) }
        .ok_or_else(|| "找不到 NtQueryInformationProcess".to_string())?;
    let func: NtQueryInformationProcess = unsafe { std::mem::transmute(addr) };

    let mut info = ProcessProtection::default();
    let mut ret_len: u32 = 0;
    let status = unsafe {
        func(
            h,
            PROCESS_PROTECTION_INFORMATION,
            &mut info as *mut _ as *mut _,
            std::mem::size_of::<ProcessProtection>() as u32,
            &mut ret_len,
        )
    };
    unsafe { let _ = CloseHandle(h); };

    if status < 0 {
        return Err(format!("NtQueryInformationProcess 返回 0x{:X}", status as u32));
    }

    let level = info.protection_level & 0x7;
    let type_ = (info.protection_level >> 4) & 0x7;
    let sign = (info.protection_level >> 8) & 0x7;

    let type_str = match type_ {
        0 => "None",
        1 => "ProtectedLight",
        2 => "Protected",
        _ => "Unknown",
    };
    let level_str = match (type_, level) {
        (0, _) => "无保护".to_string(),
        (1, 0) => "ProtectedLight (None)".to_string(),
        (1, 1) => "ProtectedLight (Lsa)".to_string(),
        (1, 2) => "ProtectedLight (Windows)".to_string(),
        (1, 3) => "ProtectedLight (AntiMalware)".to_string(),
        (1, 4) => "ProtectedLight (CodeIntegrity)".to_string(),
        (1, 5) => "ProtectedLight (Authenticode)".to_string(),
        (2, 0) => "Protected (None)".to_string(),
        (2, 1) => "Protected (Lsa)".to_string(),
        (2, 2) => "Protected (Windows)".to_string(),
        (2, 3) => "Protected (AntiMalware)".to_string(),
        (2, 4) => "Protected (CodeIntegrity)".to_string(),
        (2, 5) => "Protected (Authenticode)".to_string(),
        _ => format!("Unknown ({}, {})", type_, level),
    };
    let sign_str = match sign {
        0 => "None",
        1 => "Authenticode",
        2 => "CodeIntegrity",
        3 => "Platform",
        _ => "Unknown",
    };

    Ok(format!("Type: {} | Level: {} | Sign: {}", type_str, level_str, sign_str))
}

/// 以管理员权限重启进程（需要进程 exe 路径），成功后自动结束原进程
#[tauri::command]
fn restart_as_admin(pid: u32) -> Result<String, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;

    let exe_path = {
        let sys = SYS.lock().unwrap();
        sys.process(sysinfo::Pid::from_u32(pid))
            .and_then(|p| p.exe().map(|e| e.to_path_buf()))
    }
    .ok_or_else(|| format!("未找到进程 {} 或无法获取 exe 路径", pid))?;

    let wide: Vec<u16> = exe_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();

    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_NORMAL,
        )
    };

    if result.0 as usize <= 32 {
        return Err(format!("ShellExecute 失败，返回码 {}", result.0 as usize));
    }

    let sys = SYS.lock().unwrap();
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
        let _ = process.kill();
    }
    drop(sys);

    Ok(format!("已以管理员权限重启进程 {} 并关闭原进程", pid))
}

// ==================== Git 可视化管理 ====================

/// 在指定目录执行 git 命令，返回 (成功与否, stdout, stderr)
fn run_git(repo: &str, args: &[&str]) -> (bool, String, String) {
    use std::process::{Command, Stdio};
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let candidates: Vec<std::ffi::OsString> = vec![
        "git".into(),
        r"C:\Program Files\Git\cmd\git.exe".into(),
        r"C:\Program Files (x86)\Git\cmd\git.exe".into(),
    ];
    let mut last_err = String::new();
    for exe in candidates {
        match Command::new(&exe)
            .arg("-c")
            .arg("core.quotepath=false")
            .args(args)
            .current_dir(repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                return (out.status.success(), stdout, stderr);
            }
            Err(e) => last_err = e.to_string(),
        }
    }
    (false, String::new(), last_err)
}

/// 检测系统是否安装了 git（PATH 优先，其次常见安装目录）
#[command]
fn check_git() -> bool {
    use std::process::{Command, Stdio};
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    if Command::new("git")
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return true;
    }
    let candidates = [
        r"C:\Program Files\Git\cmd\git.exe",
        r"C:\Program Files (x86)\Git\cmd\git.exe",
    ];
    candidates.iter().any(|p| Path::new(p).exists())
}

/// 通过 winget 一键安装 Git
#[command]
fn install_git() -> Result<String, String> {
    use std::process::{Command, Stdio};
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = Command::new("winget")
        .args([
            "install",
            "--id", "Git.Git",
            "-e",
            "--source", "winget",
            "--accept-package-agreements",
            "--accept-source-agreements",
            "--silent",
            "--disable-interactivity",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "winget 不可用，请手动安装 Git".to_string()
            } else {
                format!("启动 winget 失败: {e}")
            }
        })?;

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
        let tail: String = if stderr.trim().len() > stdout.trim().len() {
            stderr
        } else {
            stdout
        }
        .lines()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
        Err(if tail.trim().is_empty() {
            "winget 安装失败".to_string()
        } else {
            tail
        })
    }
}

/// 获取应用默认工作目录（供前端探测仓库）
#[command]
fn git_default_dir() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 探测给定路径所属的 git 仓库根目录，不是仓库时返回 None
#[command]
fn git_repo_root(path: String) -> Option<String> {
    if !Path::new(&path).exists() {
        return None;
    }
    let (ok, out, _) = run_git(&path, &["rev-parse", "--show-toplevel"]);
    if ok {
        let root = out.trim().to_string();
        if root.is_empty() {
            None
        } else {
            Some(root)
        }
    } else {
        None
    }
}

/// Git 文件状态项
#[derive(serde::Serialize)]
struct GitFile {
    path: String,
    x: String,
    y: String,
    staged: bool,
}

/// Git 状态汇总
#[derive(serde::Serialize)]
struct GitStatus {
    branch: String,
    ahead: u32,
    behind: u32,
    files: Vec<GitFile>,
    clean: bool,
}

/// 查询仓库状态（git status --porcelain=v1 -b）
#[command]
fn git_status(repo: String) -> Result<GitStatus, String> {
    let (ok, out, err) = run_git(&repo, &["status", "--porcelain=v1", "-b"]);
    if !ok {
        return Err(err.trim().to_string());
    }
    let mut branch = String::new();
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut files = Vec::new();
    for line in out.lines() {
        if !line.is_empty() && line.starts_with("##") {
            let head = line[2..].trim();
            if let Some(rest) = head.strip_prefix("No commits yet on ") {
                branch = rest.trim_end_matches('.').trim().to_string();
                continue;
            }
            let parts: Vec<&str> = head.split("...").collect();
            branch = parts[0].trim().to_string();
            for seg in parts.iter().skip(1) {
                for token in seg.split_whitespace() {
                    if let Some(v) = token.strip_prefix("ahead ") {
                        ahead = v.parse().unwrap_or(0);
                    } else if let Some(v) = token.strip_prefix("behind ") {
                        behind = v.parse().unwrap_or(0);
                    }
                }
            }
            continue;
        }
        if line.len() < 4 {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let x = chars[0].to_string();
        let y = chars[1].to_string();
        let staged = chars[0] != ' ' && chars[0] != '?';
        let path = line[3..].trim().to_string();
        files.push(GitFile {
            path,
            x,
            y,
            staged,
        });
    }
    let clean = files.is_empty();
    Ok(GitStatus {
        branch,
        ahead,
        behind,
        files,
        clean,
    })
}

/// Git 提交记录
#[derive(serde::Serialize)]
struct GitCommit {
    hash: String,
    short_hash: String,
    author: String,
    date: String,
    message: String,
    body: String,
}

/// 查询提交历史（最近 limit 条）
#[command]
fn git_log(repo: String, limit: u32) -> Result<Vec<GitCommit>, String> {
    // %B 是完整 commit message（subject + body），用 %x00 分隔字段，用 %x1e 分隔记录
    let fmt = "%H%x1f%h%x1f%an%x1f%ad%x1f%B%x1e";
    let limit_str = format!("-{}", limit.max(1));
    let (ok, out, err) = run_git(
        &repo,
        &["log", &limit_str, "--date=short", &format!("--pretty=format:{}", fmt)],
    );
    if !ok {
        let lower = err.to_lowercase();
        if lower.contains("does not have any commits yet")
            || lower.contains("unknown revision")
            || lower.contains("no commits yet")
        {
            return Ok(Vec::new());
        }
        return Err(err.trim().to_string());
    }
    let mut commits = Vec::new();
    for record in out.split('\u{1e}') {
        let record = record.trim_start_matches('\n').trim();
        if record.is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.splitn(5, '\u{1f}').collect();
        if parts.len() == 5 {
            let full = parts[4];
            // 完整 message 第一行是 subject，其余是 body
            let mut lines = full.lines();
            let subject = lines.next().unwrap_or("").trim().to_string();
            let body: String = lines
                .skip_while(|l| l.trim().is_empty()) // 跳过 subject 和 body 之间的空行
                .collect::<Vec<&str>>()
                .join("\n")
                .trim()
                .to_string();
            commits.push(GitCommit {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
                message: subject,
                body,
            });
        }
    }
    Ok(commits)
}

/// 暂存文件（git add）
#[command]
fn git_add(repo: String, paths: Vec<String>) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["add", "--"];
    args.extend(paths.iter().map(|s| s.as_str()));
    let (ok, _, err) = run_git(&repo, &args);
    if ok {
        Ok(())
    } else {
        Err(err.trim().to_string())
    }
}

/// 取消暂存（git restore --staged）
#[command]
fn git_unstage(repo: String, paths: Vec<String>) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["restore", "--staged", "--"];
    args.extend(paths.iter().map(|s| s.as_str()));
    let (ok, _, err) = run_git(&repo, &args);
    if ok {
        Ok(())
    } else {
        Err(err.trim().to_string())
    }
}

/// 提交（git commit -m）
#[command]
fn git_commit(repo: String, message: String, body: Option<String>, branch: Option<String>) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("提交信息不能为空".to_string());
    }
    let has_commits = {
        let (ok, out, _) = run_git(&repo, &["rev-parse", "--verify", "HEAD"]);
        ok && !out.trim().is_empty()
    };
    if let Some(b) = &branch {
        if !b.is_empty() && has_commits {
            let (cur_ok, cur_out, _) = run_git(&repo, &["branch", "--show-current"]);
            let cur = cur_out.trim();
            if cur_ok && !cur.is_empty() && cur != b {
                let (co_ok, _, co_err) = run_git(&repo, &["checkout", b]);
                if !co_ok {
                    return Err(co_err.trim().to_string());
                }
            } else if cur.is_empty() || !cur_ok {
                let (co_ok, _, co_err) = run_git(&repo, &["checkout", b]);
                if !co_ok {
                    return Err(co_err.trim().to_string());
                }
            }
        }
    }
    // 组装提交信息：标题 + 空行 + 详细信息
    let full_message = match body {
        Some(b) if !b.trim().is_empty() => format!("{}\n\n{}", message.trim(), b.trim()),
        _ => message.trim().to_string(),
    };
    let (ok, out, err) = run_git(&repo, &["commit", "-m", &full_message]);
    if ok {
        Ok(out.trim().to_string())
    } else {
        Err(err.trim().to_string())
    }
}

/// 撤回指定提交（git revert）
#[command]
fn git_revert(repo: String, hash: String, no_commit: Option<bool>) -> Result<String, String> {
    if hash.trim().is_empty() {
        return Err("提交哈希不能为空".to_string());
    }
    let mut args = vec!["revert"];
    if no_commit.unwrap_or(false) {
        args.push("--no-commit");
    }
    args.push(hash.trim());
    let (ok, out, err) = run_git(&repo, &args);
    if ok {
        Ok(out.trim().to_string())
    } else {
        Err(err.trim().to_string())
    }
}

/// 切换工作区到指定提交（git checkout，进入 detached HEAD）
#[command]
fn git_checkout(repo: String, hash: String) -> Result<String, String> {
    if hash.trim().is_empty() {
        return Err("提交哈希不能为空".to_string());
    }
    let (ok, out, err) = run_git(&repo, &["checkout", hash.trim()]);
    if ok {
        Ok(out.trim().to_string())
    } else {
        Err(err.trim().to_string())
    }
}

/// 列出本地分支
#[command]
fn git_branches(repo: String) -> Result<Vec<String>, String> {
    let (ok, out, err) = run_git(&repo, &["branch", "--format=%(refname:short)"]);
    if !ok {
        let lower = err.to_lowercase();
        if lower.contains("does not have any commits yet") || lower.contains("no commits yet") {
            return Ok(Vec::new());
        }
        return Err(err.trim().to_string());
    }
    Ok(out
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// 推送当前分支到远程
#[command]
fn git_push(repo: String, branch: Option<String>) -> Result<String, String> {
    let target = branch.filter(|b| !b.is_empty());
    let (ok, out, err) = match &target {
        Some(b) => run_git(&repo, &["push", "-u", "origin", b]),
        None => run_git(&repo, &["push"]),
    };
    if ok {
        Ok(out.trim().to_string())
    } else {
        let msg = err.trim();
        if msg.is_empty() {
            Err("推送失败".to_string())
        } else {
            Err(msg.to_string())
        }
    }
}

/// 拉取远程提交（fetch，不合并）
#[command]
fn git_fetch(repo: String) -> Result<String, String> {
    let (ok, out, err) = run_git(&repo, &["fetch", "--all", "--prune"]);
    if ok {
        Ok(out.trim().to_string())
    } else {
        let msg = if err.trim().is_empty() { out.trim() } else { err.trim() };
        Err(if msg.is_empty() { "拉取失败".to_string() } else { msg.to_string() })
    }
}

/// 拉取并合并远程提交（pull）
#[command]
fn git_pull(repo: String) -> Result<String, String> {
    let (ok, out, err) = run_git(&repo, &["pull", "--ff-only"]);
    if ok {
        Ok(out.trim().to_string())
    } else {
        let msg = if err.trim().is_empty() { out.trim() } else { err.trim() };
        Err(if msg.is_empty() { "拉取失败".to_string() } else { msg.to_string() })
    }
}

/// 克隆远程仓库到指定路径
#[command]
async fn git_clone(url: String, target_dir: String) -> Result<String, String> {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    let task = tauri::async_runtime::spawn_blocking(move || {
        let repo_name = url
            .trim_end_matches('/')
            .rsplit('/').next()
            .map(|n| n.trim_end_matches(".git"))
            .unwrap_or("repo");
        let dest = Path::new(&target_dir).join(repo_name);
        let dest_str = dest.to_string_lossy().to_string();
        let out = Command::new("git")
            .args(["clone", &url, &dest_str])
            .creation_flags(0x0800_0000)
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(dest_str)
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    task
}

/// 在指定路径初始化一个 Git 仓库（git init）
#[command]
async fn git_init(path: String) -> Result<String, String> {
    use std::path::Path;
    use std::os::windows::process::CommandExt;
    if !Path::new(&path).exists() {
        return Err("目标路径不存在".to_string());
    }
    let task = tauri::async_runtime::spawn_blocking(move || {
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let out = Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("git init 执行失败: {e}"))?;
        if out.status.success() {
            Ok(path.clone())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    task
}

/// 弹出系统文件夹选择对话框，返回选中路径
#[command]
async fn pick_folder() -> Result<Option<String>, String> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_title("选择文件夹")
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(task)
}

/// gh 登录状态
#[derive(serde::Serialize)]
struct GhAuthState {
    gh_installed: bool,
    logged_in: bool,
    user: String,
    host: String,
}

fn gh_available() -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;
    use std::time::Duration;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let out = Command::new("gh")
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let _ = tx.send(out);
    });
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(o)) => o.status.success(),
        _ => false,
    }
}

/// 查询 gh 登录状态（async，避免阻塞 UI）
#[command]
async fn gh_auth_state() -> GhAuthState {
    let installed = tauri::async_runtime::spawn_blocking(|| gh_available())
        .await
        .unwrap_or(false);
    if !installed {
        return GhAuthState {
            gh_installed: false,
            logged_in: false,
            user: String::new(),
            host: String::new(),
        };
    }
    let (has_token, user) = tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        use std::sync::mpsc;
        use std::time::Duration;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let token_out = Command::new("gh")
                .args(["auth", "token"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let user_out = Command::new("gh")
                .args(["api", "user", "--jq", ".login"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            let _ = tx.send((token_out, user_out));
        });
        match rx.recv_timeout(Duration::from_secs(8)) {
            Ok((Ok(tok), Ok(usr))) => {
                let has_tok = tok.status.success()
                    && !String::from_utf8_lossy(&tok.stdout).trim().is_empty();
                let username = if usr.status.success() {
                    String::from_utf8_lossy(&usr.stdout).trim().to_string()
                } else {
                    String::new()
                };
                (has_tok, username)
            }
            _ => (false, String::new()),
        }
    })
    .await
    .unwrap_or((false, String::new()));

    GhAuthState {
        gh_installed: true,
        logged_in: has_token,
        user,
        host: if has_token { "github.com".to_string() } else { String::new() },
    }
}

/// 启动 gh auth login 网页登录流程（async）
#[command]
async fn gh_login_web() -> Result<String, String> {
    let combined = tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        match Command::new("gh")
            .args(["auth", "login", "--web", "--git-protocol", "https"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(out) => (
                out.status.success(),
                format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                ),
            ),
            Err(e) => (false, format!("启动 gh 失败: {e}")),
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    if combined.0 {
        Ok(combined.1)
    } else {
        Err(combined.1.trim().to_string())
    }
}

/// 为当前仓库配置 gh 作为 git 凭据助手（async）
#[command]
async fn gh_setup_git(repo: String) -> Result<(), String> {
    let installed = tauri::async_runtime::spawn_blocking(gh_available)
        .await
        .unwrap_or(false);
    if !installed {
        return Err("gh 未安装".to_string());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        match Command::new("gh")
            .arg("auth")
            .arg("setup-git")
            .current_dir(&repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
            Err(e) => Err(format!("setup-git 失败: {e}")),
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}

/// 退出 GitHub 登录（强制登出所有账号，无交互）
#[command]
async fn gh_logout() -> Result<String, String> {
    let installed = tauri::async_runtime::spawn_blocking(gh_available)
        .await
        .unwrap_or(false);
    if !installed {
        return Err("gh 未安装".to_string());
    }
    let result = tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // 先用 gh auth status --json 获取所有已登录账号（gh 2.40+ 支持多账号）
        // 没有账号或命令失败时直接返回成功（视为已登出）
        let status_out = Command::new("gh")
            .args(["auth", "status", "--json", "host,oauthToken,accounts"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("执行 gh auth status 失败: {e}"))?;

        let mut logged_hosts: std::collections::HashSet<String> = std::collections::HashSet::new();
        if status_out.status.success() {
            let status_json = String::from_utf8_lossy(&status_out.stdout).trim().to_string();
            // 简易解析：从 JSON 中找 "host":"..." 字段
            // 避免引入 serde_json 依赖，直接字符串查找
            let mut rest = status_json.as_str();
            while let Some(pos) = rest.find("\"host\"") {
                rest = &rest[pos + 6..];
                if let Some(colon) = rest.find(':') {
                    rest = &rest[colon + 1..];
                    if let Some(start) = rest.find('"') {
                        rest = &rest[start + 1..];
                        if let Some(end) = rest.find('"') {
                            logged_hosts.insert(rest[..end].to_string());
                            rest = &rest[end + 1..];
                        }
                    }
                }
            }
        }

        // fallback：如果没解析出 host，默认尝试登出 github.com
        if logged_hosts.is_empty() {
            logged_hosts.insert("github.com".to_string());
        }

        let mut last_msg = String::new();
        let mut any_err = false;
        for host in &logged_hosts {
            let out = Command::new("gh")
                .args(["auth", "logout", "--hostname", host])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("执行 gh auth logout 失败 ({host}): {e}"))?;
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let combined = if stdout.is_empty() { stderr } else { stdout };
            if !combined.is_empty() {
                last_msg = combined.clone();
            }
            // gh 2.40+ 在多账号场景下 logout 会进入交互选择，需要用 --user 指定
            // 这里用循环直到该 host 所有账号都登出（最多重试 5 次防止死循环）
            if out.status.success() {
                // 该 host 可能还有其他账号，再查一次
                for _ in 0..5 {
                    let still = Command::new("gh")
                        .args(["auth", "status"])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .creation_flags(CREATE_NO_WINDOW)
                        .output();
                    match still {
                        Ok(o) => {
                            let stdout_str = String::from_utf8_lossy(&o.stdout).to_string();
                            let stderr_str = String::from_utf8_lossy(&o.stderr).to_string();
                            let combined = format!("{stdout_str}{stderr_str}");
                            if !combined.to_lowercase().contains(host.as_str()) {
                                break;
                            }
                            // 还有账号，继续 logout
                            let _ = Command::new("gh")
                                .args(["auth", "logout", "--hostname", host])
                                .stdout(Stdio::piped())
                                .stderr(Stdio::piped())
                                .creation_flags(CREATE_NO_WINDOW)
                                .output();
                        }
                        Err(_) => break,
                    }
                }
            } else if combined.to_lowercase().contains("not logged") {
                // 视为已登出
            } else {
                any_err = true;
            }
        }
        if any_err {
            Err(last_msg)
        } else {
            Ok(if last_msg.is_empty() {
                "已登出所有账号".to_string()
            } else {
                last_msg
            })
        }
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}

/// 全局保存 gh auth login PowerShell 子进程句柄，用于取消登录时终止
static GH_LOGIN_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);

/// 启动新 PowerShell 窗口运行 gh auth login（交互式登录必须在真实终端中完成）
/// 返回子进程 PID，前端可用于追踪
#[command]
async fn gh_login_interactive() -> Result<u32, String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        // 启动新 PowerShell 窗口运行 gh auth login
        // 优化启动速度：直接用 mode con: 设置 cols/lines，标题用于后续查找
        // 不再用 Add-Type + GetConsoleWindow（编译 Add-Type 慢）
        // 登录完成（成功/失败）后延时自动关闭，无需用户按回车
        // 先循环 gh auth logout 清除所有旧 token，避免旧登录状态干扰 gh_wait_login 轮询
        let script = "$Host.UI.RawUI.WindowTitle = 'GitHub CLI 登录'; mode con: cols=100 lines=30; Write-Host '===== GitHub CLI 登录 =====' -ForegroundColor Cyan; Write-Host '正在清除所有旧的登录状态...' -ForegroundColor Yellow; for ($i=0; $i -lt 10; $i++) { $null = gh auth logout --hostname github.com 2>&1; if ($LASTEXITCODE -ne 0) { break } }; Write-Host '（旧登录已清除或无登录）' -ForegroundColor DarkGray; Write-Host ''; Write-Host '请在下方交互式完成 GitHub 授权流程' -ForegroundColor Yellow; Write-Host ''; gh auth login; if ($LASTEXITCODE -eq 0) { Write-Host ''; Write-Host '登录成功，正在切换账号...' -ForegroundColor Green; for ($i=0; $i -lt 5; $i++) { $null = gh auth switch --hostname github.com 2>&1; if ($LASTEXITCODE -eq 0) { break } }; Write-Host '登录成功，3 秒后窗口将自动关闭...' -ForegroundColor Green; Start-Sleep -Seconds 3 } else { Write-Host ''; Write-Host '登录失败或已取消，10 秒后窗口将自动关闭...' -ForegroundColor Red; Start-Sleep -Seconds 10 }";
        let child = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .creation_flags(0)
            .spawn()
            .map_err(|e| format!("启动登录窗口失败: {e}"))?;
        let pid = child.id();
        // 保存到全局，便于 cancel 时终止
        if let Ok(mut guard) = GH_LOGIN_CHILD.lock() {
            *guard = Some(child);
        }
        Ok::<u32, String>(pid)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 取消正在进行的 gh auth login：杀掉 PowerShell 子进程及其子进程树
#[command]
async fn gh_cancel_login() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut killed_any = false;
        if let Ok(mut guard) = GH_LOGIN_CHILD.lock() {
            if let Some(mut child) = guard.take() {
                let pid = child.id();
                // 用 taskkill /T /F 杀整棵进程树（PowerShell + gh 子进程）
                let _ = Command::new("taskkill")
                    .args(["/PID", &pid.to_string(), "/T", "/F"])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
                // 也尝试 kill Child
                let _ = child.kill();
                killed_any = true;
            }
        }
        // 兜底：通过窗口标题查找并关闭（万一 PID 没保存成功）
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like '*GitHub CLI*' } | Stop-Process -Force -ErrorAction SilentlyContinue",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        if killed_any {
            Ok(())
        } else {
            Ok(()) // 即使没杀到也不报错，前端只要关闭对话框即可
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 轮询 gh api user 获取登录用户名，等待用户完成登录
#[command]
async fn gh_wait_login(timeout_secs: u64) -> Result<String, String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        if std::time::Instant::now() > deadline {
            return Err("等待登录超时".to_string());
        }
        let user = tauri::async_runtime::spawn_blocking(|| {
            use std::os::windows::process::CommandExt;
            use std::process::{Command, Stdio};
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            // 用 gh api user --jq .login 直接获取登录用户名
            // 未登录时会失败（非零退出码），已登录时返回纯用户名
            match Command::new("gh")
                .args(["api", "user", "--jq", ".login"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output()
            {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).trim().to_string()
                }
                _ => String::new(),
            }
        })
        .await
        .map_err(|e| e.to_string())?;

        if !user.is_empty() {
            return Ok(user);
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

/// 设置 git 用户名和邮箱（全局配置）
#[command]
async fn git_config_user(name: String, email: String) -> Result<(), String> {
    if name.trim().is_empty() || email.trim().is_empty() {
        return Err("用户名和邮箱不能为空".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let run = |key: &str, val: &str| -> Result<(), String> {
            let out = Command::new("git")
                .args(["config", "--global", key, val])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("执行 git config 失败: {e}"))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(if err.is_empty() {
                    format!("git config {} 失败", key)
                } else {
                    err
                });
            }
            Ok(())
        };

        run("user.name", &name)?;
        run("user.email", &email)?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 读取当前 git 全局配置的 user.name 和 user.email
#[command]
async fn git_get_user_config() -> Result<(String, String), String> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let read = |key: &str| -> String {
            let out = Command::new("git")
                .args(["config", "--global", "--get", key])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            match out {
                Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
                _ => String::new(),
            }
        };

        Ok((read("user.name"), read("user.email")))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 一键同时安装 git 和 gh（winget 串行执行）
#[command]
async fn install_git_and_gh() -> Result<String, String> {
    let result = tauri::async_runtime::spawn_blocking(|| {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        let install_one = |id: &str| -> Result<String, String> {
            let out = Command::new("winget")
                .args([
                    "install",
                    "--id",
                    id,
                    "-e",
                    "--source",
                    "winget",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                    "--silent",
                    "--disable-interactivity",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        "winget 不可用，请手动安装".to_string()
                    } else {
                        format!("启动 winget 失败: {e}")
                    }
                })?;
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                Ok(stdout)
            } else {
                let tail: String = if stderr.trim().len() > stdout.trim().len() {
                    stderr
                } else {
                    stdout
                }
                .lines()
                .rev()
                .take(6)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
                Err(if tail.trim().is_empty() {
                    format!("安装 {} 失败", id)
                } else {
                    tail
                })
            }
        };

        // 先安装 git，再安装 gh（gh 依赖 git）
        let git_out = install_one("Git.Git")?;
        let gh_out = install_one("GitHub.cli")?;
        Ok(format!("{}\n{}", git_out, gh_out))
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}


// ========== 系统优化 ==========

/// 检测当前进程是否以管理员权限运行
#[command]
fn is_admin() -> bool {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token: HANDLE = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut ret = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret,
        ).is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
    }
}

/// 以管理员权限重新启动当前应用（通过 ShellExecute runas）
#[command]
fn relaunch_as_admin() -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_str = exe.to_string_lossy().to_string();
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process -FilePath '{}' -Verb RunAs",
                exe_str.replace('\'', "''")
            ),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
enum RegKind {
    Dword,
    String,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RegOp {
    hive: String,
    path: String,
    name: String,
    #[allow(dead_code)]
    kind: RegKind,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OptimizeItem {
    key: String,
    title: String,
    desc: String,
    reg: Vec<RegOp>,
    service: Option<String>,
}

#[derive(serde::Serialize)]
struct OptimizeState {
    key: String,
    enabled: bool,
}

fn reg_root(name: &str) -> Option<winreg::RegKey> {
    use winreg::enums::*;
    Some(match name {
        "HKLM" => HKEY_LOCAL_MACHINE,
        "HKCU" => HKEY_CURRENT_USER,
        _ => return None,
    })
    .map(|h| winreg::RegKey::predef(h))
}

fn read_reg_dword(hive: &str, path: &str, name: &str) -> Option<u32> {
    let root = reg_root(hive)?;
    let key = root.open_subkey(path).ok()?;
    key.get_value::<u32, _>(name).ok()
}

fn read_reg_string(hive: &str, path: &str, name: &str) -> Option<String> {
    let root = reg_root(hive)?;
    let key = root.open_subkey(path).ok()?;
    key.get_value::<String, _>(name).ok()
}

fn write_reg_dword(hive: &str, path: &str, name: &str, value: u32) -> Result<(), String> {
    let root = reg_root(hive).ok_or("无效的 hive")?;
    let (key, _disp) = root.create_subkey(path).map_err(|e| e.to_string())?;
    key.set_value(name, &value).map_err(|e| e.to_string())?;
    Ok(())
}

fn write_reg_string(hive: &str, path: &str, name: &str, value: &str) -> Result<(), String> {
    let root = reg_root(hive).ok_or("无效的 hive")?;
    let (key, _disp) = root.create_subkey(path).map_err(|e| e.to_string())?;
    key.set_value(name, &value).map_err(|e| e.to_string())?;
    Ok(())
}

fn control_service(name: &str, stop: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let action = if stop { "stop" } else { "start" };
    let out = Command::new("sc")
        .args([action, name])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() || stdout.contains("FAILED") || stdout.contains("拒绝") {
        let detail = if !stdout.trim().is_empty() { stdout.trim() } else { stderr.trim() };
        let hint = if stdout.contains("access") || stdout.contains("5") || stdout.contains("拒绝") {
            "（需要管理员权限，且部分受保护服务需先禁用其启动类型）"
        } else {
            ""
        };
        return Err(format!("{}{}", detail, if hint.is_empty() { "" } else { hint }));
    }
    Ok(())
}

/// 设置服务启动类型（disabled=禁用 / auto=自动 / demand=手动）
fn config_service_start(name: &str, mode: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = Command::new("sc")
        .args(["config", name, "start=", mode])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() || stdout.contains("FAILED") || stdout.contains("拒绝") {
        let detail = if !stdout.trim().is_empty() { stdout.trim() } else { stderr.trim() };
        return Err(detail.to_string());
    }
    Ok(())
}

fn is_service_running(name: &str) -> bool {
    use std::process::Command;
    use std::os::windows::process::CommandExt;
    Command::new("sc")
        .args(["query", name])
        .creation_flags(0x0800_0000)
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_lowercase();
            s.contains("running")
        })
        .unwrap_or(false)
}

/// 优化项配置：每一项定义注册表/服务，前端按 key 查询状态
fn optimize_items() -> Vec<OptimizeItem> {
    vec![
        OptimizeItem {
            key: "smartscreen".into(),
            title: "SmartScreen".into(),
            desc: "应用与文件信誉检查".into(),
            reg: vec![
                RegOp {
                    hive: "HKLM".into(),
                    path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer".into(),
                    name: "SmartScreenEnabled".into(),
                    kind: RegKind::String,
                },
                RegOp {
                    hive: "HKCU".into(),
                    path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppHost".into(),
                    name: "EnableWebContentEvaluation".into(),
                    kind: RegKind::Dword,
                },
            ],
            service: None,
        },
        OptimizeItem {
            key: "uac".into(),
            title: "UAC 提示".into(),
            desc: "用户账户控制弹窗".into(),
            reg: vec![RegOp {
                hive: "HKLM".into(),
                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
                name: "EnableLUA".into(),
                kind: RegKind::Dword,
            }],
            service: None,
        },
        OptimizeItem {
            key: "amsi".into(),
            title: "AMSI".into(),
            desc: "反恶意软件扫描接口".into(),
            reg: vec![RegOp {
                hive: "HKLM".into(),
                path: r"SOFTWARE\Microsoft\AMSI\Provider".into(),
                name: "Enabled".into(),
                kind: RegKind::Dword,
            }],
            service: None,
        },
        OptimizeItem {
            key: "stickykeys".into(),
            title: "粘滞键提示".into(),
            desc: "连按 Shift 弹出粘滞键".into(),
            reg: vec![
                RegOp {
                    hive: "HKCU".into(),
                    path: r"Control Panel\Accessibility\StickyKeys".into(),
                    name: "Flags".into(),
                    kind: RegKind::String,
                },
            ],
            service: None,
        },
    ]
}

/// 查询所有优化项的当前状态
/// 约定：enabled=true 表示该功能已开启（需关闭以优化），false 表示已关闭
#[command]
fn optimize_states() -> Vec<OptimizeState> {
    optimize_items()
        .iter()
        .map(|item| {
            let enabled = if let Some(svc) = &item.service {
                is_service_running(svc)
            } else if !item.reg.is_empty() {
                let r = &item.reg[0];
                match r.kind {
                    RegKind::Dword => read_reg_dword(&r.hive, &r.path, &r.name).unwrap_or(0) != 0,
                    RegKind::String => read_reg_string(&r.hive, &r.path, &r.name)
                        .map(|v| !v.is_empty() && v != "Off" && v != "0")
                        .unwrap_or(false),
                }
            } else {
                false
            };
            OptimizeState {
                key: item.key.clone(),
                enabled,
            }
        })
        .collect()
}

/// 设置某个优化项（enable=true 开启功能，false 关闭以优化）
#[command]
async fn optimize_set(key: String, enable: bool) -> Result<(), String> {
    let item = optimize_items().into_iter().find(|i| i.key == key)
        .ok_or_else(|| "未找到优化项".to_string())?;
    let task = tauri::async_runtime::spawn_blocking(move || {
        if enable {
            if let Some(svc) = &item.service {
                let _ = config_service_start(svc, "auto");
                let _ = control_service(svc, false);
            }
            for r in &item.reg {
                match r.kind {
                    RegKind::Dword => write_reg_dword(&r.hive, &r.path, &r.name, 1)?,
                    RegKind::String => write_reg_string(&r.hive, &r.path, &r.name, "On")?,
                }
            }
        } else {
            for r in &item.reg {
                match r.kind {
                    RegKind::Dword => write_reg_dword(&r.hive, &r.path, &r.name, 0)?,
                    RegKind::String => write_reg_string(&r.hive, &r.path, &r.name, "Off")?,
                }
            }
            if let Some(svc) = &item.service {
                let _ = config_service_start(svc, "disabled");
                let _ = control_service(svc, true);
            }
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?;
    task
}

#[repr(C)]
#[allow(non_snake_case)]
struct AccentPolicy {
    AccentState: u32,
    AccentFlags: u32,
    GradientColor: u32,
    AnimationId: u32,
}

#[repr(C)]
#[allow(non_snake_case)]
struct WindowCompositionAttribData {
    Attrib: u32,
    pvData: *mut std::ffi::c_void,
    cbData: usize,
}

const WCA_ACCENT_POLICY: u32 = 19;
const ACCENT_DISABLE: u32 = 0;
const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;

#[command]
fn set_window_backdrop(app: tauri::AppHandle, backdrop: u32) -> Result<(), String> {
    use tauri::Manager;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    };

    let window = app.get_webview_window("main").ok_or("未找到主窗口")?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;

    if backdrop != 1 {
        let value = backdrop as i32;
        let r = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &value as *const _ as *const _,
                std::mem::size_of::<i32>() as u32,
            )
        };
        if r.is_ok() {
            let _ = set_win32_accent(hwnd, 0);
            return Ok(());
        }
    }

    let accent_state = match backdrop {
        2 => ACCENT_ENABLE_BLURBEHIND,
        3 => ACCENT_ENABLE_ACRYLICBLURBEHIND,
        _ => ACCENT_DISABLE,
    };
    set_win32_accent(hwnd, accent_state)?;
    Ok(())
}

#[command]
fn refresh_window_backdrop(app: tauri::AppHandle, backdrop: u32) -> Result<(), String> {
    use tauri::Manager;
    use windows::Win32::Graphics::Dwm::DwmFlush;
    use windows::Win32::Graphics::Gdi::InvalidateRect;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOZORDER,
    };
    let _ = backdrop;
    let window = app.get_webview_window("main").ok_or("未找到主窗口")?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let _ = InvalidateRect(Some(hwnd), None, true);
        let _ = DwmFlush();
    }
    Ok(())
}

fn set_win32_accent(hwnd: windows::Win32::Foundation::HWND, accent_state: u32) -> Result<(), String> {
    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

    type SetWindowCompositionAttribute =
        unsafe extern "system" fn(windows::Win32::Foundation::HWND, *const WindowCompositionAttribData) -> i32;

    let user32 = unsafe { GetModuleHandleW(windows::core::w!("user32.dll")) }
        .map_err(|e| format!("无法获取 user32: {e}"))?;
    let addr = unsafe { GetProcAddress(user32, windows::core::s!("SetWindowCompositionAttribute")) }
        .ok_or_else(|| "找不到 SetWindowCompositionAttribute".to_string())?;
    let func: SetWindowCompositionAttribute = unsafe { std::mem::transmute(addr) };

    let policy = AccentPolicy {
        AccentState: accent_state,
        AccentFlags: 0,
        GradientColor: 0x99000000,
        AnimationId: 0,
    };
    let data = WindowCompositionAttribData {
        Attrib: WCA_ACCENT_POLICY,
        pvData: &policy as *const _ as *mut _,
        cbData: std::mem::size_of::<AccentPolicy>(),
    };
    let ok = unsafe { func(hwnd, &data) };
    if ok == 0 {
        return Err("SetWindowCompositionAttribute 返回 0".to_string());
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            scan_junk,
            clean_junk,
            get_hardware_info,
            get_performance_stats,
            list_processes,
            process_icons,
            kill_process,
            get_foreground_window_pid,
            suspend_process,
            resume_process,
            get_ppl_protection,
            restart_as_admin,
            check_git,
            install_git,
            git_default_dir,
            git_repo_root,
            git_status,
            git_log,
            git_add,
            git_unstage,
            git_commit,
            git_branches,
            git_push,
            git_revert,
            git_checkout,
            git_fetch,
            git_pull,
            git_clone,
            git_init,
            pick_folder,
            gh_auth_state,
            gh_login_web,
            gh_logout,
            gh_login_interactive,
            gh_cancel_login,
            gh_wait_login,
            gh_setup_git,
            install_git_and_gh,
            git_config_user,
            git_get_user_config,
            is_admin,
            relaunch_as_admin,
            optimize_states,
            optimize_set,
            set_window_backdrop,
            refresh_window_backdrop
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
