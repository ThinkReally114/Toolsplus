   1→// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
   2→use std::fs;
   3→use std::path::Path;
   4→use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
   5→use std::sync::OnceLock;
   6→use tauri::command;
   7→use windows::Win32::Foundation::{LRESULT, LPARAM, WPARAM};
   8→use windows::Win32::UI::WindowsAndMessaging::HHOOK;
   9→
  10→/// 扫描结果项
  11→#[derive(serde::Serialize)]
  12→struct JunkItem {
  13→    key: String,
  14→    path: String,
  15→    size: u64,
  16→}
  17→
  18→/// 清理目标项（从前端传入）
  19→#[derive(serde::Deserialize)]
  20→struct CleanTarget {
  21→    key: String,
  22→    path: String,
  23→    size: u64,
  24→}
  25→
  26→/// 递归计算目录大小（字节数），遇到无权限或不存在返回 0
  27→fn dir_size(path: &Path) -> u64 {
  28→    const MAX_DEPTH: u32 = 32;
  29→    const MAX_ENTRIES: usize = 5_000_000;
  30→    if !path.exists() {
  31→        return 0;
  32→    }
  33→    let total = AtomicU64::new(0);
  34→    let visited = AtomicU64::new(0);
  35→    let stack = vec![path.to_path_buf()];
  36→    let mut current = stack;
  37→    let mut depth_stack = vec![0u32];
  38→    let mut top_subdirs: Vec<std::path::PathBuf> = Vec::new();
  39→    while let Some(dir) = current.pop() {
  40→        let depth = depth_stack.pop().unwrap_or(0);
  41→        if depth > MAX_DEPTH {
  42→            continue;
  43→        }
  44→        if visited.load(Ordering::Relaxed) > MAX_ENTRIES as u64 {
  45→            break;
  46→        }
  47→        let entries = match fs::read_dir(&dir) {
  48→            Ok(e) => e,
  49→            Err(_) => continue,
  50→        };
  51→        for entry in entries.flatten() {
  52→            visited.fetch_add(1, Ordering::Relaxed);
  53→            let p = entry.path();
  54→            let ft = match entry.file_type() {
  55→                Ok(t) => t,
  56→                Err(_) => continue,
  57→            };
  58→            if ft.is_dir() {
  59→                if depth == 0 {
  60→                    top_subdirs.push(p);
  61→                } else {
  62→                    current.push(p);
  63→                    depth_stack.push(depth + 1);
  64→                }
  65→            } else if ft.is_file() {
  66→                if let Ok(meta) = entry.metadata() {
  67→                    total.fetch_add(meta.len(), Ordering::Relaxed);
  68→                }
  69→            }
  70→        }
  71→    }
  72→
  73→    if top_subdirs.is_empty() {
  74→        return total.load(Ordering::Relaxed);
  75→    }
  76→
  77→    std::thread::scope(|s| {
  78→        for subdir in &top_subdirs {
  79→            let total = &total;
  80→            let visited = &visited;
  81→            s.spawn(move || {
  82→                let local_total = AtomicU64::new(0);
  83→                let local_visited = AtomicU64::new(0);
  84→                let stack = vec![subdir.clone()];
  85→                let mut current = stack;
  86→                let mut depth_stack = vec![1u32];
  87→                while let Some(dir) = current.pop() {
  88→                    let depth = depth_stack.pop().unwrap_or(0);
  89→                    if depth > MAX_DEPTH {
  90→                        continue;
  91→                    }
  92→                    if visited.load(Ordering::Relaxed) + local_visited.load(Ordering::Relaxed)
  93→                        > MAX_ENTRIES as u64
  94→                    {
  95→                        break;
  96→                    }
  97→                    let entries = match fs::read_dir(&dir) {
  98→                        Ok(e) => e,
  99→                        Err(_) => continue,
 100→                    };
 101→                    for entry in entries.flatten() {
 102→                        local_visited.fetch_add(1, Ordering::Relaxed);
 103→                        let p = entry.path();
 104→                        let ft = match entry.file_type() {
 105→                            Ok(t) => t,
 106→                            Err(_) => continue,
 107→                        };
 108→                        if ft.is_dir() {
 109→                            current.push(p);
 110→                            depth_stack.push(depth + 1);
 111→                        } else if ft.is_file() {
 112→                            if let Ok(meta) = entry.metadata() {
 113→                                local_total.fetch_add(meta.len(), Ordering::Relaxed);
 114→                            }
 115→                        }
 116→                    }
 117→                }
 118→                total.fetch_add(local_total.load(Ordering::Relaxed), Ordering::Relaxed);
 119→                visited.fetch_add(local_visited.load(Ordering::Relaxed), Ordering::Relaxed);
 120→            });
 121→        }
 122→    });
 123→
 124→    total.load(Ordering::Relaxed)
 125→}
 126→
 127→/// 展开环境变量并返回规范化的路径字符串
 128→fn expand_path(raw: &str) -> String {
 129→    // 简单展开 %TEMP% / %LOCALAPPDATA% / %SYSTEMROOT% 等
 130→    let mut out = raw.to_string();
 131→    for (var, val) in [
 132→        ("TEMP", std::env::var("TEMP").unwrap_or_default()),
 133→        ("TMP", std::env::var("TMP").unwrap_or_default()),
 134→        (
 135→            "LOCALAPPDATA",
 136→            std::env::var("LOCALAPPDATA").unwrap_or_default(),
 137→        ),
 138→        ("APPDATA", std::env::var("APPDATA").unwrap_or_default()),
 139→        (
 140→            "SYSTEMROOT",
 141→            std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".to_string()),
 142→        ),
 143→        (
 144→            "USERPROFILE",
 145→            std::env::var("USERPROFILE").unwrap_or_default(),
 146→        ),
 147→    ] {
 148→        if val.is_empty() {
 149→            continue;
 150→        }
 151→        let from = format!("%{}%", var);
 152→        out = out.replace(&from, &val);
 153→    }
 154→    out
 155→}
 156→
 157→/// 扫描 C 盘常见可回收垃圾（异步：放到线程池避免阻塞 UI）
 158→#[command]
 159→async fn scan_junk() -> Vec<JunkItem> {
 160→    tauri::async_runtime::spawn_blocking(|| {
 161→        let targets: [(&str, &str); 5] = [
 162→            ("temp", "%TEMP%"),
 163→            ("windowsTemp", "%SYSTEMROOT%\\Temp"),
 164→            ("recycle", "C:\\$Recycle.Bin"),
 165→            ("prefetch", "%SYSTEMROOT%\\Prefetch"),
 166→            ("logs", "%SYSTEMROOT%\\Logs"),
 167→        ];
 168→
 169→        let expanded: Vec<(String, String)> = targets
 170→            .iter()
 171→            .map(|(key, raw)| (key.to_string(), expand_path(raw)))
 172→            .collect();
 173→
 174→        std::thread::scope(|s| {
 175→            let handles: Vec<_> = expanded
 176→                .iter()
 177→                .map(|(key, path)| {
 178→                    let key = key.clone();
 179→                    let path = path.clone();
 180→                    s.spawn(move || {
 181→                        let size = dir_size(std::path::Path::new(&path));
 182→                        (key, path, size)
 183→                    })
 184→                })
 185→                .collect();
 186→
 187→            handles
 188→                .into_iter()
 189→                .filter_map(|h| h.join().ok())
 190→                .map(|(key, path, size)| JunkItem { key, path, size })
 191→                .collect()
 192→        })
 193→    })
 194→    .await
 195→    .unwrap_or_default()
 196→}
 197→
 198→/// 递归删除目录下所有内容（保留目录本身），返回是否无错误完成
 199→fn clean_dir_contents(path: &Path) -> bool {
 200→    if !path.exists() {
 201→        return false;
 202→    }
 203→    let entries = match fs::read_dir(path) {
 204→        Ok(e) => e,
 205→        Err(_) => return false,
 206→    };
 207→
 208→    let mut file_paths: Vec<std::path::PathBuf> = Vec::new();
 209→    let mut dir_paths: Vec<std::path::PathBuf> = Vec::new();
 210→    let mut all_ok = true;
 211→
 212→    for entry in entries.flatten() {
 213→        let p = entry.path();
 214→        let ft = match entry.file_type() {
 215→            Ok(t) => t,
 216→            Err(_) => {
 217→                all_ok = false;
 218→                continue;
 219→            }
 220→        };
 221→        if ft.is_dir() {
 222→            dir_paths.push(p);
 223→        } else if ft.is_file() {
 224→            file_paths.push(p);
 225→        }
 226→    }
 227→
 228→    for p in file_paths {
 229→        if fs::remove_file(&p).is_err() {
 230→            all_ok = false;
 231→        }
 232→    }
 233→
 234→    if !dir_paths.is_empty() {
 235→        let ok_flags = std::sync::Mutex::new(vec![false; dir_paths.len()]);
 236→        std::thread::scope(|s| {
 237→            for (i, p) in dir_paths.iter().enumerate() {
 238→                let ok_flags = &ok_flags;
 239→                s.spawn(move || {
 240→                    let ok = fs::remove_dir_all(p).is_ok();
 241→                    ok_flags.lock().unwrap()[i] = ok;
 242→                });
 243→            }
 244→        });
 245→        if ok_flags.lock().unwrap().iter().any(|&ok| !ok) {
 246→            all_ok = false;
 247→        }
 248→    }
 249→
 250→    all_ok
 251→}
 252→
 253→/// 清空回收站（调用系统 SHEmptyRecycleBinW，无需管理员权限）
 254→fn empty_recycle_bin() -> bool {
 255→    use windows::core::PCWSTR;
 256→    use windows::Win32::UI::Shell::{
 257→        SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND,
 258→    };
 259→    unsafe {
 260→        SHEmptyRecycleBinW(
 261→            None,
 262→            PCWSTR::null(),
 263→            SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND,
 264→        )
 265→    }
 266→    .is_ok()
 267→}
 268→
 269→/// 清理选中的垃圾项（异步：放到线程池避免阻塞 UI）
 270→#[command]
 271→async fn clean_junk(targets: Vec<CleanTarget>) -> u64 {
 272→    tauri::async_runtime::spawn_blocking(move || {
 273→        std::thread::scope(|s| {
 274→            let handles: Vec<_> = targets
 275→                .into_iter()
 276→                .map(|t| {
 277→                    s.spawn(move || -> u64 {
 278→                        let path = Path::new(&t.path);
 279→                        if t.key == "recycle" {
 280→                            if empty_recycle_bin() {
 281→                                t.size
 282→                            } else {
 283→                                0
 284→                            }
 285→                        } else if clean_dir_contents(path) {
 286→                            t.size
 287→                        } else {
 288→                            0
 289→                        }
 290→                    })
 291→                })
 292→                .collect();
 293→
 294→            handles
 295→                .into_iter()
 296→                .filter_map(|h| h.join().ok())
 297→                .sum()
 298→        })
 299→    })
 300→    .await
 301→    .unwrap_or(0)
 302→}
 303→
 304→/// 硬件信息
 305→#[derive(serde::Serialize)]
 306→struct HardwareInfo {
 307→    motherboard: String,
 308→    cpu: String,
 309→    gpu: String,
 310→    ram_total: String,
 311→    ram_used: String,
 312→    ram_speed: String,
 313→    ram_manufacturer: String,
 314→    gpu_vram_total: String,
 315→    gpu_driver: String,
 316→    disks: Vec<DiskInfo>,
 317→}
 318→
 319→#[derive(serde::Serialize, Clone)]
 320→struct DiskInfo {
 321→    name: String,
 322→    model: String,
 323→    total: String,
 324→    free: String,
 325→    disk_type: String,
 326→    interface: String,
 327→}
 328→
 329→/// 格式化字节数为人类可读
 330→fn fmt_bytes(bytes: u64) -> String {
 331→    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
 332→    let mut size = bytes as f64;
 333→    let mut unit = 0;
 334→    while size >= 1024.0 && unit < UNITS.len() - 1 {
 335→        size /= 1024.0;
 336→        unit += 1;
 337→    }
 338→    format!("{:.1} {}", size, UNITS[unit])
 339→}
 340→
 341→static SYS: std::sync::LazyLock<std::sync::Mutex<sysinfo::System>> =
 342→    std::sync::LazyLock::new(|| std::sync::Mutex::new(sysinfo::System::new()));
 343→
 344→static BOARD_INFO: std::sync::OnceLock<String> = std::sync::OnceLock::new();
 345→static GPU_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
 346→
 347→#[derive(Clone, Default)]
 348→struct GpuLiveStats {
 349→    usage: Option<f32>,
 350→    vram_total: Option<u64>,
 351→    vram_used: Option<u64>,
 352→}
 353→
 354→static GPU_LIVE: std::sync::LazyLock<std::sync::Mutex<GpuLiveStats>> =
 355→    std::sync::LazyLock::new(|| std::sync::Mutex::new(GpuLiveStats::default()));
 356→
 357→static GPU_WORKER: std::sync::LazyLock<()> = std::sync::LazyLock::new(|| {
 358→    std::thread::spawn(|| loop {
 359→        let (usage, vram_total, vram_used) = query_gpu_stats();
 360→        if let Ok(mut c) = GPU_LIVE.lock() {
 361→            c.usage = usage;
 362→            c.vram_total = vram_total;
 363→            c.vram_used = vram_used;
 364→        }
 365→        std::thread::sleep(std::time::Duration::from_secs(3));
 366→    });
 367→});
 368→
 369→/// 查询硬件信息：主板/CPU/显卡/RAM
 370→#[tauri::command]
 371→fn get_hardware_info() -> HardwareInfo {
 372→    let mut sys = SYS.lock().unwrap();
 373→    sys.refresh_cpu_all();
 374→    sys.refresh_memory();
 375→
 376→    let cpus = sys.cpus();
 377→    let cpu = if !cpus.is_empty() {
 378→        format!("{} ({} 核心)", cpus[0].brand(), cpus.len())
 379→    } else {
 380→        "未知".to_string()
 381→    };
 382→
 383→    let ram_total = fmt_bytes(sys.total_memory());
 384→    let ram_used = fmt_bytes(sys.used_memory());
 385→    drop(sys);
 386→
 387→    let motherboard = BOARD_INFO.get_or_init(query_motherboard).clone();
 388→    let gpu = GPU_NAME.get_or_init(query_gpu).clone();
 389→    let (ram_speed, ram_manufacturer) = query_ram_detail();
 390→    let (gpu_vram_total, gpu_driver) = query_gpu_detail();
 391→    let disks = query_disks();
 392→
 393→    HardwareInfo {
 394→        motherboard,
 395→        cpu,
 396→        gpu,
 397→        ram_total,
 398→        ram_used,
 399→        ram_speed,
 400→        ram_manufacturer,
 401→        gpu_vram_total,
 402→        gpu_driver,
 403→        disks,
 404→    }
 405→}
 406→
 407→/// 通过 WMI COM API 查询主板信息（原生，无需 PowerShell）
 408→fn query_motherboard() -> String {
 409→    use std::collections::HashMap;
 410→    use std::time::Duration;
 411→    let result = std::thread::scope(|s| {
 412→        s.spawn(|| {
 413→            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
 414→                .ok()?;
 415→            let results: Vec<HashMap<String, wmi::Variant>> = wmi
 416→                .raw_query("SELECT Manufacturer, Product FROM Win32_BaseBoard")
 417→                .ok()?;
 418→            let row = results.first()?;
 419→            let mfg = variant_to_string(row.get("Manufacturer"));
 420→            let product = variant_to_string(row.get("Product"));
 421→            Some(if mfg.is_empty() && product.is_empty() {
 422→                "未知".to_string()
 423→            } else if mfg.is_empty() {
 424→                product
 425→            } else if product.is_empty() {
 426→                mfg
 427→            } else {
 428→                format!("{} {}", mfg, product)
 429→            })
 430→        })
 431→        .join()
 432→        .ok()
 433→        .flatten()
 434→    });
 435→    result.unwrap_or_else(|| {
 436→        let script = "Get-CimInstance Win32_BaseBoard -ErrorAction SilentlyContinue | ForEach-Object { \"$($_.Manufacturer)|$($_.Product)\" }";
 437→        let output = run_powershell(script, Duration::from_secs(10));
 438→        let raw = parse_powershell_output(output);
 439→        let line = raw.lines().next().unwrap_or("").trim().to_string();
 440→        let (mfg, product) = match line.split_once('|') {
 441→            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
 442→            None => (String::new(), String::new()),
 443→        };
 444→        if mfg.is_empty() && product.is_empty() {
 445→            "未知".to_string()
 446→        } else if mfg.is_empty() {
 447→            product
 448→        } else if product.is_empty() {
 449→            mfg
 450→        } else {
 451→            format!("{} {}", mfg, product)
 452→        }
 453→    })
 454→}
 455→
 456→/// 通过 WMI COM API 查询显卡信息（原生，无需 PowerShell）
 457→fn query_gpu() -> String {
 458→    use std::collections::HashMap;
 459→    use std::time::Duration;
 460→    let result = std::thread::scope(|s| {
 461→        s.spawn(|| {
 462→            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
 463→                .ok()?;
 464→            let results: Vec<HashMap<String, wmi::Variant>> = wmi
 465→                .raw_query("SELECT Name FROM Win32_VideoController WHERE Name IS NOT NULL")
 466→                .ok()?;
 467→            let names: Vec<String> = results
 468→                .iter()
 469→                .filter_map(|row| {
 470→                    let n = variant_to_string(row.get("Name"));
 471→                    if n.is_empty() { None } else { Some(n) }
 472→                })
 473→                .collect();
 474→            Some(if names.is_empty() {
 475→                "未知".to_string()
 476→            } else {
 477→                names.join(" + ")
 478→            })
 479→        })
 480→        .join()
 481→        .ok()
 482→        .flatten()
 483→    });
 484→    result.unwrap_or_else(|| {
 485→        let output = run_powershell(
 486→            "Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Where-Object { $_.Name } | Select-Object -ExpandProperty Name",
 487→            Duration::from_secs(10),
 488→        );
 489→        let raw = parse_powershell_output(output);
 490→        let names: Vec<String> = raw
 491→            .lines()
 492→            .map(|l| l.trim().to_string())
 493→            .filter(|l| !l.is_empty())
 494→            .collect();
 495→        if names.is_empty() {
 496→            "未知".to_string()
 497→        } else {
 498→            names.join(" + ")
 499→        }
 500→    })
 501→}
 502→
 503→/// 查询显卡详情：显存总量（DXGI 原生 API）、驱动版本（WMI）
 504→fn query_gpu_detail() -> (String, String) {
 505→    use std::collections::HashMap;
 506→    use std::time::Duration;
 507→    let vram = if let Some(bytes) = query_gpu_vram_dxgi() {
 508→        fmt_bytes(bytes)
 509→    } else {
 510→        "未知".to_string()
 511→    };
 512→
 513→    let driver = {
 514→        let result = std::thread::scope(|s| {
 515→            s.spawn(|| {
 516→                let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
 517→                    .ok()?;
 518→                let results: Vec<HashMap<String, wmi::Variant>> = wmi
 519→                    .raw_query("SELECT DriverVersion FROM Win32_VideoController WHERE Name IS NOT NULL")
 520→                    .ok()?;
 521→                let row = results.first()?;
 522→                let d = variant_to_string(row.get("DriverVersion"));
 523→                Some(if d.is_empty() { "未知".to_string() } else { d })
 524→            })
 525→            .join()
 526→            .ok()
 527→            .flatten()
 528→        });
 529→        result.unwrap_or_else(|| {
 530→            let script = "Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Where-Object { $_.Name } | Select-Object -First 1 -ExpandProperty DriverVersion";
 531→            let output = run_powershell(script, Duration::from_secs(10));
 532→            let raw = parse_powershell_output(output);
 533→            let d = raw.trim().to_string();
 534→            if d.is_empty() { "未知".to_string() } else { d }
 535→        })
 536→    };
 537→
 538→    (vram, driver)
 539→}
 540→
 541→/// 将 WMI Variant 值转为 String
 542→fn variant_to_string(v: Option<&wmi::Variant>) -> String {
 543→    use wmi::Variant;
 544→    match v {
 545→        Some(Variant::String(s)) => s.trim().to_string(),
 546→        Some(Variant::I1(n)) => n.to_string(),
 547→        Some(Variant::I2(n)) => n.to_string(),
 548→        Some(Variant::I4(n)) => n.to_string(),
 549→        Some(Variant::I8(n)) => n.to_string(),
 550→        Some(Variant::UI1(n)) => n.to_string(),
 551→        Some(Variant::UI2(n)) => n.to_string(),
 552→        Some(Variant::UI4(n)) => n.to_string(),
 553→        Some(Variant::UI8(n)) => n.to_string(),
 554→        Some(Variant::R4(n)) => n.to_string(),
 555→        Some(Variant::R8(n)) => n.to_string(),
 556→        Some(Variant::Bool(b)) => b.to_string(),
 557→        _ => String::new(),
 558→    }
 559→}
 560→
 561→/// 通过 DXGI 枚举显卡获取真实显存（WMI AdapterRAM 为 32 位字段，4GB 以上会溢出）
 562→fn query_gpu_vram_dxgi() -> Option<u64> {
 563→    use windows::Win32::Graphics::Dxgi::{
 564→        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
 565→    };
 566→    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.ok()?;
 567→    for i in 0..8 {
 568→        let adapter = match unsafe { factory.EnumAdapters1(i) } {
 569→            Ok(a) => a,
 570→            Err(_) => break,
 571→        };
 572→        let desc = unsafe { adapter.GetDesc1() }.ok()?;
 573→        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
 574→            continue;
 575→        }
 576→        let name = String::from_utf16_lossy(&desc.Description);
 577→        if name.trim_end_matches('\0').trim().is_empty() {
 578→            continue;
 579→        }
 580→        if desc.DedicatedVideoMemory > 0 {
 581→            return Some(desc.DedicatedVideoMemory as u64);
 582→        }
 583→    }
 584→    None
 585→}
 586→
 587→/// 查询内存条信息：速率、品牌（原生 WMI COM API）
 588→fn query_ram_detail() -> (String, String) {
 589→    use std::collections::HashMap;
 590→    use std::time::Duration;
 591→    let result = std::thread::scope(|s| {
 592→        s.spawn(|| {
 593→            let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
 594→                .ok()?;
 595→            let results: Vec<HashMap<String, wmi::Variant>> = wmi
 596→                .raw_query("SELECT Speed, Manufacturer FROM Win32_PhysicalMemory")
 597→                .ok()?;
 598→            let row = results.first()?;
 599→            let speed_raw = variant_to_string(row.get("Speed"));
 600→            let mfg = variant_to_string(row.get("Manufacturer"));
 601→            let speed = speed_raw
 602→                .parse::<u64>()
 603→                .map(|s| format!("{} MHz", s))
 604→                .unwrap_or_else(|_| {
 605→                    if speed_raw.is_empty() {
 606→                        "未知".to_string()
 607→                    } else {
 608→                        speed_raw.clone()
 609→                    }
 610→                });
 611→            let brand = if mfg.is_empty() {
 612→                "未知".to_string()
 613→            } else {
 614→                mfg.trim_matches(char::is_control).trim().to_string()
 615→            };
 616→            Some((speed, brand))
 617→        })
 618→        .join()
 619→        .ok()
 620→        .flatten()
 621→    });
 622→    result.unwrap_or_else(|| {
 623→        let script = "Get-CimInstance Win32_PhysicalMemory -ErrorAction SilentlyContinue | Select-Object -First 1 | ForEach-Object { \"{0}|{1}\" -f $_.Speed, $_.Manufacturer }";
 624→        let output = run_powershell(script, Duration::from_secs(10));
 625→        let raw = parse_powershell_output(output);
 626→        let line = raw.lines().next().unwrap_or("").trim().to_string();
 627→        let mut parts = line.split('|');
 628→        let speed_raw = parts.next().unwrap_or("").trim().to_string();
 629→        let mfg = parts.next().unwrap_or("").trim().to_string();
 630→        let speed = speed_raw
 631→            .parse::<u64>()
 632→            .map(|s| format!("{} MHz", s))
 633→            .unwrap_or_else(|_| if speed_raw.is_empty() { "未知".to_string() } else { speed_raw });
 634→        let brand = if mfg.is_empty() { "未知".to_string() } else { mfg.trim_matches(char::is_control).trim().to_string() };
 635→        (speed, brand)
 636→    })
 637→}
 638→
 639→/// 查询磁盘信息（sysinfo 主路径 + WMI 原生 API 补充型号/接口）
 640→fn query_disks() -> Vec<DiskInfo> {
 641→    use std::collections::HashMap;
 642→    use sysinfo::Disks;
 643→    let mut result = Vec::new();
 644→    let disks = Disks::new_with_refreshed_list();
 645→    for d in disks.list() {
 646→        let name = d.name().to_string_lossy().to_string();
 647→        let total = fmt_bytes(d.total_space());
 648→        let free = fmt_bytes(d.available_space());
 649→        result.push(DiskInfo {
 650→            name,
 651→            model: String::new(),
 652→            total,
 653→            free,
 654→            disk_type: String::new(),
 655→            interface: String::new(),
 656→        });
 657→    }
 658→    if result.is_empty() {
 659→        let mut result_ext: Vec<DiskInfo> = Vec::new();
 660→        let wmi_list = std::thread::scope(|s| {
 661→            s.spawn(|| {
 662→                let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\CIMV2", wmi::COMLibrary::new().ok()?)
 663→                    .ok()?;
 664→                let results: Vec<HashMap<String, wmi::Variant>> = wmi
 665→                    .raw_query("SELECT Model, Size, InterfaceType FROM Win32_DiskDrive")
 666→                    .ok()?;
 667→                let mut list = Vec::new();
 668→                for row in results {
 669→                    let model = variant_to_string(row.get("Model"));
 670→                    let size_raw = variant_to_string(row.get("Size"));
 671→                    let iface = variant_to_string(row.get("InterfaceType"));
 672→                    let total = size_raw
 673→                        .parse::<u64>()
 674→                        .map(fmt_bytes)
 675→                        .unwrap_or_else(|_| "未知".to_string());
 676→                    list.push(DiskInfo {
 677→                        name: model.clone(),
 678→                        model,
 679→                        total,
 680→                        free: "—".to_string(),
 681→                        disk_type: String::new(),
 682→                        interface: iface,
 683→                    });
 684→                }
 685→                Some(list)
 686→            })
 687→            .join()
 688→            .ok()
 689→            .flatten()
 690→        });
 691→        if let Some(list) = wmi_list {
 692→            result_ext.extend(list);
 693→        }
 694→        if result_ext.is_empty() {
 695→            let script = "Get-CimInstance Win32_DiskDrive -ErrorAction SilentlyContinue | ForEach-Object { \"{0}|{1}|{2}\" -f $_.Model, $_.Size, $_.InterfaceType }";
 696→            let output = run_powershell(script, std::time::Duration::from_secs(10));
 697→            let raw = parse_powershell_output(output);
 698→            for line in raw.lines() {
 699→                let line = line.trim();
 700→                if line.is_empty() { continue; }
 701→                let mut parts = line.split('|');
 702→                let model = parts.next().unwrap_or("").trim().to_string();
 703→                let size_raw = parts.next().unwrap_or("").trim().to_string();
 704→                let iface = parts.next().unwrap_or("").trim().to_string();
 705→                let total = size_raw.parse::<u64>().map(fmt_bytes).unwrap_or_else(|_| "未知".to_string());
 706→                result_ext.push(DiskInfo {
 707→                    name: model.clone(),
 708→                    model,
 709→                    total,
 710→                    free: "—".to_string(),
 711→                    disk_type: String::new(),
 712→                    interface: iface,
 713→                });
 714→            }
 715→        }
 716→        result_ext
 717→    } else {
 718→        result
 719→    }
 720→}
 721→
 722→/// 带超时执行 PowerShell 命令，避免查询卡死
 723→fn run_powershell(
 724→    script: &str,
 725→    timeout: std::time::Duration,
 726→) -> Result<std::process::Output, std::io::Error> {
 727→    use std::io::ErrorKind;
 728→    use std::process::{Command, Stdio};
 729→    use std::time::Instant;
 730→
 731→    use std::os::windows::process::CommandExt;
 732→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
 733→
 734→    let mut child = Command::new("powershell")
 735→        .args(["-NoProfile", "-NonInteractive", "-Command", script])
 736→        .stdout(Stdio::piped())
 737→        .stderr(Stdio::piped())
 738→        .creation_flags(CREATE_NO_WINDOW)
 739→        .spawn()?;
 740→
 741→    let start = Instant::now();
 742→    let mut timed_out = false;
 743→    while child.try_wait()?.is_none() {
 744→        if start.elapsed() > timeout {
 745→            let _ = child.kill();
 746→            let _ = child.wait();
 747→            timed_out = true;
 748→            break;
 749→        }
 750→        std::thread::sleep(std::time::Duration::from_millis(15));
 751→    }
 752→    if timed_out {
 753→        return Err(std::io::Error::new(ErrorKind::TimedOut, "powershell timeout"));
 754→    }
 755→    child.wait_with_output()
 756→}
 757→
 758→/// 解析 PowerShell 命令输出为字符串
 759→fn parse_powershell_output(output: Result<std::process::Output, std::io::Error>) -> String {
 760→    match output {
 761→        Ok(o) => {
 762→            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
 763→            if stdout.trim().is_empty() {
 764→                String::from_utf8_lossy(&o.stderr).to_string()
 765→            } else {
 766→                stdout
 767→            }
 768→        }
 769→        Err(_) => String::new(),
 770→    }
 771→}
 772→
 773→/// 性能监测数据
 774→#[derive(serde::Serialize)]
 775→struct PerformanceStats {
 776→    cpu_usage: f32,
 777→    cpu_per_core: Vec<f32>,
 778→    cpu_freq: u64,
 779→    cpu_cores: usize,
 780→    cpu_name: String,
 781→    ram_total: u64,
 782→    ram_used: u64,
 783→    ram_usage: f32,
 784→    gpu_name: String,
 785→    gpu_usage: Option<f32>,
 786→    gpu_vram_total: Option<u64>,
 787→    gpu_vram_used: Option<u64>,
 788→}
 789→
 790→/// 查询性能监测数据
 791→#[tauri::command]
 792→fn get_performance_stats() -> PerformanceStats {
 793→    std::sync::LazyLock::force(&GPU_WORKER);
 794→
 795→    let mut sys = SYS.lock().unwrap();
 796→    sys.refresh_cpu_all();
 797→    sys.refresh_memory();
 798→
 799→    let cpus = sys.cpus();
 800→    let cpu_usage = if !cpus.is_empty() {
 801→        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
 802→    } else {
 803→        0.0
 804→    };
 805→    let cpu_per_core: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
 806→    let cpu_freq = cpus.first().map(|c| c.frequency()).unwrap_or(0);
 807→    let cpu_cores = cpus.len();
 808→    let cpu_name = cpus
 809→        .first()
 810→        .map(|c| c.brand().to_string())
 811→        .unwrap_or_default();
 812→
 813→    let ram_total = sys.total_memory();
 814→    let ram_used = sys.used_memory();
 815→    let ram_usage = if ram_total > 0 {
 816→        ram_used as f32 / ram_total as f32 * 100.0
 817→    } else {
 818→        0.0
 819→    };
 820→    drop(sys);
 821→
 822→    let gpu_name = GPU_NAME.get_or_init(query_gpu).clone();
 823→    let gpu = GPU_LIVE.lock().map(|c| c.clone()).unwrap_or_default();
 824→
 825→    PerformanceStats {
 826→        cpu_usage,
 827→        cpu_per_core,
 828→        cpu_freq,
 829→        cpu_cores,
 830→        cpu_name,
 831→        ram_total,
 832→        ram_used,
 833→        ram_usage,
 834→        gpu_name,
 835→        gpu_usage: gpu.usage,
 836→        gpu_vram_total: gpu.vram_total,
 837→        gpu_vram_used: gpu.vram_used,
 838→    }
 839→}
 840→
 841→/// 通过 PowerShell 性能计数器查询 GPU 使用率和显存
 842→fn query_gpu_stats() -> (Option<f32>, Option<u64>, Option<u64>) {
 843→    let script = r#"
 844→$e = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine -ErrorAction SilentlyContinue)
 845→$m = @(Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory -ErrorAction SilentlyContinue)
 846→$avg = 0.0
 847→if ($e.Count -gt 0) { $avg = ($e | Measure-Object UtilizationPercentage -Average).Average }
 848→$sum = 0
 849→if ($m.Count -gt 0) { $sum = ($m | Measure-Object DedicatedUsage -Sum).Sum }
 850→$vram = @(Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue | Select-Object -ExpandProperty AdapterRAM)
 851→$total = $null
 852→if ($vram.Count -gt 0) { $total = $vram[0] }
 853→Write-Output ("{0:F2}|{1}|{2}" -f $avg, $sum, $total)
 854→"#;
 855→    let output = run_powershell(script, std::time::Duration::from_secs(10));
 856→    let raw = parse_powershell_output(output);
 857→    let line = raw.lines().next().unwrap_or("").trim();
 858→    let mut parts = line.split('|');
 859→    let usage = parts.next().and_then(|s| s.parse::<f32>().ok());
 860→    let vram_used = parts.next().and_then(|s| s.parse::<u64>().ok());
 861→    let vram_total = parts.next().and_then(|s| s.parse::<u64>().ok());
 862→
 863→    (usage, vram_total, vram_used)
 864→}
 865→
 866→/// 进程信息
 867→#[derive(serde::Serialize)]
 868→struct ProcessInfo {
 869→    pid: u32,
 870→    name: String,
 871→    cpu_usage: f32,
 872→    memory: u64,
 873→    memory_percent: f32,
 874→    status: String,
 875→    icon: Option<String>,
 876→    is_self: bool,
 877→    is_related: bool,
 878→    is_system: bool,
 879→    parent_pid: Option<u32>,
 880→}
 881→
 882→/// exe 图标缓存：相同路径只提取一次
 883→static ICON_CACHE: std::sync::LazyLock<
 884→    std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, Option<String>>>,
 885→> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
 886→
 887→/// 从 exe 路径提取图标（16x16 PNG base64）
 888→fn extract_exe_icon(exe_path: &std::path::Path) -> Option<String> {
 889→    if let Ok(cache) = ICON_CACHE.lock() {
 890→        if let Some(v) = cache.get(exe_path) {
 891→            return v.clone();
 892→        }
 893→    }
 894→    use std::os::windows::ffi::OsStrExt;
 895→    use windows::core::PCWSTR;
 896→    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGFI_ICON, SHGFI_SMALLICON, SHFILEINFOW};
 897→    use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;
 898→
 899→    let path_wide: Vec<u16> = exe_path
 900→        .as_os_str()
 901→        .encode_wide()
 902→        .chain(std::iter::once(0))
 903→        .collect();
 904→
 905→    let mut shfi = SHFILEINFOW::default();
 906→    let flags = SHGFI_ICON | SHGFI_SMALLICON;
 907→    let result = unsafe {
 908→        SHGetFileInfoW(
 909→            PCWSTR(path_wide.as_ptr()),
 910→            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
 911→            Some(&mut shfi),
 912→            std::mem::size_of::<SHFILEINFOW>() as u32,
 913→            flags,
 914→        )
 915→    };
 916→
 917→    if result == 0 || shfi.hIcon.is_invalid() {
 918→        return None;
 919→    }
 920→
 921→    let hicon = shfi.hIcon;
 922→    let png = icon_to_png_base64(hicon);
 923→    unsafe { let _ = DestroyIcon(hicon); }
 924→    if let Ok(mut cache) = ICON_CACHE.lock() {
 925→        cache.insert(exe_path.to_path_buf(), png.clone());
 926→    }
 927→    png
 928→}
 929→
 930→/// 将 HICON 转换为 PNG base64
 931→fn icon_to_png_base64(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<String> {
 932→    use windows::Win32::Graphics::Gdi::{
 933→        CreateCompatibleDC, DeleteObject, DeleteDC, GetDIBits,
 934→        BITMAPINFO, BITMAPINFOHEADER, DIB_USAGE, RGBQUAD,
 935→    };
 936→    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
 937→
 938→    let mut icon_info = ICONINFO::default();
 939→    unsafe { GetIconInfo(hicon, &mut icon_info).ok()? };
 940→
 941→    let hdc = unsafe { CreateCompatibleDC(None) };
 942→    if hdc.is_invalid() {
 943→        unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
 944→        unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }
 945→        return None;
 946→    }
 947→
 948→    let mut bi = BITMAPINFOHEADER::default();
 949→    bi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
 950→    bi.biWidth = 16;
 951→    bi.biHeight = -16;
 952→    bi.biPlanes = 1;
 953→    bi.biBitCount = 32;
 954→    bi.biCompression = 0;
 955→
 956→    let mut bmi = BITMAPINFO {
 957→        bmiHeader: bi,
 958→        bmiColors: [RGBQUAD::default(); 1],
 959→    };
 960→
 961→    let mut pixels: Vec<u8> = vec![0u8; (16 * 16 * 4) as usize];
 962→    let rows = unsafe {
 963→        GetDIBits(
 964→            hdc,
 965→            icon_info.hbmColor,
 966→            0,
 967→            16,
 968→            Some(pixels.as_mut_ptr() as *mut _),
 969→            &mut bmi,
 970→            DIB_USAGE(0),
 971→        )
 972→    };
 973→
 974→    unsafe { let _ = DeleteDC(hdc); }
 975→    unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
 976→    unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }
 977→
 978→    if rows == 0 {
 979→        return None;
 980→    }
 981→
 982→    let png = rgba_to_png(16, 16, &pixels)?;
 983→    use base64::Engine;
 984→    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
 985→    Some(format!("data:image/png;base64,{}", b64))
 986→}
 987→
 988→/// 简易 RGBA → PNG 编码（无依赖实现）
 989→fn rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
 990→    let mut out: Vec<u8> = Vec::new();
 991→
 992→    let mut filtered: Vec<u8> = Vec::with_capacity((width * height * 4 + height) as usize);
 993→    let mut row_start = 0;
 994→    for _y in 0..height {
 995→        filtered.push(0u8);
 996→        for x in 0..width {
 997→            let i = row_start + (x * 4) as usize;
 998→            let b = rgba[i];
 999→            let g = rgba[i + 1];
1000→            let r = rgba[i + 2];
1001→            let a = rgba[i + 3];
1002→            filtered.push(r);
1003→            filtered.push(g);
1004→            filtered.push(b);
1005→            filtered.push(a);
1006→        }
1007→        row_start += (width * 4) as usize;
1008→    }
1009→
1010→    let raw = &filtered;
1011→    let zlib = flate_write(raw)?;
1012→    let crc_table = build_crc_table();
1013→
1014→    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
1015→    write_chunk(&mut out, b"IHDR", {
1016→        let mut ihdr = Vec::with_capacity(13);
1017→        ihdr.extend_from_slice(&width.to_be_bytes());
1018→        ihdr.extend_from_slice(&height.to_be_bytes());
1019→        ihdr.push(8);
1020→        ihdr.push(6);
1021→        ihdr.push(0);
1022→        ihdr.push(0);
1023→        ihdr.push(0);
1024→        ihdr
1025→    }, &crc_table);
1026→    write_chunk(&mut out, b"IDAT", zlib, &crc_table);
1027→    write_chunk(&mut out, b"IEND", Vec::new(), &crc_table);
1028→
1029→    Some(out)
1030→}
1031→
1032→fn build_crc_table() -> [u32; 256] {
1033→    let mut table = [0u32; 256];
1034→    for n in 0..256u32 {
1035→        let mut c = n;
1036→        for _ in 0..8 {
1037→            if c & 1 != 0 {
1038→                c = 0xedb88320 ^ (c >> 1);
1039→            } else {
1040→                c >>= 1;
1041→            }
1042→        }
1043→        table[n as usize] = c;
1044→    }
1045→    table
1046→}
1047→
1048→fn crc32(data: &[u8], table: &[u32; 256]) -> u32 {
1049→    let mut crc = 0xffffffff;
1050→    for &b in data {
1051→        crc = table[((crc ^ b as u32) & 0xff) as usize] ^ (crc >> 8);
1052→    }
1053→    crc ^ 0xffffffff
1054→}
1055→
1056→fn write_chunk(out: &mut Vec<u8>, type_: &[u8; 4], data: Vec<u8>, table: &[u32; 256]) {
1057→    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
1058→    let start = out.len();
1059→    out.extend_from_slice(type_);
1060→    out.extend_from_slice(&data);
1061→    let crc = crc32(&out[start..], table);
1062→    out.extend_from_slice(&crc.to_be_bytes());
1063→}
1064→
1065→fn flate_write(data: &[u8]) -> Option<Vec<u8>> {
1066→    // DEFLATE stored（无压缩）块：BFINAL=1, BTYPE=00 占首字节（含 5 bit 对齐），后跟 LEN/NLEN
1067→    if data.len() > u16::MAX as usize {
1068→        return None;
1069→    }
1070→    let mut zlib = Vec::with_capacity(data.len() + 16);
1071→    zlib.push(0x78);
1072→    zlib.push(0x01);
1073→    zlib.push(0x01);
1074→    let len = data.len() as u16;
1075→    zlib.extend_from_slice(&len.to_le_bytes());
1076→    zlib.extend_from_slice(&(!len).to_le_bytes());
1077→    zlib.extend_from_slice(data);
1078→    let adler = adler32(data);
1079→    zlib.extend_from_slice(&adler.to_be_bytes());
1080→    Some(zlib)
1081→}
1082→
1083→fn adler32(data: &[u8]) -> u32 {
1084→    let mut a: u32 = 1;
1085→    let mut b: u32 = 0;
1086→    for &byte in data {
1087→        a = (a + byte as u32) % 65521;
1088→        b = (b + a) % 65521;
1089→    }
1090→    (b << 16) | a
1091→}
1092→
1093→/// 调用 Windows SCM API 枚举所有正在运行的服务进程 PID
1094→fn query_service_pids() -> std::collections::HashSet<u32> {
1095→    use windows::core::PCWSTR;
1096→    use windows::Win32::System::Services::{
1097→        CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW,
1098→        SC_ENUM_PROCESS_INFO, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_STATE_ALL, SERVICE_WIN32,
1099→    };
1100→
1101→    let mut pids: std::collections::HashSet<u32> = std::collections::HashSet::new();
1102→
1103→    let scm = unsafe {
1104→        OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)
1105→    };
1106→    let Ok(scm) = scm else { return pids; };
1107→
1108→    let mut bytes_needed: u32 = 0;
1109→    let mut services_returned: u32 = 0;
1110→    let mut resume_handle: u32 = 0;
1111→
1112→    let _ = unsafe {
1113→        EnumServicesStatusExW(
1114→            scm,
1115→            SC_ENUM_PROCESS_INFO,
1116→            SERVICE_WIN32,
1117→            SERVICE_STATE_ALL,
1118→            None,
1119→            &mut bytes_needed,
1120→            &mut services_returned,
1121→            Some(&mut resume_handle),
1122→            PCWSTR::null(),
1123→        )
1124→    };
1125→
1126→    if bytes_needed == 0 {
1127→        unsafe { let _ = CloseServiceHandle(scm); };
1128→        return pids;
1129→    }
1130→
1131→    let mut buffer: Vec<u8> = vec![0u8; bytes_needed as usize];
1132→    let ok = unsafe {
1133→        EnumServicesStatusExW(
1134→            scm,
1135→            SC_ENUM_PROCESS_INFO,
1136→            SERVICE_WIN32,
1137→            SERVICE_STATE_ALL,
1138→            Some(buffer.as_mut_slice()),
1139→            &mut bytes_needed,
1140→            &mut services_returned,
1141→            Some(&mut resume_handle),
1142→            PCWSTR::null(),
1143→        )
1144→    };
1145→
1146→    if ok.is_ok() && services_returned > 0 {
1147→        let entries_ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;
1148→        for i in 0..services_returned as usize {
1149→            let entry = unsafe { &*entries_ptr.add(i) };
1150→            let pid = entry.ServiceStatusProcess.dwProcessId;
1151→            if pid != 0 {
1152→                pids.insert(pid);
1153→            }
1154→        }
1155→    }
1156→
1157→    unsafe { let _ = CloseServiceHandle(scm); };
1158→
1159→    pids
1160→}
1161→
1162→/// 查询进程列表（不含图标，图标由 process_icons 异步补充）
1163→#[tauri::command]
1164→fn list_processes() -> Vec<ProcessInfo> {
1165→    use sysinfo::ProcessesToUpdate;
1166→
1167→    let mut sys = SYS.lock().unwrap();
1168→    sys.refresh_processes(ProcessesToUpdate::All, true);
1169→
1170→    let total_mem = sys.total_memory();
1171→    let self_pid = std::process::id();
1172→
1173→    let mut related_pids: std::collections::HashSet<u32> = std::collections::HashSet::new();
1174→    related_pids.insert(self_pid);
1175→    let mut queue: std::collections::VecDeque<u32> = std::collections::VecDeque::new();
1176→    queue.push_back(self_pid);
1177→    while let Some(parent_pid) = queue.pop_front() {
1178→        for (pid, p) in sys.processes() {
1179→            let pid_u32 = pid.as_u32();
1180→            if related_pids.contains(&pid_u32) {
1181→                continue;
1182→            }
1183→            if p.parent().map(|pp| pp.as_u32() == parent_pid).unwrap_or(false) {
1184→                related_pids.insert(pid_u32);
1185→                queue.push_back(pid_u32);
1186→            }
1187→        }
1188→    }
1189→
1190→    let service_pids = query_service_pids();
1191→
1192→    let mut entries: Vec<ProcessInfo> = sys
1193→        .processes()
1194→        .iter()
1195→        .map(|(pid, p)| {
1196→            let mem = p.memory();
1197→            let mem_percent = if total_mem > 0 {
1198→                mem as f32 / total_mem as f32 * 100.0
1199→            } else {
1200→                0.0
1201→            };
1202→            let status = match p.status() {
1203→                sysinfo::ProcessStatus::Run => "running",
1204→                sysinfo::ProcessStatus::Sleep => "sleeping",
1205→                sysinfo::ProcessStatus::Stop => "stopped",
1206→                _ => "unknown",
1207→            }
1208→            .to_string();
1209→            let pid_u32 = pid.as_u32();
1210→            let is_self = pid_u32 == self_pid;
1211→            let is_related = !is_self && related_pids.contains(&pid_u32);
1212→            let is_system = !is_self && !is_related && service_pids.contains(&pid_u32);
1213→            ProcessInfo {
1214→                pid: pid_u32,
1215→                name: p.name().to_string_lossy().to_string(),
1216→                cpu_usage: p.cpu_usage(),
1217→                memory: mem,
1218→                memory_percent: mem_percent,
1219→                status,
1220→                icon: None,
1221→                is_self,
1222→                is_related,
1223→                is_system,
1224→                parent_pid: p.parent().map(|pp| pp.as_u32()),
1225→            }
1226→        })
1227→        .collect();
1228→    drop(sys);
1229→
1230→    entries.sort_by(|a, b| b.memory.cmp(&a.memory));
1231→
1232→    entries
1233→}
1234→
1235→/// 进程图标条目
1236→#[derive(serde::Serialize)]
1237→struct ProcessIcon {
1238→    pid: u32,
1239→    icon: Option<String>,
1240→}
1241→
1242→/// 批量提取进程图标（多线程并行，带路径级缓存）
1243→#[tauri::command]
1244→fn process_icons(pids: Vec<u32>) -> Vec<ProcessIcon> {
1245→    let paths: Vec<(u32, std::path::PathBuf)> = {
1246→        let sys = SYS.lock().unwrap();
1247→        pids.into_iter()
1248→            .filter_map(|pid| {
1249→                sys.process(sysinfo::Pid::from_u32(pid))
1250→                    .and_then(|p| p.exe().map(|e| (pid, e.to_path_buf())))
1251→            })
1252→            .collect()
1253→    };
1254→
1255→    let mut result: Vec<ProcessIcon> = Vec::with_capacity(paths.len());
1256→    let mut pending: Vec<(u32, std::path::PathBuf)> = Vec::new();
1257→
1258→    for (pid, path) in paths {
1259→        let cached = ICON_CACHE
1260→            .lock()
1261→            .ok()
1262→            .and_then(|c| c.get(&path).cloned());
1263→        match cached {
1264→            Some(v) => result.push(ProcessIcon { pid, icon: v }),
1265→            None => pending.push((pid, path)),
1266→        }
1267→    }
1268→
1269→    if !pending.is_empty() {
1270→        let worker_count = std::thread::available_parallelism()
1271→            .map(|n| n.get().min(4))
1272→            .unwrap_or(2);
1273→        let chunk_size = pending.len().div_ceil(worker_count);
1274→        let mut handles = Vec::new();
1275→        for chunk in pending.chunks(chunk_size) {
1276→            let chunk = chunk.to_vec();
1277→            handles.push(std::thread::spawn(move || {
1278→                chunk
1279→                    .into_iter()
1280→                    .map(|(pid, path)| ProcessIcon {
1281→                        pid,
1282→                        icon: extract_exe_icon(&path),
1283→                    })
1284→                    .collect::<Vec<_>>()
1285→            }));
1286→        }
1287→        for h in handles {
1288→            if let Ok(mut part) = h.join() {
1289→                result.append(&mut part);
1290→            }
1291→        }
1292→    }
1293→
1294→    result
1295→}
1296→
1297→/// 结束进程
1298→#[tauri::command]
1299→fn kill_process(_app: tauri::AppHandle, pid: u32) -> Result<String, String> {
1300→    let sys = SYS.lock().unwrap();
1301→    let my_pid = std::process::id();
1302→    if pid == my_pid {
1303→        return Err("禁止结束 ToolsPlus 自身进程".to_string());
1304→    }
1305→    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
1306→        if process.kill() {
1307→            Ok(format!("进程 {} 已结束", pid))
1308→        } else {
1309→            Err(format!("无法结束进程 {}", pid))
1310→        }
1311→    } else {
1312→        Err(format!("未找到进程 {}", pid))
1313→    }
1314→}
1315→
1316→/// 获取当前前台窗口对应的进程 PID
1317→#[tauri::command]
1318→fn get_foreground_window_pid() -> Result<u32, String> {
1319→    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
1320→    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
1321→    let hwnd = unsafe { GetForegroundWindow() };
1322→    if hwnd.0.is_null() {
1323→        return Err("无法获取前台窗口".to_string());
1324→    }
1325→    let mut pid: u32 = 0;
1326→    unsafe {
1327→        GetWindowThreadProcessId(hwnd, Some(&mut pid));
1328→    }
1329→    if pid == 0 {
1330→        return Err("无法获取窗口进程 PID".to_string());
1331→    }
1332→    Ok(pid)
1333→}
1334→
1335→/// 冻结进程（NtSuspendProcess）
1336→#[tauri::command]
1337→fn suspend_process(pid: u32) -> Result<String, String> {
1338→    use windows::Win32::Foundation::CloseHandle;
1339→    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
1340→    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
1341→
1342→    type NtSuspendProcess = unsafe extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32;
1343→
1344→    let h = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) }
1345→        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
1346→    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
1347→        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
1348→    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtSuspendProcess")) }
1349→        .ok_or_else(|| "找不到 NtSuspendProcess".to_string())?;
1350→    let func: NtSuspendProcess = unsafe { std::mem::transmute(addr) };
1351→    let status = unsafe { func(h) };
1352→    unsafe { let _ = CloseHandle(h); };
1353→    if status < 0 {
1354→        return Err(format!("NtSuspendProcess 返回 0x{:X}", status as u32));
1355→    }
1356→    Ok(format!("进程 {} 已冻结", pid))
1357→}
1358→
1359→/// 恢复进程（NtResumeProcess）
1360→#[tauri::command]
1361→fn resume_process(pid: u32) -> Result<String, String> {
1362→    use windows::Win32::Foundation::CloseHandle;
1363→    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
1364→    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};
1365→
1366→    type NtResumeProcess = unsafe extern "system" fn(windows::Win32::Foundation::HANDLE) -> i32;
1367→
1368→    let h = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) }
1369→        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
1370→    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
1371→        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
1372→    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtResumeProcess")) }
1373→        .ok_or_else(|| "找不到 NtResumeProcess".to_string())?;
1374→    let func: NtResumeProcess = unsafe { std::mem::transmute(addr) };
1375→    let status = unsafe { func(h) };
1376→    unsafe { let _ = CloseHandle(h); };
1377→    if status < 0 {
1378→        return Err(format!("NtResumeProcess 返回 0x{:X}", status as u32));
1379→    }
1380→    Ok(format!("进程 {} 已恢复", pid))
1381→}
1382→
1383→/// 查询进程的 PPL 保护级别
1384→#[tauri::command]
1385→fn get_ppl_protection(pid: u32) -> Result<String, String> {
1386→    use windows::Win32::Foundation::{CloseHandle, HANDLE};
1387→    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
1388→    use windows::Win32::System::Threading::{
1389→        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
1390→    };
1391→
1392→    type NtQueryInformationProcess = unsafe extern "system" fn(
1393→        HANDLE,
1394→        u32,
1395→        *mut std::ffi::c_void,
1396→        u32,
1397→        *mut u32,
1398→    ) -> i32;
1399→
1400→    const PROCESS_PROTECTION_INFORMATION: u32 = 61;
1401→    #[repr(C)]
1402→    #[derive(Default)]
1403→    struct ProcessProtection {
1404→        protection_level: u16,
1405→        _reserved1: u16,
1406→        _reserved2: u32,
1407→    }
1408→
1409→    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
1410→        .map_err(|e| format!("无法打开进程 {}：{}", pid, e))?;
1411→    let ntdll = unsafe { GetModuleHandleW(windows::core::w!("ntdll.dll")) }
1412→        .map_err(|e| format!("无法获取 ntdll 句柄：{}", e))?;
1413→    let addr = unsafe { GetProcAddress(ntdll, windows::core::s!("NtQueryInformationProcess")) }
1414→        .ok_or_else(|| "找不到 NtQueryInformationProcess".to_string())?;
1415→    let func: NtQueryInformationProcess = unsafe { std::mem::transmute(addr) };
1416→
1417→    let mut info = ProcessProtection::default();
1418→    let mut ret_len: u32 = 0;
1419→    let status = unsafe {
1420→        func(
1421→            h,
1422→            PROCESS_PROTECTION_INFORMATION,
1423→            &mut info as *mut _ as *mut _,
1424→            std::mem::size_of::<ProcessProtection>() as u32,
1425→            &mut ret_len,
1426→        )
1427→    };
1428→    unsafe { let _ = CloseHandle(h); };
1429→
1430→    if status < 0 {
1431→        return Err(format!("NtQueryInformationProcess 返回 0x{:X}", status as u32));
1432→    }
1433→
1434→    let level = info.protection_level & 0x7;
1435→    let type_ = (info.protection_level >> 4) & 0x7;
1436→    let sign = (info.protection_level >> 8) & 0x7;
1437→
1438→    let type_str = match type_ {
1439→        0 => "None",
1440→        1 => "ProtectedLight",
1441→        2 => "Protected",
1442→        _ => "Unknown",
1443→    };
1444→    let level_str = match (type_, level) {
1445→        (0, _) => "无保护".to_string(),
1446→        (1, 0) => "ProtectedLight (None)".to_string(),
1447→        (1, 1) => "ProtectedLight (Lsa)".to_string(),
1448→        (1, 2) => "ProtectedLight (Windows)".to_string(),
1449→        (1, 3) => "ProtectedLight (AntiMalware)".to_string(),
1450→        (1, 4) => "ProtectedLight (CodeIntegrity)".to_string(),
1451→        (1, 5) => "ProtectedLight (Authenticode)".to_string(),
1452→        (2, 0) => "Protected (None)".to_string(),
1453→        (2, 1) => "Protected (Lsa)".to_string(),
1454→        (2, 2) => "Protected (Windows)".to_string(),
1455→        (2, 3) => "Protected (AntiMalware)".to_string(),
1456→        (2, 4) => "Protected (CodeIntegrity)".to_string(),
1457→        (2, 5) => "Protected (Authenticode)".to_string(),
1458→        _ => format!("Unknown ({}, {})", type_, level),
1459→    };
1460→    let sign_str = match sign {
1461→        0 => "None",
1462→        1 => "Authenticode",
1463→        2 => "CodeIntegrity",
1464→        3 => "Platform",
1465→        _ => "Unknown",
1466→    };
1467→
1468→    Ok(format!("Type: {} | Level: {} | Sign: {}", type_str, level_str, sign_str))
1469→}
1470→
1471→/// 以管理员权限重启进程（需要进程 exe 路径），成功后自动结束原进程
1472→#[tauri::command]
1473→fn restart_as_admin(pid: u32) -> Result<String, String> {
1474→    use std::os::windows::ffi::OsStrExt;
1475→    use windows::core::PCWSTR;
1476→    use windows::Win32::UI::Shell::ShellExecuteW;
1477→
1478→    let exe_path = {
1479→        let sys = SYS.lock().unwrap();
1480→        sys.process(sysinfo::Pid::from_u32(pid))
1481→            .and_then(|p| p.exe().map(|e| e.to_path_buf()))
1482→    }
1483→    .ok_or_else(|| format!("未找到进程 {} 或无法获取 exe 路径", pid))?;
1484→
1485→    let wide: Vec<u16> = exe_path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
1486→    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
1487→
1488→    let result = unsafe {
1489→        ShellExecuteW(
1490→            None,
1491→            PCWSTR(verb.as_ptr()),
1492→            PCWSTR(wide.as_ptr()),
1493→            PCWSTR::null(),
1494→            PCWSTR::null(),
1495→            windows::Win32::UI::WindowsAndMessaging::SW_NORMAL,
1496→        )
1497→    };
1498→
1499→    if result.0 as usize <= 32 {
1500→        return Err(format!("ShellExecute 失败，返回码 {}", result.0 as usize));
1501→    }
1502→
1503→    let sys = SYS.lock().unwrap();
1504→    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
1505→        let _ = process.kill();
1506→    }
1507→    drop(sys);
1508→
1509→    Ok(format!("已以管理员权限重启进程 {} 并关闭原进程", pid))
1510→}
1511→
1512→// ==================== Git 可视化管理 ====================
1513→
1514→/// 在指定目录执行 git 命令，返回 (成功与否, stdout, stderr)
1515→fn run_git(repo: &str, args: &[&str]) -> (bool, String, String) {
1516→    use std::process::{Command, Stdio};
1517→    use std::os::windows::process::CommandExt;
1518→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
1519→    let candidates: Vec<std::ffi::OsString> = vec![
1520→        "git".into(),
1521→        r"C:\Program Files\Git\cmd\git.exe".into(),
1522→        r"C:\Program Files (x86)\Git\cmd\git.exe".into(),
1523→    ];
1524→    let mut last_err = String::new();
1525→    for exe in candidates {
1526→        match Command::new(&exe)
1527→            .arg("-c")
1528→            .arg("core.quotepath=false")
1529→            .args(args)
1530→            .current_dir(repo)
1531→            .stdout(Stdio::piped())
1532→            .stderr(Stdio::piped())
1533→            .creation_flags(CREATE_NO_WINDOW)
1534→            .output()
1535→        {
1536→            Ok(out) => {
1537→                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
1538→                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
1539→                return (out.status.success(), stdout, stderr);
1540→            }
1541→            Err(e) => last_err = e.to_string(),
1542→        }
1543→    }
1544→    (false, String::new(), last_err)
1545→}
1546→
1547→/// 检测系统是否安装了 git（PATH 优先，其次常见安装目录）
1548→#[command]
1549→fn check_git() -> bool {
1550→    use std::process::{Command, Stdio};
1551→    use std::os::windows::process::CommandExt;
1552→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
1553→    if Command::new("git")
1554→        .arg("--version")
1555→        .stdout(Stdio::piped())
1556→        .stderr(Stdio::piped())
1557→        .creation_flags(CREATE_NO_WINDOW)
1558→        .output()
1559→        .map(|o| o.status.success())
1560→        .unwrap_or(false)
1561→    {
1562→        return true;
1563→    }
1564→    let candidates = [
1565→        r"C:\Program Files\Git\cmd\git.exe",
1566→        r"C:\Program Files (x86)\Git\cmd\git.exe",
1567→    ];
1568→    candidates.iter().any(|p| Path::new(p).exists())
1569→}
1570→
1571→/// 通过 winget 一键安装 Git
1572→#[command]
1573→fn install_git() -> Result<String, String> {
1574→    use std::process::{Command, Stdio};
1575→    use std::os::windows::process::CommandExt;
1576→
1577→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
1578→    let out = Command::new("winget")
1579→        .args([
1580→            "install",
1581→            "--id", "Git.Git",
1582→            "-e",
1583→            "--source", "winget",
1584→            "--accept-package-agreements",
1585→            "--accept-source-agreements",
1586→            "--silent",
1587→            "--disable-interactivity",
1588→        ])
1589→        .stdout(Stdio::piped())
1590→        .stderr(Stdio::piped())
1591→        .creation_flags(CREATE_NO_WINDOW)
1592→        .output()
1593→        .map_err(|e| {
1594→            if e.kind() == std::io::ErrorKind::NotFound {
1595→                "winget 不可用，请手动安装 Git".to_string()
1596→            } else {
1597→                format!("启动 winget 失败: {e}")
1598→            }
1599→        })?;
1600→
1601→    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
1602→    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
1603→    if out.status.success() {
1604→        Ok(stdout)
1605→    } else {
1606→        let tail: String = if stderr.trim().len() > stdout.trim().len() {
1607→            stderr
1608→        } else {
1609→            stdout
1610→        }
1611→        .lines()
1612→        .rev()
1613→        .take(6)
1614→        .collect::<Vec<_>>()
1615→        .into_iter()
1616→        .rev()
1617→        .collect::<Vec<_>>()
1618→        .join("\n");
1619→        Err(if tail.trim().is_empty() {
1620→            "winget 安装失败".to_string()
1621→        } else {
1622→            tail
1623→        })
1624→    }
1625→}
1626→
1627→/// 获取应用默认工作目录（供前端探测仓库）
1628→#[command]
1629→fn git_default_dir() -> String {
1630→    std::env::current_dir()
1631→        .map(|p| p.to_string_lossy().to_string())
1632→        .unwrap_or_default()
1633→}
1634→
1635→/// 探测给定路径所属的 git 仓库根目录，不是仓库时返回 None
1636→#[command]
1637→fn git_repo_root(path: String) -> Option<String> {
1638→    if !Path::new(&path).exists() {
1639→        return None;
1640→    }
1641→    let (ok, out, _) = run_git(&path, &["rev-parse", "--show-toplevel"]);
1642→    if ok {
1643→        let root = out.trim().to_string();
1644→        if root.is_empty() {
1645→            None
1646→        } else {
1647→            Some(root)
1648→        }
1649→    } else {
1650→        None
1651→    }
1652→}
1653→
1654→/// Git 文件状态项
1655→#[derive(serde::Serialize)]
1656→struct GitFile {
1657→    path: String,
1658→    x: String,
1659→    y: String,
1660→    staged: bool,
1661→}
1662→
1663→/// Git 状态汇总
1664→#[derive(serde::Serialize)]
1665→struct GitStatus {
1666→    branch: String,
1667→    ahead: u32,
1668→    behind: u32,
1669→    files: Vec<GitFile>,
1670→    clean: bool,
1671→}
1672→
1673→/// 查询仓库状态（git status --porcelain=v1 -b）
1674→#[command]
1675→fn git_status(repo: String) -> Result<GitStatus, String> {
1676→    let (ok, out, err) = run_git(&repo, &["status", "--porcelain=v1", "-b"]);
1677→    if !ok {
1678→        return Err(err.trim().to_string());
1679→    }
1680→    let mut branch = String::new();
1681→    let mut ahead = 0u32;
1682→    let mut behind = 0u32;
1683→    let mut files = Vec::new();
1684→    for line in out.lines() {
1685→        if !line.is_empty() && line.starts_with("##") {
1686→            let head = line[2..].trim();
1687→            if let Some(rest) = head.strip_prefix("No commits yet on ") {
1688→                branch = rest.trim_end_matches('.').trim().to_string();
1689→                continue;
1690→            }
1691→            let parts: Vec<&str> = head.split("...").collect();
1692→            branch = parts[0].trim().to_string();
1693→            for seg in parts.iter().skip(1) {
1694→                for token in seg.split_whitespace() {
1695→                    if let Some(v) = token.strip_prefix("ahead ") {
1696→                        ahead = v.parse().unwrap_or(0);
1697→                    } else if let Some(v) = token.strip_prefix("behind ") {
1698→                        behind = v.parse().unwrap_or(0);
1699→                    }
1700→                }
1701→            }
1702→            continue;
1703→        }
1704→        if line.len() < 4 {
1705→            continue;
1706→        }
1707→        let chars: Vec<char> = line.chars().collect();
1708→        let x = chars[0].to_string();
1709→        let y = chars[1].to_string();
1710→        let staged = chars[0] != ' ' && chars[0] != '?';
1711→        let path = line[3..].trim().to_string();
1712→        files.push(GitFile {
1713→            path,
1714→            x,
1715→            y,
1716→            staged,
1717→        });
1718→    }
1719→    let clean = files.is_empty();
1720→    Ok(GitStatus {
1721→        branch,
1722→        ahead,
1723→        behind,
1724→        files,
1725→        clean,
1726→    })
1727→}
1728→
1729→/// Git 提交记录
1730→#[derive(serde::Serialize)]
1731→struct GitCommit {
1732→    hash: String,
1733→    short_hash: String,
1734→    author: String,
1735→    date: String,
1736→    message: String,
1737→    body: String,
1738→}
1739→
1740→/// 查询提交历史（最近 limit 条）
1741→#[command]
1742→fn git_log(repo: String, limit: u32) -> Result<Vec<GitCommit>, String> {
1743→    // %B 是完整 commit message（subject + body），用 %x00 分隔字段，用 %x1e 分隔记录
1744→    let fmt = "%H%x1f%h%x1f%an%x1f%ad%x1f%B%x1e";
1745→    let limit_str = format!("-{}", limit.max(1));
1746→    let (ok, out, err) = run_git(
1747→        &repo,
1748→        &["log", &limit_str, "--date=short", &format!("--pretty=format:{}", fmt)],
1749→    );
1750→    if !ok {
1751→        let lower = err.to_lowercase();
1752→        if lower.contains("does not have any commits yet")
1753→            || lower.contains("unknown revision")
1754→            || lower.contains("no commits yet")
1755→        {
1756→            return Ok(Vec::new());
1757→        }
1758→        return Err(err.trim().to_string());
1759→    }
1760→    let mut commits = Vec::new();
1761→    for record in out.split('\u{1e}') {
1762→        let record = record.trim_start_matches('\n').trim();
1763→        if record.is_empty() {
1764→            continue;
1765→        }
1766→        let parts: Vec<&str> = record.splitn(5, '\u{1f}').collect();
1767→        if parts.len() == 5 {
1768→            let full = parts[4];
1769→            // 完整 message 第一行是 subject，其余是 body
1770→            let mut lines = full.lines();
1771→            let subject = lines.next().unwrap_or("").trim().to_string();
1772→            let body: String = lines
1773→                .skip_while(|l| l.trim().is_empty()) // 跳过 subject 和 body 之间的空行
1774→                .collect::<Vec<&str>>()
1775→                .join("\n")
1776→                .trim()
1777→                .to_string();
1778→            commits.push(GitCommit {
1779→                hash: parts[0].to_string(),
1780→                short_hash: parts[1].to_string(),
1781→                author: parts[2].to_string(),
1782→                date: parts[3].to_string(),
1783→                message: subject,
1784→                body,
1785→            });
1786→        }
1787→    }
1788→    Ok(commits)
1789→}
1790→
1791→/// 暂存文件（git add）
1792→#[command]
1793→fn git_add(repo: String, paths: Vec<String>) -> Result<(), String> {
1794→    let mut args: Vec<&str> = vec!["add", "--"];
1795→    args.extend(paths.iter().map(|s| s.as_str()));
1796→    let (ok, _, err) = run_git(&repo, &args);
1797→    if ok {
1798→        Ok(())
1799→    } else {
1800→        Err(err.trim().to_string())
1801→    }
1802→}
1803→
1804→/// 取消暂存（git restore --staged）
1805→#[command]
1806→fn git_unstage(repo: String, paths: Vec<String>) -> Result<(), String> {
1807→    let mut args: Vec<&str> = vec!["restore", "--staged", "--"];
1808→    args.extend(paths.iter().map(|s| s.as_str()));
1809→    let (ok, _, err) = run_git(&repo, &args);
1810→    if ok {
1811→        Ok(())
1812→    } else {
1813→        Err(err.trim().to_string())
1814→    }
1815→}
1816→
1817→/// 提交（git commit -m）
1818→#[command]
1819→fn git_commit(repo: String, message: String, body: Option<String>, branch: Option<String>) -> Result<String, String> {
1820→    if message.trim().is_empty() {
1821→        return Err("提交信息不能为空".to_string());
1822→    }
1823→    let has_commits = {
1824→        let (ok, out, _) = run_git(&repo, &["rev-parse", "--verify", "HEAD"]);
1825→        ok && !out.trim().is_empty()
1826→    };
1827→    if let Some(b) = &branch {
1828→        if !b.is_empty() && has_commits {
1829→            let (cur_ok, cur_out, _) = run_git(&repo, &["branch", "--show-current"]);
1830→            let cur = cur_out.trim();
1831→            if cur_ok && !cur.is_empty() && cur != b {
1832→                let (co_ok, _, co_err) = run_git(&repo, &["checkout", b]);
1833→                if !co_ok {
1834→                    return Err(co_err.trim().to_string());
1835→                }
1836→            } else if cur.is_empty() || !cur_ok {
1837→                let (co_ok, _, co_err) = run_git(&repo, &["checkout", b]);
1838→                if !co_ok {
1839→                    return Err(co_err.trim().to_string());
1840→                }
1841→            }
1842→        }
1843→    }
1844→    // 组装提交信息：标题 + 空行 + 详细信息
1845→    let full_message = match body {
1846→        Some(b) if !b.trim().is_empty() => format!("{}\n\n{}", message.trim(), b.trim()),
1847→        _ => message.trim().to_string(),
1848→    };
1849→    let (ok, out, err) = run_git(&repo, &["commit", "-m", &full_message]);
1850→    if ok {
1851→        Ok(out.trim().to_string())
1852→    } else {
1853→        Err(err.trim().to_string())
1854→    }
1855→}
1856→
1857→/// 撤回指定提交（git revert）
1858→#[command]
1859→fn git_revert(repo: String, hash: String, no_commit: Option<bool>) -> Result<String, String> {
1860→    if hash.trim().is_empty() {
1861→        return Err("提交哈希不能为空".to_string());
1862→    }
1863→    let mut args = vec!["revert"];
1864→    if no_commit.unwrap_or(false) {
1865→        args.push("--no-commit");
1866→    }
1867→    args.push(hash.trim());
1868→    let (ok, out, err) = run_git(&repo, &args);
1869→    if ok {
1870→        Ok(out.trim().to_string())
1871→    } else {
1872→        Err(err.trim().to_string())
1873→    }
1874→}
1875→
1876→/// 切换工作区到指定提交（git checkout，进入 detached HEAD）
1877→#[command]
1878→fn git_checkout(repo: String, hash: String) -> Result<String, String> {
1879→    if hash.trim().is_empty() {
1880→        return Err("提交哈希不能为空".to_string());
1881→    }
1882→    let (ok, out, err) = run_git(&repo, &["checkout", hash.trim()]);
1883→    if ok {
1884→        Ok(out.trim().to_string())
1885→    } else {
1886→        Err(err.trim().to_string())
1887→    }
1888→}
1889→
1890→/// 列出本地分支
1891→#[command]
1892→fn git_branches(repo: String) -> Result<Vec<String>, String> {
1893→    let (ok, out, err) = run_git(&repo, &["branch", "--format=%(refname:short)"]);
1894→    if !ok {
1895→        let lower = err.to_lowercase();
1896→        if lower.contains("does not have any commits yet") || lower.contains("no commits yet") {
1897→            return Ok(Vec::new());
1898→        }
1899→        return Err(err.trim().to_string());
1900→    }
1901→    Ok(out
1902→        .lines()
1903→        .map(|l| l.trim().to_string())
1904→        .filter(|s| !s.is_empty())
1905→        .collect())
1906→}
1907→
1908→/// 推送当前分支到远程
1909→#[command]
1910→fn git_push(repo: String, branch: Option<String>) -> Result<String, String> {
1911→    let target = branch.filter(|b| !b.is_empty());
1912→    let (ok, out, err) = match &target {
1913→        Some(b) => run_git(&repo, &["push", "-u", "origin", b]),
1914→        None => run_git(&repo, &["push"]),
1915→    };
1916→    if ok {
1917→        Ok(out.trim().to_string())
1918→    } else {
1919→        let msg = err.trim();
1920→        if msg.is_empty() {
1921→            Err("推送失败".to_string())
1922→        } else {
1923→            Err(msg.to_string())
1924→        }
1925→    }
1926→}
1927→
1928→/// 拉取远程提交（fetch，不合并）
1929→#[command]
1930→fn git_fetch(repo: String) -> Result<String, String> {
1931→    let (ok, out, err) = run_git(&repo, &["fetch", "--all", "--prune"]);
1932→    if ok {
1933→        Ok(out.trim().to_string())
1934→    } else {
1935→        let msg = if err.trim().is_empty() { out.trim() } else { err.trim() };
1936→        Err(if msg.is_empty() { "拉取失败".to_string() } else { msg.to_string() })
1937→    }
1938→}
1939→
1940→/// 拉取并合并远程提交（pull）
1941→#[command]
1942→fn git_pull(repo: String) -> Result<String, String> {
1943→    let (ok, out, err) = run_git(&repo, &["pull", "--ff-only"]);
1944→    if ok {
1945→        Ok(out.trim().to_string())
1946→    } else {
1947→        let msg = if err.trim().is_empty() { out.trim() } else { err.trim() };
1948→        Err(if msg.is_empty() { "拉取失败".to_string() } else { msg.to_string() })
1949→    }
1950→}
1951→
1952→/// 克隆远程仓库到指定路径
1953→#[command]
1954→async fn git_clone(url: String, target_dir: String) -> Result<String, String> {
1955→    use std::process::Command;
1956→    use std::os::windows::process::CommandExt;
1957→    let task = tauri::async_runtime::spawn_blocking(move || {
1958→        let repo_name = url
1959→            .trim_end_matches('/')
1960→            .rsplit('/').next()
1961→            .map(|n| n.trim_end_matches(".git"))
1962→            .unwrap_or("repo");
1963→        let dest = Path::new(&target_dir).join(repo_name);
1964→        let dest_str = dest.to_string_lossy().to_string();
1965→        let out = Command::new("git")
1966→            .args(["clone", &url, &dest_str])
1967→            .creation_flags(0x0800_0000)
1968→            .output()
1969→            .map_err(|e| e.to_string())?;
1970→        if out.status.success() {
1971→            Ok(dest_str)
1972→        } else {
1973→            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
1974→        }
1975→    })
1976→    .await
1977→    .map_err(|e| e.to_string())?;
1978→    task
1979→}
1980→
1981→/// 在指定路径初始化一个 Git 仓库（git init）
1982→#[command]
1983→async fn git_init(path: String) -> Result<String, String> {
1984→    use std::path::Path;
1985→    use std::os::windows::process::CommandExt;
1986→    if !Path::new(&path).exists() {
1987→        return Err("目标路径不存在".to_string());
1988→    }
1989→    let task = tauri::async_runtime::spawn_blocking(move || {
1990→        use std::process::Command;
1991→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
1992→        let out = Command::new("git")
1993→            .args(["init"])
1994→            .current_dir(&path)
1995→            .creation_flags(CREATE_NO_WINDOW)
1996→            .output()
1997→            .map_err(|e| format!("git init 执行失败: {e}"))?;
1998→        if out.status.success() {
1999→            Ok(path.clone())
2000→        } else {
2001→            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
2002→        }
2003→    })
2004→    .await
2005→    .map_err(|e| e.to_string())?;
2006→    task
2007→}
2008→
2009→/// 弹出系统文件夹选择对话框，返回选中路径
2010→#[command]
2011→async fn pick_folder() -> Result<Option<String>, String> {
2012→    let task = tauri::async_runtime::spawn_blocking(move || {
2013→        rfd::FileDialog::new()
2014→            .set_title("选择文件夹")
2015→            .pick_folder()
2016→            .map(|p| p.to_string_lossy().to_string())
2017→    })
2018→    .await
2019→    .map_err(|e| e.to_string())?;
2020→    Ok(task)
2021→}
2022→
2023→/// 弹出系统图片选择对话框，返回选中文件路径
2024→#[command]
2025→async fn pick_image() -> Result<Option<String>, String> {
2026→    let task = tauri::async_runtime::spawn_blocking(move || {
2027→        rfd::FileDialog::new()
2028→            .set_title("选择图片")
2029→            .add_filter(
2030→                "图片",
2031→                &["png", "jpg", "jpeg", "bmp", "webp", "gif", "tif", "tiff"],
2032→            )
2033→            .pick_file()
2034→            .map(|p| p.to_string_lossy().to_string())
2035→    })
2036→    .await
2037→    .map_err(|e| e.to_string())?;
2038→    Ok(task)
2039→}
2040→
2041→/// 读取图片文件并返回 data URL（data:image/<mime>;base64,...）
2042→fn guess_mime(ext: &str) -> &'static str {
2043→    match ext.to_lowercase().as_str() {
2044→        "png" => "image/png",
2045→        "jpg" | "jpeg" => "image/jpeg",
2046→        "bmp" => "image/bmp",
2047→        "webp" => "image/webp",
2048→        "gif" => "image/gif",
2049→        "tif" | "tiff" => "image/tiff",
2050→        _ => "image/png",
2051→    }
2052→}
2053→
2054→#[command]
2055→async fn read_image_as_data_url(path: String) -> Result<String, String> {
2056→    let task = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
2057→        use base64::Engine;
2058→        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
2059→        let ext = std::path::Path::new(&path)
2060→            .extension()
2061→            .and_then(|e| e.to_str())
2062→            .unwrap_or("png");
2063→        let mime = guess_mime(ext);
2064→        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
2065→        Ok(format!("data:{};base64,{}", mime, b64))
2066→    })
2067→    .await
2068→    .map_err(|e| e.to_string())?;
2069→    task
2070→}
2071→
2072→/// gh 登录状态
2073→#[derive(serde::Serialize)]
2074→struct GhAuthState {
2075→    gh_installed: bool,
2076→    logged_in: bool,
2077→    user: String,
2078→    host: String,
2079→}
2080→
2081→fn gh_available() -> bool {
2082→    use std::os::windows::process::CommandExt;
2083→    use std::process::{Command, Stdio};
2084→    use std::sync::mpsc;
2085→    use std::time::Duration;
2086→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2087→    let (tx, rx) = mpsc::channel();
2088→    std::thread::spawn(move || {
2089→        let out = Command::new("gh")
2090→            .arg("--version")
2091→            .stdout(Stdio::piped())
2092→            .stderr(Stdio::piped())
2093→            .creation_flags(CREATE_NO_WINDOW)
2094→            .output();
2095→        let _ = tx.send(out);
2096→    });
2097→    match rx.recv_timeout(Duration::from_secs(2)) {
2098→        Ok(Ok(o)) => o.status.success(),
2099→        _ => false,
2100→    }
2101→}
2102→
2103→/// 查询 gh 登录状态（async，避免阻塞 UI）
2104→#[command]
2105→async fn gh_auth_state() -> GhAuthState {
2106→    let installed = tauri::async_runtime::spawn_blocking(|| gh_available())
2107→        .await
2108→        .unwrap_or(false);
2109→    if !installed {
2110→        return GhAuthState {
2111→            gh_installed: false,
2112→            logged_in: false,
2113→            user: String::new(),
2114→            host: String::new(),
2115→        };
2116→    }
2117→    let (has_token, user) = tauri::async_runtime::spawn_blocking(|| {
2118→        use std::os::windows::process::CommandExt;
2119→        use std::process::{Command, Stdio};
2120→        use std::sync::mpsc;
2121→        use std::time::Duration;
2122→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2123→        let (tx, rx) = mpsc::channel();
2124→        std::thread::spawn(move || {
2125→            let token_out = Command::new("gh")
2126→                .args(["auth", "token"])
2127→                .stdout(Stdio::piped())
2128→                .stderr(Stdio::piped())
2129→                .creation_flags(CREATE_NO_WINDOW)
2130→                .output();
2131→            let user_out = Command::new("gh")
2132→                .args(["api", "user", "--jq", ".login"])
2133→                .stdout(Stdio::piped())
2134→                .stderr(Stdio::piped())
2135→                .creation_flags(CREATE_NO_WINDOW)
2136→                .output();
2137→            let _ = tx.send((token_out, user_out));
2138→        });
2139→        match rx.recv_timeout(Duration::from_secs(8)) {
2140→            Ok((Ok(tok), Ok(usr))) => {
2141→                let has_tok = tok.status.success()
2142→                    && !String::from_utf8_lossy(&tok.stdout).trim().is_empty();
2143→                let username = if usr.status.success() {
2144→                    String::from_utf8_lossy(&usr.stdout).trim().to_string()
2145→                } else {
2146→                    String::new()
2147→                };
2148→                (has_tok, username)
2149→            }
2150→            _ => (false, String::new()),
2151→        }
2152→    })
2153→    .await
2154→    .unwrap_or((false, String::new()));
2155→
2156→    GhAuthState {
2157→        gh_installed: true,
2158→        logged_in: has_token,
2159→        user,
2160→        host: if has_token { "github.com".to_string() } else { String::new() },
2161→    }
2162→}
2163→
2164→/// 启动 gh auth login 网页登录流程（async）
2165→#[command]
2166→async fn gh_login_web() -> Result<String, String> {
2167→    let combined = tauri::async_runtime::spawn_blocking(|| {
2168→        use std::os::windows::process::CommandExt;
2169→        use std::process::{Command, Stdio};
2170→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2171→        match Command::new("gh")
2172→            .args(["auth", "login", "--web", "--git-protocol", "https"])
2173→            .stdin(Stdio::null())
2174→            .stdout(Stdio::piped())
2175→            .stderr(Stdio::piped())
2176→            .creation_flags(CREATE_NO_WINDOW)
2177→            .output()
2178→        {
2179→            Ok(out) => (
2180→                out.status.success(),
2181→                format!(
2182→                    "{}\n{}",
2183→                    String::from_utf8_lossy(&out.stdout),
2184→                    String::from_utf8_lossy(&out.stderr)
2185→                ),
2186→            ),
2187→            Err(e) => (false, format!("启动 gh 失败: {e}")),
2188→        }
2189→    })
2190→    .await
2191→    .map_err(|e| e.to_string())?;
2192→    if combined.0 {
2193→        Ok(combined.1)
2194→    } else {
2195→        Err(combined.1.trim().to_string())
2196→    }
2197→}
2198→
2199→/// 为当前仓库配置 gh 作为 git 凭据助手（async）
2200→#[command]
2201→async fn gh_setup_git(repo: String) -> Result<(), String> {
2202→    let installed = tauri::async_runtime::spawn_blocking(gh_available)
2203→        .await
2204→        .unwrap_or(false);
2205→    if !installed {
2206→        return Err("gh 未安装".to_string());
2207→    }
2208→    let result = tauri::async_runtime::spawn_blocking(move || {
2209→        use std::os::windows::process::CommandExt;
2210→        use std::process::{Command, Stdio};
2211→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2212→        match Command::new("gh")
2213→            .arg("auth")
2214→            .arg("setup-git")
2215→            .current_dir(&repo)
2216→            .stdout(Stdio::piped())
2217→            .stderr(Stdio::piped())
2218→            .creation_flags(CREATE_NO_WINDOW)
2219→            .output()
2220→        {
2221→            Ok(out) if out.status.success() => Ok(()),
2222→            Ok(out) => Err(String::from_utf8_lossy(&out.stderr).trim().to_string()),
2223→            Err(e) => Err(format!("setup-git 失败: {e}")),
2224→        }
2225→    })
2226→    .await
2227→    .map_err(|e| e.to_string())?;
2228→    result
2229→}
2230→
2231→/// 退出 GitHub 登录（强制登出所有账号，无交互）
2232→#[command]
2233→async fn gh_logout() -> Result<String, String> {
2234→    let installed = tauri::async_runtime::spawn_blocking(gh_available)
2235→        .await
2236→        .unwrap_or(false);
2237→    if !installed {
2238→        return Err("gh 未安装".to_string());
2239→    }
2240→    let result = tauri::async_runtime::spawn_blocking(|| {
2241→        use std::os::windows::process::CommandExt;
2242→        use std::process::{Command, Stdio};
2243→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2244→        // 先用 gh auth status --json 获取所有已登录账号（gh 2.40+ 支持多账号）
2245→        // 没有账号或命令失败时直接返回成功（视为已登出）
2246→        let status_out = Command::new("gh")
2247→            .args(["auth", "status", "--json", "host,oauthToken,accounts"])
2248→            .stdout(Stdio::piped())
2249→            .stderr(Stdio::piped())
2250→            .creation_flags(CREATE_NO_WINDOW)
2251→            .output()
2252→            .map_err(|e| format!("执行 gh auth status 失败: {e}"))?;
2253→
2254→        let mut logged_hosts: std::collections::HashSet<String> = std::collections::HashSet::new();
2255→        if status_out.status.success() {
2256→            let status_json = String::from_utf8_lossy(&status_out.stdout).trim().to_string();
2257→            // 简易解析：从 JSON 中找 "host":"..." 字段
2258→            // 避免引入 serde_json 依赖，直接字符串查找
2259→            let mut rest = status_json.as_str();
2260→            while let Some(pos) = rest.find("\"host\"") {
2261→                rest = &rest[pos + 6..];
2262→                if let Some(colon) = rest.find(':') {
2263→                    rest = &rest[colon + 1..];
2264→                    if let Some(start) = rest.find('"') {
2265→                        rest = &rest[start + 1..];
2266→                        if let Some(end) = rest.find('"') {
2267→                            logged_hosts.insert(rest[..end].to_string());
2268→                            rest = &rest[end + 1..];
2269→                        }
2270→                    }
2271→                }
2272→            }
2273→        }
2274→
2275→        // fallback：如果没解析出 host，默认尝试登出 github.com
2276→        if logged_hosts.is_empty() {
2277→            logged_hosts.insert("github.com".to_string());
2278→        }
2279→
2280→        let mut last_msg = String::new();
2281→        let mut any_err = false;
2282→        for host in &logged_hosts {
2283→            let out = Command::new("gh")
2284→                .args(["auth", "logout", "--hostname", host])
2285→                .stdout(Stdio::piped())
2286→                .stderr(Stdio::piped())
2287→                .creation_flags(CREATE_NO_WINDOW)
2288→                .output()
2289→                .map_err(|e| format!("执行 gh auth logout 失败 ({host}): {e}"))?;
2290→            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
2291→            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
2292→            let combined = if stdout.is_empty() { stderr } else { stdout };
2293→            if !combined.is_empty() {
2294→                last_msg = combined.clone();
2295→            }
2296→            // gh 2.40+ 在多账号场景下 logout 会进入交互选择，需要用 --user 指定
2297→            // 这里用循环直到该 host 所有账号都登出（最多重试 5 次防止死循环）
2298→            if out.status.success() {
2299→                // 该 host 可能还有其他账号，再查一次
2300→                for _ in 0..5 {
2301→                    let still = Command::new("gh")
2302→                        .args(["auth", "status"])
2303→                        .stdout(Stdio::piped())
2304→                        .stderr(Stdio::piped())
2305→                        .creation_flags(CREATE_NO_WINDOW)
2306→                        .output();
2307→                    match still {
2308→                        Ok(o) => {
2309→                            let stdout_str = String::from_utf8_lossy(&o.stdout).to_string();
2310→                            let stderr_str = String::from_utf8_lossy(&o.stderr).to_string();
2311→                            let combined = format!("{stdout_str}{stderr_str}");
2312→                            if !combined.to_lowercase().contains(host.as_str()) {
2313→                                break;
2314→                            }
2315→                            // 还有账号，继续 logout
2316→                            let _ = Command::new("gh")
2317→                                .args(["auth", "logout", "--hostname", host])
2318→                                .stdout(Stdio::piped())
2319→                                .stderr(Stdio::piped())
2320→                                .creation_flags(CREATE_NO_WINDOW)
2321→                                .output();
2322→                        }
2323→                        Err(_) => break,
2324→                    }
2325→                }
2326→            } else if combined.to_lowercase().contains("not logged") {
2327→                // 视为已登出
2328→            } else {
2329→                any_err = true;
2330→            }
2331→        }
2332→        if any_err {
2333→            Err(last_msg)
2334→        } else {
2335→            Ok(if last_msg.is_empty() {
2336→                "已登出所有账号".to_string()
2337→            } else {
2338→                last_msg
2339→            })
2340→        }
2341→    })
2342→    .await
2343→    .map_err(|e| e.to_string())?;
2344→    result
2345→}
2346→
2347→/// 全局保存 gh auth login PowerShell 子进程句柄，用于取消登录时终止
2348→static GH_LOGIN_CHILD: std::sync::Mutex<Option<std::process::Child>> = std::sync::Mutex::new(None);
2349→
2350→/// 启动新 PowerShell 窗口运行 gh auth login（交互式登录必须在真实终端中完成）
2351→/// 返回子进程 PID，前端可用于追踪
2352→#[command]
2353→async fn gh_login_interactive() -> Result<u32, String> {
2354→    tauri::async_runtime::spawn_blocking(|| {
2355→        use std::os::windows::process::CommandExt;
2356→        use std::process::Command;
2357→        // 启动新 PowerShell 窗口运行 gh auth login
2358→        // 优化启动速度：直接用 mode con: 设置 cols/lines，标题用于后续查找
2359→        // 不再用 Add-Type + GetConsoleWindow（编译 Add-Type 慢）
2360→        // 登录完成（成功/失败）后延时自动关闭，无需用户按回车
2361→        // 先循环 gh auth logout 清除所有旧 token，避免旧登录状态干扰 gh_wait_login 轮询
2362→        let script = "$Host.UI.RawUI.WindowTitle = 'GitHub CLI 登录'; mode con: cols=100 lines=30; Write-Host '===== GitHub CLI 登录 =====' -ForegroundColor Cyan; Write-Host '正在清除所有旧的登录状态...' -ForegroundColor Yellow; for ($i=0; $i -lt 10; $i++) { $null = gh auth logout --hostname github.com 2>&1; if ($LASTEXITCODE -ne 0) { break } }; Write-Host '（旧登录已清除或无登录）' -ForegroundColor DarkGray; Write-Host ''; Write-Host '请在下方交互式完成 GitHub 授权流程' -ForegroundColor Yellow; Write-Host ''; gh auth login; if ($LASTEXITCODE -eq 0) { Write-Host ''; Write-Host '登录成功，正在切换账号...' -ForegroundColor Green; for ($i=0; $i -lt 5; $i++) { $null = gh auth switch --hostname github.com 2>&1; if ($LASTEXITCODE -eq 0) { break } }; Write-Host '登录成功，3 秒后窗口将自动关闭...' -ForegroundColor Green; Start-Sleep -Seconds 3 } else { Write-Host ''; Write-Host '登录失败或已取消，10 秒后窗口将自动关闭...' -ForegroundColor Red; Start-Sleep -Seconds 10 }";
2363→        let child = Command::new("powershell")
2364→            .args(["-NoProfile", "-Command", script])
2365→            .creation_flags(0)
2366→            .spawn()
2367→            .map_err(|e| format!("启动登录窗口失败: {e}"))?;
2368→        let pid = child.id();
2369→        // 保存到全局，便于 cancel 时终止
2370→        if let Ok(mut guard) = GH_LOGIN_CHILD.lock() {
2371→            *guard = Some(child);
2372→        }
2373→        Ok::<u32, String>(pid)
2374→    })
2375→    .await
2376→    .map_err(|e| e.to_string())?
2377→}
2378→
2379→/// 取消正在进行的 gh auth login：杀掉 PowerShell 子进程及其子进程树
2380→#[command]
2381→async fn gh_cancel_login() -> Result<(), String> {
2382→    tauri::async_runtime::spawn_blocking(|| {
2383→        use std::os::windows::process::CommandExt;
2384→        use std::process::Command;
2385→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2386→        let mut killed_any = false;
2387→        if let Ok(mut guard) = GH_LOGIN_CHILD.lock() {
2388→            if let Some(mut child) = guard.take() {
2389→                let pid = child.id();
2390→                // 用 taskkill /T /F 杀整棵进程树（PowerShell + gh 子进程）
2391→                let _ = Command::new("taskkill")
2392→                    .args(["/PID", &pid.to_string(), "/T", "/F"])
2393→                    .creation_flags(CREATE_NO_WINDOW)
2394→                    .output();
2395→                // 也尝试 kill Child
2396→                let _ = child.kill();
2397→                killed_any = true;
2398→            }
2399→        }
2400→        // 兜底：通过窗口标题查找并关闭（万一 PID 没保存成功）
2401→        let _ = Command::new("powershell")
2402→            .args([
2403→                "-NoProfile",
2404→                "-Command",
2405→                "Get-Process powershell -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowTitle -like '*GitHub CLI*' } | Stop-Process -Force -ErrorAction SilentlyContinue",
2406→            ])
2407→            .creation_flags(CREATE_NO_WINDOW)
2408→            .output();
2409→        if killed_any {
2410→            Ok(())
2411→        } else {
2412→            Ok(()) // 即使没杀到也不报错，前端只要关闭对话框即可
2413→        }
2414→    })
2415→    .await
2416→    .map_err(|e| e.to_string())?
2417→}
2418→
2419→/// 轮询 gh api user 获取登录用户名，等待用户完成登录
2420→#[command]
2421→async fn gh_wait_login(timeout_secs: u64) -> Result<String, String> {
2422→    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
2423→    loop {
2424→        if std::time::Instant::now() > deadline {
2425→            return Err("等待登录超时".to_string());
2426→        }
2427→        let user = tauri::async_runtime::spawn_blocking(|| {
2428→            use std::os::windows::process::CommandExt;
2429→            use std::process::{Command, Stdio};
2430→            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2431→            // 用 gh api user --jq .login 直接获取登录用户名
2432→            // 未登录时会失败（非零退出码），已登录时返回纯用户名
2433→            match Command::new("gh")
2434→                .args(["api", "user", "--jq", ".login"])
2435→                .stdout(Stdio::piped())
2436→                .stderr(Stdio::piped())
2437→                .creation_flags(CREATE_NO_WINDOW)
2438→                .output()
2439→            {
2440→                Ok(o) if o.status.success() => {
2441→                    String::from_utf8_lossy(&o.stdout).trim().to_string()
2442→                }
2443→                _ => String::new(),
2444→            }
2445→        })
2446→        .await
2447→        .map_err(|e| e.to_string())?;
2448→
2449→        if !user.is_empty() {
2450→            return Ok(user);
2451→        }
2452→        std::thread::sleep(std::time::Duration::from_secs(2));
2453→    }
2454→}
2455→
2456→/// 设置 git 用户名和邮箱（全局配置）
2457→#[command]
2458→async fn git_config_user(name: String, email: String) -> Result<(), String> {
2459→    if name.trim().is_empty() || email.trim().is_empty() {
2460→        return Err("用户名和邮箱不能为空".to_string());
2461→    }
2462→    tauri::async_runtime::spawn_blocking(move || {
2463→        use std::os::windows::process::CommandExt;
2464→        use std::process::{Command, Stdio};
2465→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2466→
2467→        let run = |key: &str, val: &str| -> Result<(), String> {
2468→            let out = Command::new("git")
2469→                .args(["config", "--global", key, val])
2470→                .stdout(Stdio::piped())
2471→                .stderr(Stdio::piped())
2472→                .creation_flags(CREATE_NO_WINDOW)
2473→                .output()
2474→                .map_err(|e| format!("执行 git config 失败: {e}"))?;
2475→            if !out.status.success() {
2476→                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
2477→                return Err(if err.is_empty() {
2478→                    format!("git config {} 失败", key)
2479→                } else {
2480→                    err
2481→                });
2482→            }
2483→            Ok(())
2484→        };
2485→
2486→        run("user.name", &name)?;
2487→        run("user.email", &email)?;
2488→        Ok::<(), String>(())
2489→    })
2490→    .await
2491→    .map_err(|e| e.to_string())?
2492→}
2493→
2494→/// 读取当前 git 全局配置的 user.name 和 user.email
2495→#[command]
2496→async fn git_get_user_config() -> Result<(String, String), String> {
2497→    tauri::async_runtime::spawn_blocking(|| {
2498→        use std::os::windows::process::CommandExt;
2499→        use std::process::{Command, Stdio};
2500→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2501→
2502→        let read = |key: &str| -> String {
2503→            let out = Command::new("git")
2504→                .args(["config", "--global", "--get", key])
2505→                .stdout(Stdio::piped())
2506→                .stderr(Stdio::piped())
2507→                .creation_flags(CREATE_NO_WINDOW)
2508→                .output();
2509→            match out {
2510→                Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
2511→                _ => String::new(),
2512→            }
2513→        };
2514→
2515→        Ok((read("user.name"), read("user.email")))
2516→    })
2517→    .await
2518→    .map_err(|e| e.to_string())?
2519→}
2520→
2521→/// 一键同时安装 git 和 gh（winget 串行执行）
2522→#[command]
2523→async fn install_git_and_gh() -> Result<String, String> {
2524→    let result = tauri::async_runtime::spawn_blocking(|| {
2525→        use std::os::windows::process::CommandExt;
2526→        use std::process::{Command, Stdio};
2527→        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2528→
2529→        let install_one = |id: &str| -> Result<String, String> {
2530→            let out = Command::new("winget")
2531→                .args([
2532→                    "install",
2533→                    "--id",
2534→                    id,
2535→                    "-e",
2536→                    "--source",
2537→                    "winget",
2538→                    "--accept-package-agreements",
2539→                    "--accept-source-agreements",
2540→                    "--silent",
2541→                    "--disable-interactivity",
2542→                ])
2543→                .stdout(Stdio::piped())
2544→                .stderr(Stdio::piped())
2545→                .creation_flags(CREATE_NO_WINDOW)
2546→                .output()
2547→                .map_err(|e| {
2548→                    if e.kind() == std::io::ErrorKind::NotFound {
2549→                        "winget 不可用，请手动安装".to_string()
2550→                    } else {
2551→                        format!("启动 winget 失败: {e}")
2552→                    }
2553→                })?;
2554→            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
2555→            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
2556→            if out.status.success() {
2557→                Ok(stdout)
2558→            } else {
2559→                let tail: String = if stderr.trim().len() > stdout.trim().len() {
2560→                    stderr
2561→                } else {
2562→                    stdout
2563→                }
2564→                .lines()
2565→                .rev()
2566→                .take(6)
2567→                .collect::<Vec<_>>()
2568→                .into_iter()
2569→                .rev()
2570→                .collect::<Vec<_>>()
2571→                .join("\n");
2572→                Err(if tail.trim().is_empty() {
2573→                    format!("安装 {} 失败", id)
2574→                } else {
2575→                    tail
2576→                })
2577→            }
2578→        };
2579→
2580→        // 先安装 git，再安装 gh（gh 依赖 git）
2581→        let git_out = install_one("Git.Git")?;
2582→        let gh_out = install_one("GitHub.cli")?;
2583→        Ok(format!("{}\n{}", git_out, gh_out))
2584→    })
2585→    .await
2586→    .map_err(|e| e.to_string())?;
2587→    result
2588→}
2589→
2590→
2591→// ========== 系统优化 ==========
2592→
2593→/// 检测当前进程是否以管理员权限运行
2594→#[command]
2595→fn is_admin() -> bool {
2596→    use windows::Win32::Foundation::{CloseHandle, HANDLE};
2597→    use windows::Win32::Security::{
2598→        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
2599→    };
2600→    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
2601→    unsafe {
2602→        let mut token: HANDLE = HANDLE::default();
2603→        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
2604→            return false;
2605→        }
2606→        let mut elevation = TOKEN_ELEVATION::default();
2607→        let mut ret = 0u32;
2608→        let ok = GetTokenInformation(
2609→            token,
2610→            TokenElevation,
2611→            Some(&mut elevation as *mut _ as *mut _),
2612→            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
2613→            &mut ret,
2614→        ).is_ok();
2615→        let _ = CloseHandle(token);
2616→        ok && elevation.TokenIsElevated != 0
2617→    }
2618→}
2619→
2620→/// 以管理员权限重新启动当前应用（通过 ShellExecute runas）
2621→#[command]
2622→fn relaunch_as_admin() -> Result<(), String> {
2623→    use std::os::windows::process::CommandExt;
2624→    use std::process::Command;
2625→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2626→    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
2627→    let exe_str = exe.to_string_lossy().to_string();
2628→    Command::new("powershell")
2629→        .args([
2630→            "-NoProfile",
2631→            "-Command",
2632→            &format!(
2633→                "Start-Process -FilePath '{}' -Verb RunAs",
2634→                exe_str.replace('\'', "''")
2635→            ),
2636→        ])
2637→        .creation_flags(CREATE_NO_WINDOW)
2638→        .spawn()
2639→        .map_err(|e| e.to_string())?;
2640→    Ok(())
2641→}
2642→
2643→#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
2644→#[serde(rename_all = "camelCase")]
2645→enum RegKind {
2646→    Dword,
2647→    String,
2648→}
2649→
2650→#[derive(serde::Serialize, serde::Deserialize, Clone)]
2651→#[serde(rename_all = "camelCase")]
2652→struct RegOp {
2653→    hive: String,
2654→    path: String,
2655→    name: String,
2656→    #[allow(dead_code)]
2657→    kind: RegKind,
2658→}
2659→
2660→#[derive(serde::Serialize, serde::Deserialize, Clone)]
2661→#[serde(rename_all = "camelCase")]
2662→struct OptimizeItem {
2663→    key: String,
2664→    title: String,
2665→    desc: String,
2666→    reg: Vec<RegOp>,
2667→    service: Option<String>,
2668→}
2669→
2670→#[derive(serde::Serialize)]
2671→struct OptimizeState {
2672→    key: String,
2673→    enabled: bool,
2674→}
2675→
2676→fn reg_root(name: &str) -> Option<winreg::RegKey> {
2677→    use winreg::enums::*;
2678→    Some(match name {
2679→        "HKLM" => HKEY_LOCAL_MACHINE,
2680→        "HKCU" => HKEY_CURRENT_USER,
2681→        _ => return None,
2682→    })
2683→    .map(|h| winreg::RegKey::predef(h))
2684→}
2685→
2686→fn read_reg_dword(hive: &str, path: &str, name: &str) -> Option<u32> {
2687→    let root = reg_root(hive)?;
2688→    let key = root.open_subkey(path).ok()?;
2689→    key.get_value::<u32, _>(name).ok()
2690→}
2691→
2692→fn read_reg_string(hive: &str, path: &str, name: &str) -> Option<String> {
2693→    let root = reg_root(hive)?;
2694→    let key = root.open_subkey(path).ok()?;
2695→    key.get_value::<String, _>(name).ok()
2696→}
2697→
2698→fn write_reg_dword(hive: &str, path: &str, name: &str, value: u32) -> Result<(), String> {
2699→    let root = reg_root(hive).ok_or("无效的 hive")?;
2700→    let (key, _disp) = root.create_subkey(path).map_err(|e| e.to_string())?;
2701→    key.set_value(name, &value).map_err(|e| e.to_string())?;
2702→    Ok(())
2703→}
2704→
2705→fn write_reg_string(hive: &str, path: &str, name: &str, value: &str) -> Result<(), String> {
2706→    let root = reg_root(hive).ok_or("无效的 hive")?;
2707→    let (key, _disp) = root.create_subkey(path).map_err(|e| e.to_string())?;
2708→    key.set_value(name, &value).map_err(|e| e.to_string())?;
2709→    Ok(())
2710→}
2711→
2712→fn control_service(name: &str, stop: bool) -> Result<(), String> {
2713→    use std::os::windows::process::CommandExt;
2714→    use std::process::Command;
2715→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2716→    let action = if stop { "stop" } else { "start" };
2717→    let out = Command::new("sc")
2718→        .args([action, name])
2719→        .creation_flags(CREATE_NO_WINDOW)
2720→        .output()
2721→        .map_err(|e| e.to_string())?;
2722→    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
2723→    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
2724→    if !out.status.success() || stdout.contains("FAILED") || stdout.contains("拒绝") {
2725→        let detail = if !stdout.trim().is_empty() { stdout.trim() } else { stderr.trim() };
2726→        let hint = if stdout.contains("access") || stdout.contains("5") || stdout.contains("拒绝") {
2727→            "（需要管理员权限，且部分受保护服务需先禁用其启动类型）"
2728→        } else {
2729→            ""
2730→        };
2731→        return Err(format!("{}{}", detail, if hint.is_empty() { "" } else { hint }));
2732→    }
2733→    Ok(())
2734→}
2735→
2736→/// 设置服务启动类型（disabled=禁用 / auto=自动 / demand=手动）
2737→fn config_service_start(name: &str, mode: &str) -> Result<(), String> {
2738→    use std::os::windows::process::CommandExt;
2739→    use std::process::Command;
2740→    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
2741→    let out = Command::new("sc")
2742→        .args(["config", name, "start=", mode])
2743→        .creation_flags(CREATE_NO_WINDOW)
2744→        .output()
2745→        .map_err(|e| e.to_string())?;
2746→    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
2747→    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
2748→    if !out.status.success() || stdout.contains("FAILED") || stdout.contains("拒绝") {
2749→        let detail = if !stdout.trim().is_empty() { stdout.trim() } else { stderr.trim() };
2750→        return Err(detail.to_string());
2751→    }
2752→    Ok(())
2753→}
2754→
2755→fn is_service_running(name: &str) -> bool {
2756→    use std::process::Command;
2757→    use std::os::windows::process::CommandExt;
2758→    Command::new("sc")
2759→        .args(["query", name])
2760→        .creation_flags(0x0800_0000)
2761→        .output()
2762→        .map(|o| {
2763→            let s = String::from_utf8_lossy(&o.stdout).to_lowercase();
2764→            s.contains("running")
2765→        })
2766→        .unwrap_or(false)
2767→}
2768→
2769→/// 优化项配置：每一项定义注册表/服务，前端按 key 查询状态
2770→fn optimize_items() -> Vec<OptimizeItem> {
2771→    vec![
2772→        OptimizeItem {
2773→            key: "smartscreen".into(),
2774→            title: "SmartScreen".into(),
2775→            desc: "应用与文件信誉检查".into(),
2776→            reg: vec![
2777→                RegOp {
2778→                    hive: "HKLM".into(),
2779→                    path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer".into(),
2780→                    name: "SmartScreenEnabled".into(),
2781→                    kind: RegKind::String,
2782→                },
2783→                RegOp {
2784→                    hive: "HKCU".into(),
2785→                    path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppHost".into(),
2786→                    name: "EnableWebContentEvaluation".into(),
2787→                    kind: RegKind::Dword,
2788→                },
2789→            ],
2790→            service: None,
2791→        },
2792→        OptimizeItem {
2793→            key: "uac".into(),
2794→            title: "UAC 提示".into(),
2795→            desc: "用户账户控制弹窗".into(),
2796→            reg: vec![RegOp {
2797→                hive: "HKLM".into(),
2798→                path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".into(),
2799→                name: "EnableLUA".into(),
2800→                kind: RegKind::Dword,
2801→            }],
2802→            service: None,
2803→        },
2804→        OptimizeItem {
2805→            key: "amsi".into(),
2806→            title: "AMSI".into(),
2807→            desc: "反恶意软件扫描接口".into(),
2808→            reg: vec![RegOp {
2809→                hive: "HKLM".into(),
2810→                path: r"SOFTWARE\Microsoft\AMSI\Provider".into(),
2811→                name: "Enabled".into(),
2812→                kind: RegKind::Dword,
2813→            }],
2814→            service: None,
2815→        },
2816→        OptimizeItem {
2817→            key: "stickykeys".into(),
2818→            title: "粘滞键提示".into(),
2819→            desc: "连按 Shift 弹出粘滞键".into(),
2820→            reg: vec![
2821→                RegOp {
2822→                    hive: "HKCU".into(),
2823→                    path: r"Control Panel\Accessibility\StickyKeys".into(),
2824→                    name: "Flags".into(),
2825→                    kind: RegKind::String,
2826→                },
2827→            ],
2828→            service: None,
2829→        },
2830→    ]
2831→}
2832→
2833→/// 查询所有优化项的当前状态
2834→/// 约定：enabled=true 表示该功能已开启（需关闭以优化），false 表示已关闭
2835→#[command]
2836→fn optimize_states() -> Vec<OptimizeState> {
2837→    optimize_items()
2838→        .iter()
2839→        .map(|item| {
2840→            let enabled = if let Some(svc) = &item.service {
2841→                is_service_running(svc)
2842→            } else if !item.reg.is_empty() {
2843→                let r = &item.reg[0];
2844→                match r.kind {
2845→                    RegKind::Dword => read_reg_dword(&r.hive, &r.path, &r.name).unwrap_or(0) != 0,
2846→                    RegKind::String => read_reg_string(&r.hive, &r.path, &r.name)
2847→                        .map(|v| !v.is_empty() && v != "Off" && v != "0")
2848→                        .unwrap_or(false),
2849→                }
2850→            } else {
2851→                false
2852→            };
2853→            OptimizeState {
2854→                key: item.key.clone(),
2855→                enabled,
2856→            }
2857→        })
2858→        .collect()
2859→}
2860→
2861→/// 设置某个优化项（enable=true 开启功能，false 关闭以优化）
2862→#[command]
2863→async fn optimize_set(key: String, enable: bool) -> Result<(), String> {
2864→    let item = optimize_items().into_iter().find(|i| i.key == key)
2865→        .ok_or_else(|| "未找到优化项".to_string())?;
2866→    let task = tauri::async_runtime::spawn_blocking(move || {
2867→        if enable {
2868→            if let Some(svc) = &item.service {
2869→                let _ = config_service_start(svc, "auto");
2870→                let _ = control_service(svc, false);
2871→            }
2872→            for r in &item.reg {
2873→                match r.kind {
2874→                    RegKind::Dword => write_reg_dword(&r.hive, &r.path, &r.name, 1)?,
2875→                    RegKind::String => write_reg_string(&r.hive, &r.path, &r.name, "On")?,
2876→                }
2877→            }
2878→        } else {
2879→            for r in &item.reg {
2880→                match r.kind {
2881→                    RegKind::Dword => write_reg_dword(&r.hive, &r.path, &r.name, 0)?,
2882→                    RegKind::String => write_reg_string(&r.hive, &r.path, &r.name, "Off")?,
2883→                }
2884→            }
2885→            if let Some(svc) = &item.service {
2886→                let _ = config_service_start(svc, "disabled");
2887→                let _ = control_service(svc, true);
2888→            }
2889→        }
2890→        Ok::<(), String>(())
2891→    })
2892→    .await
2893→    .map_err(|e| e.to_string())?;
2894→    task
2895→}
2896→
2897→#[repr(C)]
2898→#[allow(non_snake_case)]
2899→struct AccentPolicy {
2900→    AccentState: u32,
2901→    AccentFlags: u32,
2902→    GradientColor: u32,
2903→    AnimationId: u32,
2904→}
2905→
2906→#[repr(C)]
2907→#[allow(non_snake_case)]
2908→struct WindowCompositionAttribData {
2909→    Attrib: u32,
2910→    pvData: *mut std::ffi::c_void,
2911→    cbData: usize,
2912→}
2913→
2914→const WCA_ACCENT_POLICY: u32 = 19;
2915→const ACCENT_DISABLE: u32 = 0;
2916→const ACCENT_ENABLE_BLURBEHIND: u32 = 3;
2917→const ACCENT_ENABLE_ACRYLICBLURBEHIND: u32 = 4;
2918→
2919→#[command]
2920→fn set_window_backdrop(app: tauri::AppHandle, backdrop: u32) -> Result<(), String> {
2921→    use tauri::Manager;
2922→    use windows::Win32::Graphics::Dwm::{
2923→        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
2924→        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DEFAULT, DWMWCP_ROUND,
2925→    };
2926→
2927→    let window = app.get_webview_window("main").ok_or("未找到主窗口")?;
2928→    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
2929→
2930→    if backdrop != 1 {
2931→        let value = backdrop as i32;
2932→        let r = unsafe {
2933→            DwmSetWindowAttribute(
2934→                hwnd,
2935→                DWMWA_SYSTEMBACKDROP_TYPE,
2936→                &value as *const _ as *const _,
2937→                std::mem::size_of::<i32>() as u32,
2938→            )
2939→        };
2940→        if r.is_ok() {
2941→            let _ = set_win32_accent(hwnd, 0);
2942→            let border_color: u32 = 0x00000000;
2943→            let _ = unsafe {
2944→                DwmSetWindowAttribute(
2945→                    hwnd,
2946→                    DWMWA_BORDER_COLOR,
2947→                    &border_color as *const _ as *const _,
2948→                    std::mem::size_of::<u32>() as u32,
2949→                )
2950→            };
2951→            let _ = unsafe {
2952→                DwmSetWindowAttribute(
2953→                    hwnd,
2954→                    DWMWA_WINDOW_CORNER_PREFERENCE,
2955→                    &DWMWCP_ROUND as *const _ as *const _,
2956→                    std::mem::size_of::<u32>() as u32,
2957→                )
2958→            };
2959→            return Ok(());
2960→        }
2961→    }
2962→
2963→    let accent_state = match backdrop {
2964→        2 => ACCENT_ENABLE_BLURBEHIND,
2965→        3 => ACCENT_ENABLE_ACRYLICBLURBEHIND,
2966→        _ => ACCENT_DISABLE,
2967→    };
2968→    if accent_state == ACCENT_DISABLE {
2969→        let border_default: u32 = 0xFFFFFFFF;
2970→        let _ = unsafe {
2971→            DwmSetWindowAttribute(
2972→                hwnd,
2973→                DWMWA_BORDER_COLOR,
2974→                &border_default as *const _ as *const _,
2975→                std::mem::size_of::<u32>() as u32,
2976→            )
2977→        };
2978→        let _ = unsafe {
2979→            DwmSetWindowAttribute(
2980→                hwnd,
2981→                DWMWA_WINDOW_CORNER_PREFERENCE,
2982→                &DWMWCP_DEFAULT as *const _ as *const _,
2983→                std::mem::size_of::<u32>() as u32,
2984→            )
2985→        };
2986→    }
2987→    set_win32_accent(hwnd, accent_state)?;
2988→    Ok(())
2989→}
2990→
2991→#[command]
2992→fn refresh_window_backdrop(app: tauri::AppHandle, backdrop: u32) -> Result<(), String> {
2993→    use tauri::Manager;
2994→    use windows::Win32::Graphics::Dwm::DwmFlush;
2995→    use windows::Win32::Graphics::Gdi::InvalidateRect;
2996→    use windows::Win32::UI::WindowsAndMessaging::{
2997→        SetWindowPos, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
2998→        SWP_NOZORDER,
2999→    };
3000→    let _ = backdrop;
3001→    let window = app.get_webview_window("main").ok_or("未找到主窗口")?;
3002→    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
3003→    unsafe {
3004→        let _ = SetWindowPos(
3005→            hwnd,
3006→            None,
3007→            0,
3008→            0,
3009→            0,
3010→            0,
3011→            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
3012→        );
3013→        let _ = InvalidateRect(Some(hwnd), None, true);
3014→        let _ = DwmFlush();
3015→    }
3016→    Ok(())
3017→}
3018→
3019→/// 桌面窗口信息
3020→#[derive(serde::Serialize)]
3021→struct WindowInfo {
3022→    hwnd: usize,
3023→    pid: u32,
3024→    title: String,
3025→    class_name: String,
3026→    exe_path: String,
3027→    exe_name: String,
3028→    is_visible: bool,
3029→    is_topmost: bool,
3030→    is_click_through: bool,
3031→    is_minimized: bool,
3032→    is_maximized: bool,
3033→    opacity: u8,
3034→    icon: Option<String>,
3035→}
3036→
3037→/// 从窗口提取 HICON 并转 PNG base64（自适应尺寸）
3038→fn extract_window_icon(hwnd: windows::Win32::Foundation::HWND) -> Option<String> {
3039→    use windows::Win32::Foundation::{LPARAM, WPARAM};
3040→    use windows::Win32::UI::WindowsAndMessaging::{
3041→        GetClassLongPtrW, SendMessageW, GCLP_HICON, HICON, ICON_BIG, ICON_SMALL, WM_GETICON,
3042→    };
3043→    let mut hicon: HICON = HICON::default();
3044→    unsafe {
3045→        let r = SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(0)));
3046→        if r.0 != 0 {
3047→            hicon = HICON(r.0 as *mut _);
3048→        } else {
3049→            let r2 = SendMessageW(hwnd, WM_GETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(0)));
3050→            if r2.0 != 0 {
3051→                hicon = HICON(r2.0 as *mut _);
3052→            } else {
3053→                let cls = GetClassLongPtrW(hwnd, GCLP_HICON);
3054→                if cls != 0 {
3055→                    hicon = HICON(cls as *mut _);
3056→                }
3057→            }
3058→        }
3059→    }
3060→    if hicon.is_invalid() {
3061→        return None;
3062→    }
3063→    icon_to_png_base64_auto(hicon)
3064→}
3065→
3066→/// 自适应尺寸的 HICON -> PNG base64
3067→fn icon_to_png_base64_auto(hicon: windows::Win32::UI::WindowsAndMessaging::HICON) -> Option<String> {
3068→    use windows::Win32::Graphics::Gdi::{
3069→        CreateCompatibleDC, DeleteObject, DeleteDC, GetDIBits, GetObjectW,
3070→        BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_USAGE, RGBQUAD,
3071→    };
3072→    use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
3073→
3074→    let mut icon_info = ICONINFO::default();
3075→    unsafe { GetIconInfo(hicon, &mut icon_info).ok()? };
3076→
3077→    let mut bmp = BITMAP::default();
3078→    let has_size = unsafe {
3079→        GetObjectW(
3080→            icon_info.hbmColor.into(),
3081→            std::mem::size_of::<BITMAP>() as i32,
3082→            Some(&mut bmp as *mut _ as *mut _),
3083→        ) != 0
3084→    };
3085→    let w = if has_size && bmp.bmWidth > 0 { bmp.bmWidth } else { 32 };
3086→    let h = if has_size && bmp.bmHeight > 0 { bmp.bmHeight } else { 32 };
3087→
3088→    let hdc = unsafe { CreateCompatibleDC(None) };
3089→    if hdc.is_invalid() {
3090→        unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
3091→        unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }
3092→        return None;
3093→    }
3094→
3095→    let mut bi = BITMAPINFOHEADER::default();
3096→    bi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
3097→    bi.biWidth = w;
3098→    bi.biHeight = -h;
3099→    bi.biPlanes = 1;
3100→    bi.biBitCount = 32;
3101→    bi.biCompression = 0;
3102→
3103→    let mut bmi = BITMAPINFO {
3104→        bmiHeader: bi,
3105→        bmiColors: [RGBQUAD::default(); 1],
3106→    };
3107→
3108→    let mut pixels: Vec<u8> = vec![0u8; (w * h * 4) as usize];
3109→    let rows = unsafe {
3110→        GetDIBits(
3111→            hdc,
3112→            icon_info.hbmColor,
3113→            0,
3114→            h as u32,
3115→            Some(pixels.as_mut_ptr() as *mut _),
3116→            &mut bmi,
3117→            DIB_USAGE(0),
3118→        )
3119→    };
3120→
3121→    unsafe { let _ = DeleteDC(hdc); }
3122→    unsafe { let _ = DeleteObject(icon_info.hbmColor.into()); }
3123→    unsafe { let _ = DeleteObject(icon_info.hbmMask.into()); }
3124→
3125→    if rows == 0 {
3126→        return None;
3127→    }
3128→
3129→    let png = rgba_to_png(w as u32, h as u32, &pixels)?;
3130→    use base64::Engine;
3131→    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
3132→    Some(format!("data:image/png;base64,{}", b64))
3133→}
3134→
3135→/// 从 HWND 获取所属进程 PID
3136→fn hwnd_to_pid(hwnd: windows::Win32::Foundation::HWND) -> u32 {
3137→    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;
3138→    let mut pid: u32 = 0;
3139→    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
3140→    pid
3141→}
3142→
3143→/// 从 PID 获取 exe 路径（QueryFullProcessImageNameW）
3144→fn pid_to_exe(pid: u32) -> (String, String) {
3145→    use std::os::windows::ffi::OsStringExt;
3146→    use windows::core::PWSTR;
3147→    use windows::Win32::Foundation::CloseHandle;
3148→    use windows::Win32::System::Threading::{
3149→        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
3150→        PROCESS_QUERY_LIMITED_INFORMATION,
3151→    };
3152→
3153→    let exe_path: String = {
3154→        let handle = unsafe {
3155→            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
3156→        };
3157→        let Ok(handle) = handle else { return (String::new(), String::new()) };
3158→        let mut buf = [0u16; 1024];
3159→        let mut len: u32 = buf.len() as u32;
3160→        let ok = unsafe {
3161→            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
3162→        };
3163→        let _ = unsafe { CloseHandle(handle) };
3164→        if ok.is_err() || len == 0 {
3165→            return (String::new(), String::new());
3166→        }
3167→        let os_str: std::ffi::OsString =
3168→            <std::ffi::OsString as OsStringExt>::from_wide(&buf[..len as usize]);
3169→        os_str.to_string_lossy().to_string()
3170→    };
3171→
3172→    let exe_name = std::path::Path::new(&exe_path)
3173→        .file_name()
3174→        .map(|n| n.to_string_lossy().to_string())
3175→        .unwrap_or_default();
3176→
3177→    (exe_path, exe_name)
3178→}
3179→
3180→#[derive(serde::Serialize)]
3181→struct HoveredWindow {
3182→    hwnd: usize,
3183→    pid: u32,
3184→    title: String,
3185→    exe_name: String,
3186→    x: i32,
3187→    y: i32,
3188→    width: i32,
3189→    height: i32,
3190→    is_self: bool,
3191→    smooth: bool,
3192→    ease: u32,
3193→}
3194→
3195→static HOVER_ENABLED: AtomicBool = AtomicBool::new(true);
3196→static HUD_SMOOTH: AtomicBool = AtomicBool::new(true);
3197→static HUD_EASE_MS: AtomicU32 = AtomicU32::new(180);
3198→
3199→#[tauri::command]
3200→fn set_hud_ease(ms: u32) {
3201→    let clamped = ms.clamp(60, 600);
3202→    HUD_EASE_MS.store(clamped, Ordering::Relaxed);
3203→}
3204→
3205→#[tauri::command]
3206→fn set_hover_overlay(enabled: bool) {
3207→    HOVER_ENABLED.store(enabled, Ordering::Relaxed);
3208→}
3209→
3210→#[tauri::command]
3211→fn set_hud_smooth(enabled: bool) {
3212→    HUD_SMOOTH.store(enabled, Ordering::Relaxed);
3213→}
3214→
3215→#[derive(serde::Serialize)]
3216→struct VisualState {
3217→    hover: bool,
3218→    smooth: bool,
3219→    ease: u32,
3220→}
3221→
3222→#[tauri::command]
3223→fn get_visual_state() -> VisualState {
3224→    VisualState {
3225→        hover: HOVER_ENABLED.load(Ordering::Relaxed),
3226→        smooth: HUD_SMOOTH.load(Ordering::Relaxed),
3227→        ease: HUD_EASE_MS.load(Ordering::Relaxed),
3228→    }
3229→}
3230→
3231→#[tauri::command]
3232→fn get_window_under_cursor() -> Option<HoveredWindow> {
3233→    if !HOVER_ENABLED.load(Ordering::Relaxed) {
3234→        return None;
3235→    }
3236→    use windows::Win32::Foundation::POINT;
3237→    use windows::Win32::UI::WindowsAndMessaging::{
3238→        GetCursorPos, GetWindowRect, GetWindowTextW, WindowFromPoint,
3239→    };
3240→
3241→    let mut pt = POINT::default();
3242→    if unsafe { GetCursorPos(&mut pt) }.is_err() {
3243→        return None;
3244→    }
3245→    let hwnd = unsafe { WindowFromPoint(pt) };
3246→    if hwnd.0.is_null() {
3247→        return None;
3248→    }
3249→
3250→    let mut title_buf = [0u16; 512];
3251→    let title_len = unsafe { GetWindowTextW(hwnd, &mut title_buf) };
3252→    let title = if title_len > 0 {
3253→        String::from_utf16_lossy(&title_buf[..title_len as usize])
3254→    } else {
3255→        String::new()
3256→    };
3257→
3258→    let mut rc = windows::Win32::Foundation::RECT::default();
3259→    if unsafe { GetWindowRect(hwnd, &mut rc) }.is_err() {
3260→        return None;
3261→    }
3262→
3263→    let pid = hwnd_to_pid(hwnd);
3264→    let (_exe_path, exe_name) = pid_to_exe(pid);
3265→    let is_self = pid == std::process::id();
3266→
3267→    Some(HoveredWindow {
3268→        hwnd: hwnd.0 as usize,
3269→        pid,
3270→        title,
3271→        exe_name,
3272→        x: rc.left,
3273→        y: rc.top,
3274→        width: rc.right - rc.left,
3275→        height: rc.bottom - rc.top,
3276→        is_self,
3277→        smooth: HUD_SMOOTH.load(Ordering::Relaxed),
3278→        ease: HUD_EASE_MS.load(Ordering::Relaxed),
3279→    })
3280→}
3281→
3282→/// 枚举所有桌面窗口
3283→#[tauri::command]
3284→fn list_windows() -> Vec<WindowInfo> {
3285→    use windows::core::BOOL;
3286→    use windows::Win32::Foundation::{COLORREF, HWND, LPARAM};
3287→    use windows::Win32::UI::WindowsAndMessaging::{
3288→        EnumWindows, GetClassNameW, GetLayeredWindowAttributes,
3289→        GetWindowTextW, GetWindowLongW, IsIconic, IsWindowVisible, IsZoomed,
3290→        GWL_EXSTYLE, LWA_ALPHA, WNDENUMPROC, WS_EX_LAYERED, WS_EX_TOPMOST,
3291→        WS_EX_TRANSPARENT,
3292→    };
3293→
3294→    let mut result: Vec<WindowInfo> = Vec::new();
3295→    let result_ptr = &mut result as *mut Vec<WindowInfo>;
3296→
3297→    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
3298→        let result = &mut *(lparam.0 as *mut Vec<WindowInfo>);
3299→
3300→        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
3301→        let is_visible = IsWindowVisible(hwnd).as_bool();
3302→        let is_layered = (ex_style & WS_EX_LAYERED.0) != 0;
3303→
3304→        let mut title_buf = [0u16; 512];
3305→        let title_len = GetWindowTextW(hwnd, &mut title_buf);
3306→        let title = if title_len > 0 {
3307→            String::from_utf16_lossy(&title_buf[..title_len as usize])
3308→        } else {
3309→            String::new()
3310→        };
3311→
3312→        let mut class_buf = [0u16; 256];
3313→        let class_len = GetClassNameW(hwnd, &mut class_buf);
3314→        let class_name = if class_len > 0 {
3315→            String::from_utf16_lossy(&class_buf[..class_len as usize])
3316→        } else {
3317→            String::new()
3318→        };
3319→
3320→        let pid = hwnd_to_pid(hwnd);
3321→        let (exe_path, exe_name) = pid_to_exe(pid);
3322→
3323→        let is_topmost = (ex_style & WS_EX_TOPMOST.0) != 0;
3324→        let is_click_through =
3325→            (ex_style & WS_EX_TRANSPARENT.0) != 0 && is_layered;
3326→        let is_minimized = IsIconic(hwnd).as_bool();
3327→        let is_maximized = IsZoomed(hwnd).as_bool();
3328→
3329→        let mut alpha: u8 = 255;
3330→        if is_layered {
3331→            let mut colorref = COLORREF(0);
3332→            let mut a: u8 = 0;
3333→            let mut flags = windows::Win32::UI::WindowsAndMessaging::LAYERED_WINDOW_ATTRIBUTES_FLAGS::default();
3334→            let ok = GetLayeredWindowAttributes(
3335→                hwnd,
3336→                Some(&mut colorref),
3337→                Some(&mut a),
3338→                Some(&mut flags),
3339→            );
3340→            if ok.is_ok() && (flags.0 & LWA_ALPHA.0) != 0 {
3341→                alpha = a;
3342→            }
3343→        }
3344→
3345→        let icon = extract_window_icon(hwnd);
3346→
3347→        result.push(WindowInfo {
3348→            hwnd: hwnd.0 as usize,
3349→            pid,
3350→            title,
3351→            class_name,
3352→            exe_path,
3353→            exe_name,
3354→            is_visible,
3355→            is_topmost,
3356→            is_click_through,
3357→            is_minimized,
3358→            is_maximized,
3359→            opacity: alpha,
3360→            icon,
3361→        });
3362→        BOOL(1)
3363→    }
3364→
3365→    let func: WNDENUMPROC = Some(enum_proc);
3366→    unsafe {
3367→        let _ = EnumWindows(func, LPARAM(result_ptr as isize));
3368→    }
3369→
3370→    result.sort_by(|a, b| a.exe_name.to_lowercase().cmp(&b.exe_name.to_lowercase()));
3371→    result
3372→}
3373→
3374→#[tauri::command]
3375→fn window_close_task(app: tauri::AppHandle, hwnd: usize) -> Result<(), String> {
3376→    use tauri::Manager;
3377→    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
3378→    use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
3379→    let target_raw = hwnd as *mut _;
3380→    for w in app.webview_windows().values() {
3381→        if let Ok(h) = w.hwnd() {
3382→            if h.0 == target_raw {
3383→                return Err("禁止关闭 ToolsPlus 自身窗口".to_string());
3384→            }
3385→        }
3386→    }
3387→    let target = HWND(target_raw);
3388→    let r = unsafe { PostMessageW(Some(target), WM_CLOSE, WPARAM(0), LPARAM(0)) };
3389→    r.map_err(|e| e.to_string())
3390→}
3391→
3392→#[tauri::command]
3393→fn window_destroy(app: tauri::AppHandle, hwnd: usize) -> Result<(), String> {
3394→    use tauri::Manager;
3395→    use windows::Win32::Foundation::HWND;
3396→    use windows::Win32::UI::WindowsAndMessaging::DestroyWindow;
3397→    let target_raw = hwnd as *mut _;
3398→    for w in app.webview_windows().values() {
3399→        if let Ok(h) = w.hwnd() {
3400→            if h.0 == target_raw {
3401→                return Err("禁止销毁 ToolsPlus 自身窗口".to_string());
3402→            }
3403→        }
3404→    }
3405→    let target = HWND(target_raw);
3406→    let r = unsafe { DestroyWindow(target) };
3407→    r.map_err(|e| e.to_string())
3408→}
3409→
3410→#[tauri::command]
3411→fn window_set_topmost(hwnd: usize, topmost: bool) -> Result<(), String> {
3412→    use windows::Win32::Foundation::HWND;
3413→    use windows::Win32::UI::WindowsAndMessaging::{
3414→        SetWindowPos, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
3415→        SWP_SHOWWINDOW,
3416→    };
3417→    let hwnd = HWND(hwnd as *mut _);
3418→    let after = if topmost { HWND_TOPMOST } else { HWND_NOTOPMOST };
3419→    let r = unsafe {
3420→        SetWindowPos(
3421→            hwnd,
3422→            Some(after),
3423→            0, 0, 0, 0,
3424→            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
3425→        )
3426→    };
3427→    r.map_err(|e| e.to_string())
3428→}
3429→
3430→#[tauri::command]
3431→fn window_set_click_through(hwnd: usize, enabled: bool) -> Result<(), String> {
3432→    use windows::Win32::Foundation::{COLORREF, HWND};
3433→    use windows::Win32::UI::WindowsAndMessaging::{
3434→        GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes, GWL_EXSTYLE, LWA_ALPHA,
3435→        WS_EX_LAYERED, WS_EX_TRANSPARENT,
3436→    };
3437→    let hwnd = HWND(hwnd as *mut _);
3438→    let cur = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
3439→    let new = if enabled {
3440→        cur | WS_EX_TRANSPARENT.0 | WS_EX_LAYERED.0
3441→    } else {
3442→        cur & !(WS_EX_TRANSPARENT.0)
3443→    };
3444→    unsafe { SetWindowLongW(hwnd, GWL_EXSTYLE, new as i32) };
3445→    if enabled {
3446→        let r = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), 200, LWA_ALPHA) };
3447→        r.map_err(|e| e.to_string())?;
3448→    }
3449→    Ok(())
3450→}
3451→
3452→#[tauri::command]
3453→fn window_set_opacity(hwnd: usize, opacity: u8) -> Result<(), String> {
3454→    use windows::Win32::Foundation::{COLORREF, HWND};
3455→    use windows::Win32::UI::WindowsAndMessaging::{
3456→        GetWindowLongW, SetWindowLongW, SetLayeredWindowAttributes, GWL_EXSTYLE, LWA_ALPHA,
3457→        WS_EX_LAYERED,
3458→    };
3459→    let hwnd = HWND(hwnd as *mut _);
3460→    let cur = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
3461→    if (cur & WS_EX_LAYERED.0) == 0 {
3462→        unsafe { SetWindowLongW(hwnd, GWL_EXSTYLE, (cur | WS_EX_LAYERED.0) as i32) };
3463→    }
3464→    let r = unsafe { SetLayeredWindowAttributes(hwnd, COLORREF(0), opacity, LWA_ALPHA) };
3465→    r.map_err(|e| e.to_string())
3466→}
3467→
3468→#[tauri::command]
3469→fn window_minimize(hwnd: usize) -> Result<(), String> {
3470→    use windows::Win32::Foundation::HWND;
3471→    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_MINIMIZE};
3472→    let hwnd = HWND(hwnd as *mut _);
3473→    unsafe { let _ = ShowWindow(hwnd, SW_MINIMIZE); }
3474→    Ok(())
3475→}
3476→
3477→#[tauri::command]
3478→fn window_redraw(hwnd: usize) -> Result<(), String> {
3479→    use windows::Win32::Foundation::HWND;
3480→    use windows::Win32::Graphics::Dwm::DwmFlush;
3481→    use windows::Win32::Graphics::Gdi::{
3482→        InvalidateRect, RedrawWindow, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, UpdateWindow,
3483→    };
3484→    let hwnd = HWND(hwnd as *mut _);
3485→    unsafe {
3486→        let _ = InvalidateRect(Some(hwnd), None, true);
3487→        let _ = RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE | RDW_UPDATENOW | RDW_ERASE);
3488→        let _ = UpdateWindow(hwnd);
3489→        let _ = DwmFlush();
3490→    }
3491→    Ok(())
3492→}
3493→
3494→#[tauri::command]
3495→fn window_copy_path(hwnd: usize) -> Result<String, String> {
3496→    use windows::Win32::Foundation::HWND;
3497→    let hwnd = HWND(hwnd as *mut _);
3498→    let pid = hwnd_to_pid(hwnd);
3499→    if pid == 0 {
3500→        return Err("无法获取窗口进程 PID".into());
3501→    }
3502→    let (path, _name) = pid_to_exe(pid);
3503→    if path.is_empty() {
3504→        return Err("无法获取进程 exe 路径".into());
3505→    }
3506→    Ok(path)
3507→}
3508→
3509→fn set_win32_accent(hwnd: windows::Win32::Foundation::HWND, accent_state: u32) -> Result<(), String> {
3510→    use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
3511→
3512→    type SetWindowCompositionAttribute =
3513→        unsafe extern "system" fn(windows::Win32::Foundation::HWND, *const WindowCompositionAttribData) -> i32;
3514→
3515→    let user32 = unsafe { GetModuleHandleW(windows::core::w!("user32.dll")) }
3516→        .map_err(|e| format!("无法获取 user32: {e}"))?;
3517→    let addr = unsafe { GetProcAddress(user32, windows::core::s!("SetWindowCompositionAttribute")) }
3518→        .ok_or_else(|| "找不到 SetWindowCompositionAttribute".to_string())?;
3519→    let func: SetWindowCompositionAttribute = unsafe { std::mem::transmute(addr) };
3520→
3521→    let policy = AccentPolicy {
3522→        AccentState: accent_state,
3523→        AccentFlags: 0,
3524→        GradientColor: 0x99000000,
3525→        AnimationId: 0,
3526→    };
3527→    let data = WindowCompositionAttribData {
3528→        Attrib: WCA_ACCENT_POLICY,
3529→        pvData: &policy as *const _ as *mut _,
3530→        cbData: std::mem::size_of::<AccentPolicy>(),
3531→    };
3532→    let ok = unsafe { func(hwnd, &data) };
3533→    if ok == 0 {
3534→        return Err("SetWindowCompositionAttribute 返回 0".to_string());
3535→    }
3536→    Ok(())
3537→}
3538→
3539→static KS_ENABLED: AtomicBool = AtomicBool::new(false);
3540→static mut KS_KEYBOARD_HOOK: Option<HHOOK> = None;
3541→static mut KS_MOUSE_HOOK: Option<HHOOK> = None;
3542→static KS_APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
3543→
3544→fn ks_emit_all(event: &str, payload: serde_json::Value) {
3545→    use tauri::Emitter;
3546→    if let Some(handle) = KS_APP_HANDLE.get() {
3547→        let _ = handle.emit(event, payload);
3548→    }
3549→}
3550→
3551→fn ks_now_ms() -> u64 {
3552→    std::time::SystemTime::now()
3553→        .duration_since(std::time::UNIX_EPOCH)
3554→        .map(|d| u64::try_from(d.as_millis()).unwrap_or(0))
3555→        .unwrap_or(0)
3556→}
3557→
3558→fn vk_to_js_key(vk: u32) -> String {
3559→    match vk {
3560→        8 => "Backspace".to_string(),
3561→        9 => "Tab".to_string(),
3562→        13 => "Enter".to_string(),
3563→        16 => "Shift".to_string(),
3564→        17 => "Control".to_string(),
3565→        18 => "Alt".to_string(),
3566→        27 => "Escape".to_string(),
3567→        32 => " ".to_string(),
3568→        33 => "PageUp".to_string(),
3569→        34 => "PageDown".to_string(),
3570→        35 => "End".to_string(),
3571→        36 => "Home".to_string(),
3572→        37 => "ArrowLeft".to_string(),
3573→        38 => "ArrowUp".to_string(),
3574→        39 => "ArrowRight".to_string(),
3575→        40 => "ArrowDown".to_string(),
3576→        46 => "Delete".to_string(),
3577→        91 | 92 => "Meta".to_string(),
3578→        112..=123 => format!("F{}", vk - 111),
3579→        65..=90 => char::from_u32(vk).map(|c| c.to_string()).unwrap_or_default(),
3580→        48..=57 => format!("{}", vk - 48),
3581→        96..=105 => format!("Num{}", vk - 96),
3582→        186 => ";".to_string(),
3583→        188 => ",".to_string(),
3584→        190 => ".".to_string(),
3585→        191 => "/".to_string(),
3586→        219 => "[".to_string(),
3587→        220 => "\\\\".to_string(),
3588→        221 => "]".to_string(),
3589→        187 => "'".to_string(),
3590→        189 => "\"".to_string(),
3591→        222 => "'".to_string(),
3592→        106 => "\\".to_string(),
3593→        110 => "|".to_string(),
3594→        107 => "`".to_string(),
3595→        _ => format!("VK{}", vk),
3596→    }
3597→}
3598→
3599→fn ks_key_down(vk: u32, bit: u32) -> bool {
3600→    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
3601→    (unsafe { GetKeyState(vk as i32) as u32 } & bit) != 0
3602→}
3603→
3604→unsafe extern "system" fn ks_keyboard_hook(
3605→    code: i32,
3606→    wparam: WPARAM,
3607→    lparam: LPARAM,
3608→) -> LRESULT {
3609→    use windows::Win32::UI::WindowsAndMessaging::{
3610→        CallNextHookEx, KBDLLHOOKSTRUCT, LLKHF_UP,
3611→    };
3612→    if code != 0x0 && code != 0x1 {
3613→        return LRESULT(unsafe { CallNextHookEx(None, code, wparam, lparam).0 });
3614→    }
3615→    let kb = &mut *(lparam.0 as *mut KBDLLHOOKSTRUCT);
3616→    if !kb.flags.contains(LLKHF_UP) && KS_ENABLED.load(Ordering::Relaxed) {
3617→        let vk = kb.vkCode;
3618→        let key = vk_to_js_key(vk);
3619→        let ctrl = ks_key_down(17, 0x8000);
3620→        let shift = ks_key_down(16, 0x8000);
3621→        let alt = ks_key_down(18, 0x8000);
3622→        let meta = ks_key_down(91, 0x8000) || ks_key_down(92, 0x8000);
3623→        let payload = serde_json::json!({
3624→            "key": key,
3625→            "ctrlKey": ctrl,
3626→            "shiftKey": shift,
3627→            "altKey": alt,
3628→            "metaKey": meta,
3629→            "t": ks_now_ms()
3630→        });
3631→        ks_emit_all("ks-key-event", payload);
3632→    }
3633→    LRESULT(unsafe { CallNextHookEx(None, code, wparam, lparam).0 })
3634→}
3635→
3636→unsafe extern "system" fn ks_mouse_hook(
3637→    code: i32,
3638→    wparam: WPARAM,
3639→    lparam: LPARAM,
3640→) -> LRESULT {
3641→    use windows::Win32::UI::WindowsAndMessaging::{
3642→        CallNextHookEx, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_RBUTTONDOWN,
3643→    };
3644→    if !KS_ENABLED.load(Ordering::Relaxed) {
3645→        return LRESULT(unsafe { CallNextHookEx(None, code, wparam, lparam).0 });
3646→    }
3647→    let _ms = &mut *(lparam.0 as *mut MSLLHOOKSTRUCT);
3648→    if code == WM_LBUTTONDOWN as i32 {
3649→        ks_emit_all("ks-reveal", serde_json::json!({ "t": ks_now_ms() }));
3650→        let payload = serde_json::json!({ "button": 0, "t": ks_now_ms() });
3651→        ks_emit_all("ks-mouse-event", payload);
3652→    } else if code == WM_RBUTTONDOWN as i32 {
3653→        let payload = serde_json::json!({ "button": 2, "t": ks_now_ms() });
3654→        ks_emit_all("ks-mouse-event", payload);
3655→    }
3656→    LRESULT(unsafe { CallNextHookEx(None, code, wparam, lparam).0 })
3657→}
3658→
3659→#[tauri::command]
3660→fn set_ks_enabled(enabled: bool) {
3661→    use windows::Win32::UI::WindowsAndMessaging::{
3662→        SetWindowsHookExA, UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL,
3663→    };
3664→    if enabled {
3665→        if unsafe { KS_KEYBOARD_HOOK.is_none() } {
3666→            let khook = unsafe {
3667→                SetWindowsHookExA(WH_KEYBOARD_LL, Some(ks_keyboard_hook), None, 0)
3668→            };
3669→            let mhook = unsafe {
3670→                SetWindowsHookExA(WH_MOUSE_LL, Some(ks_mouse_hook), None, 0)
3671→            };
3672→            unsafe {
3673→                KS_KEYBOARD_HOOK = khook.ok();
3674→                KS_MOUSE_HOOK = mhook.ok();
3675→            }
3676→        }
3677→    } else {
3678→        unsafe {
3679→            if let Some(h) = KS_KEYBOARD_HOOK.take() {
3680→                let _ = UnhookWindowsHookEx(h);
3681→            }
3682→            if let Some(h) = KS_MOUSE_HOOK.take() {
3683→                let _ = UnhookWindowsHookEx(h);
3684→            }
3685→        }
3686→    }
3687→    KS_ENABLED.store(enabled, Ordering::Relaxed);
3688→}
3689→
3690→#[tauri::command]
3691→fn emit_ks_reveal() -> tauri::Result<()> {
3692→    ks_emit_all("ks-reveal", serde_json::json!({ "t": ks_now_ms() }));
3693→    Ok(())
3694→}
3695→
3696→#[tauri::command]
3697→fn get_ks_enabled() -> bool {
3698→    KS_ENABLED.load(Ordering::Relaxed)
3699→}
3700→
3701→#[cfg_attr(mobile, tauri::mobile_entry_point)]
3702→pub fn run() {
3703→    eprintln!("[toolsplus] run() entered at {:?}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
3704→    tauri::Builder::default()
3705→        .plugin(tauri_plugin_opener::init())
3706→        .plugin(tauri_plugin_notification::init())
3707→        .invoke_handler(tauri::generate_handler![
3708→            scan_junk,
3709→            clean_junk,
3710→            get_hardware_info,
3711→            get_performance_stats,
3712→            list_processes,
3713→            process_icons,
3714→            kill_process,
3715→            get_foreground_window_pid,
3716→            get_window_under_cursor,
3717→            set_hover_overlay,
3718→            set_hud_smooth,
3719→            set_hud_ease,
3720→            get_visual_state,
3721→            suspend_process,
3722→            resume_process,
3723→            get_ppl_protection,
3724→            restart_as_admin,
3725→            check_git,
3726→            install_git,
3727→            git_default_dir,
3728→            git_repo_root,
3729→            git_status,
3730→            git_log,
3731→            git_add,
3732→            git_unstage,
3733→            git_commit,
3734→            git_branches,
3735→            git_push,
3736→            git_revert,
3737→            git_checkout,
3738→            git_fetch,
3739→            git_pull,
3740→            git_clone,
3741→            git_init,
3742→            pick_folder,
3743→            pick_image,
3744→            read_image_as_data_url,
3745→            gh_auth_state,
3746→            gh_login_web,
3747→            gh_logout,
3748→            gh_login_interactive,
3749→            gh_cancel_login,
3750→            gh_wait_login,
3751→            gh_setup_git,
3752→            install_git_and_gh,
3753→            git_config_user,
3754→            git_get_user_config,
3755→            is_admin,
3756→            relaunch_as_admin,
3757→            optimize_states,
3758→            optimize_set,
3759→            set_window_backdrop,
3760→            refresh_window_backdrop,
3761→            list_windows,
3762→            window_close_task,
3763→            window_destroy,
3764→            window_set_topmost,
3765→            window_set_click_through,
3766→            window_set_opacity,
3767→            window_minimize,
3768→            window_redraw,
3769→            window_copy_path,
3770→            set_ks_enabled,
3771→            get_ks_enabled,
3772→            emit_ks_reveal
3773→        ])
3774→        .setup(|app| {
3775→            let _ = KS_APP_HANDLE.get_or_init(|| app.handle().clone());
3776→            Ok(())
3777→        })
3778→        .run(tauri::generate_context!())
3779→        .expect("error while running tauri application");
3780→}