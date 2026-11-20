// 天气查询桌面应用（Tauri 壳层）
// 只负责：主窗口 + 系统托盘（菜单 / 动态天气图标 / 悬停详情弹窗）
// + 关闭最小化到托盘 + 开机自启开关。
// 前端逻辑全部复用仓库根目录的网页版（由 copy-frontend.js 拷入 ../dist）。
// 托盘菜单文案跟随前端语言；天气图标按 WMO 代码归 8 组（不含温度，
// 温度/详情在悬停弹窗 tray-popup.html 中展示）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Deserialize;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, PhysicalPosition, Position as DpiPosition, Size as DpiSize, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

const MENU_SHOW: &str = "show";
const MENU_AUTOSTART: &str = "autostart";
const MENU_QUIT: &str = "quit";

const POPUP_W: f64 = 320.0;
const POPUP_H: f64 = 192.0;

/* ============================ 语言 / 托盘菜单 ============================ */

/// 托盘菜单与窗口标题文案（跟随前端语言，前端初始化/切换语言时会调用 set_tray_lang）
fn tray_texts(lang: &str) -> (String, String, String, String, String) {
    // (显示主窗口, 开机自启, 退出, 默认 tooltip, 窗口标题)
    match lang {
        "zh" => (
            "显示主窗口".into(),
            "开机自启".into(),
            "退出".into(),
            "天气查询".into(),
            "天气查询".into(),
        ),
        _ => (
            "Show".into(),
            "Launch at login".into(),
            "Quit".into(),
            "Weather".into(),
            "Weather".into(),
        ),
    }
}

/// 按语言构建托盘菜单并挂到托盘上。语言切换/自启勾选翻转时调用。
/// tooltip 优先显示天气快照（城市 · 天气 · 温度），无天气数据时用语言默认文案。
fn rebuild_tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let (t_show, t_auto, t_quit, default_tip, title) = tray_texts(lang);

    let show = MenuItem::with_id(app, MENU_SHOW, t_show, true, None::<&str>)?;
    let auto = CheckMenuItem::with_id(
        app,
        MENU_AUTOSTART,
        t_auto,
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, t_quit, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &auto, &sep, &quit])?;

    // 窗口标题同样跟随语言（配置里的初始标题是英文兜底，前端启动后即被覆盖）
    if let Some(win) = app.get_webview_window("main") {
        win.set_title(&title)?;
    }
    let tray = app.tray_by_id("main").expect("tray must be built in setup");
    tray.set_menu(Some(menu))?;
    let tip = weather_store()
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.tooltip.clone())
        .unwrap_or(default_tip);
    tray.set_tooltip(Some(tip))?;
    Ok(())
}

/// 显示（或从托盘恢复）主窗口
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

// 前端语言切换入口
#[tauri::command]
fn set_tray_lang(app: AppHandle, lang: String) -> Result<(), String> {
    // 语言未变化时跳过：轮询兜底每 2s 会重放，菜单不应被无谓重建
    let unchanged = LANG.with(|l| l.borrow().as_deref() == Some(lang.as_str()));
    if unchanged {
        return Ok(());
    }
    remember_lang(&lang);
    rebuild_tray(&app, &lang).map_err(|e| e.to_string())
}

thread_local! {
    static LANG: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// 供 set_tray_lang 写入缓存
fn remember_lang(lang: &str) {
    LANG.with(|l| *l.borrow_mut() = Some(lang.to_string()));
}

/// 读取当前语言偏好（仅用于菜单重建时的勾选态文案；未知时按英文）
fn current_lang(_app: &AppHandle) -> String {
    LANG.with(|l| l.borrow().clone().unwrap_or_else(|| "en".into()))
}

/* ============================ 天气图标 / tooltip ============================ */

/// 最近一次天气快照：前端 set_tray_weather 上报，仅保留 Rust 侧需要的两项。
#[derive(Clone)]
struct WeatherSnap {
    /// 图标组名：clear/night/partly/cloudy/fog/rain/snow/thunder
    group: String,
    /// 托盘原生 tooltip 文案（已按语言拼好）
    tooltip: String,
}

static WEATHER: OnceLock<Mutex<Option<WeatherSnap>>> = OnceLock::new();
fn weather_store() -> &'static Mutex<Option<WeatherSnap>> {
    WEATHER.get_or_init(|| Mutex::new(None))
}

/// 前端上报的托盘天气快照（字段命名与 app.js buildTraySnapshot 对应）
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeatherPayload {
    lang: String,
    city: String,
    code: i64,
    #[serde(default = "default_true")]
    is_day: bool,
    temp: i64,
    feels: i64,
    desc: String,
}
fn default_true() -> bool {
    true
}

/// WMO 代码 + 昼夜 -> 8 组图标名
fn weather_group(code: i64, is_day: bool) -> &'static str {
    match code {
        0 => {
            if is_day {
                "clear"
            } else {
                "night"
            }
        }
        1 | 2 => "partly",
        3 => "cloudy",
        45 | 48 => "fog",
        51..=67 | 80..=82 => "rain",
        71..=77 | 85 | 86 => "snow",
        95..=99 => "thunder",
        _ => "cloudy",
    }
}

/// 编译期嵌入 8 张预渲染 PNG（源 SVG / 生成脚本见 icons/tray/）
fn group_icon(group: &str) -> Option<tauri::image::Image<'static>> {
    let bytes: &[u8] = match group {
        "clear" => include_bytes!("../icons/tray/clear.png"),
        "night" => include_bytes!("../icons/tray/night.png"),
        "partly" => include_bytes!("../icons/tray/partly.png"),
        "cloudy" => include_bytes!("../icons/tray/cloudy.png"),
        "fog" => include_bytes!("../icons/tray/fog.png"),
        "rain" => include_bytes!("../icons/tray/rain.png"),
        "snow" => include_bytes!("../icons/tray/snow.png"),
        "thunder" => include_bytes!("../icons/tray/thunder.png"),
        _ => return None,
    };
    tauri::image::Image::from_bytes(bytes).ok()
}

/// 按当前快照刷新托盘图标与 tooltip
fn apply_weather(app: &AppHandle) {
    let snap = weather_store().lock().unwrap().clone();
    let Some(snap) = snap else { return };
    let Some(tray) = app.tray_by_id("main") else { return };
    if let Some(icon) = group_icon(&snap.group) {
        let _ = tray.set_icon(Some(icon));
    }
    let _ = tray.set_tooltip(Some(&snap.tooltip));
}

#[tauri::command]
fn set_tray_weather(app: AppHandle, snapshot: WeatherPayload) -> Result<(), String> {
    let group = weather_group(snapshot.code, snapshot.is_day);
    let tooltip = if snapshot.lang == "zh" {
        format!(
            "{} {} {}°（体感 {}°）",
            snapshot.city, snapshot.desc, snapshot.temp, snapshot.feels
        )
    } else {
        format!(
            "{}: {}, {}° (feels {}°)",
            snapshot.city, snapshot.desc, snapshot.temp, snapshot.feels
        )
    };
    // 去重：轮询通道每 2s 会重放同一快照，显示状态不变就不重设图标/tooltip
    {
        let mut store = weather_store().lock().unwrap();
        if let Some(old) = store.as_ref() {
            if old.group == group && old.tooltip == tooltip {
                return Ok(());
            }
        }
        *store = Some(WeatherSnap {
            group: group.to_string(),
            tooltip,
        });
    }
    apply_weather(&app);
    Ok(())
}

/* ============================ 悬停详情弹窗 ============================ */

static POPUP_GEN: OnceLock<Mutex<u64>> = OnceLock::new();
fn popup_gen() -> &'static Mutex<u64> {
    POPUP_GEN.get_or_init(|| Mutex::new(0))
}

/// 不抢前台焦点地显示窗口（Windows：SW_SHOWNOACTIVATE；其他平台直接 show，
/// 弹窗构建时 focused(false)，GTK 已设置 set_accept_focus(false)）。
#[cfg(windows)]
fn popup_show_raw(win: &WebviewWindow) {
    if let Ok(hwnd) = win.hwnd() {
        unsafe {
            windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                hwnd,
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNOACTIVATE,
            );
        }
    } else {
        let _ = win.show();
    }
}
#[cfg(not(windows))]
fn popup_show_raw(win: &WebviewWindow) {
    let _ = win.show();
}

/// 依据托盘图标矩形定位并显示弹窗：底部任务栏显示在图标上方，顶部任务栏显示在下方。
/// 托盘事件给的是 dpi::Position/Size（可能是逻辑坐标），先统一换物理坐标。
fn show_popup(app: &AppHandle, icon_pos: DpiPosition, icon_size: DpiSize) {
    // 任意 Enter 都作废旧的延迟隐藏计划
    *popup_gen().lock().unwrap() += 1;

    let Some(win) = app.get_webview_window("popup") else { return };
    // 多显示器不同缩放时用主显示器缩放仅作 Logical 兜底（Windows 托盘给的是物理坐标）
    let sf = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let pp = icon_pos.to_physical::<f64>(sf);
    let ps = icon_size.to_physical::<f64>(sf);
    let (icon_x, icon_y) = (pp.x, pp.y);
    let (icon_w, icon_h) = (ps.width as i32, ps.height as i32);

    let (pw, ph) = ((POPUP_W * sf) as i32, (POPUP_H * sf) as i32);

    let mon = app
        .monitor_from_point(icon_x, icon_y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());

    let (x, y) = if let Some(m) = mon {
        let mp = m.position();
        let ms = m.size();
        let icon_cx = icon_x as i32 + icon_w / 2;
        let icon_top = icon_y as i32;
        let icon_bottom = icon_top + icon_h;
        // 图标位于屏幕下半区 → 任务栏在底部；否则按顶部处理（左右侧任务栏归为底部样式）
        let bottom_edge =
            icon_y + icon_h as f64 / 2.0 >= mp.y as f64 + ms.height as f64 / 2.0;
        let x = (icon_cx - pw / 2).clamp(mp.x + 6, mp.x + ms.width as i32 - pw - 6);
        let y = if bottom_edge {
            icon_top - ph - 6
        } else {
            icon_bottom + 6
        };
        (x, y)
    } else {
        (icon_x as i32, icon_y as i32 - ph - 6)
    };

    let _ = win.set_position(PhysicalPosition::new(x, y));
    popup_show_raw(&win);
}

fn hide_popup(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("popup") {
        let _ = win.hide();
    }
}

/// 鼠标离开托盘图标后延迟隐藏（经过图标与弹窗之间缝隙、短暂划过都不会闪）；
/// 期间再次 Enter 会令本次计划作废。
fn schedule_popup_hide(app: &AppHandle) {
    let gen = {
        let mut g = popup_gen().lock().unwrap();
        *g += 1;
        *g
    };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        if *popup_gen().lock().unwrap() == gen {
            hide_popup(&app);
        }
    });
}

/// 弹窗卡片被点击：关弹窗 + 打开主窗口（tray-popup.html 的 invoke 入口）
#[tauri::command]
fn popup_clicked(app: AppHandle) {
    hide_popup(&app);
    show_main_window(&app);
}

/* ============================ 应用入口 ============================ */

fn main() {
    tauri::Builder::default()
        // 开机自启插件（Windows: 注册表 / Linux: .desktop / macOS: LaunchAgent）
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            set_tray_lang,
            set_tray_weather,
            popup_clicked
        ])
        .setup(|app| {
            // 初始菜单：英文兜底；前端启动检测到桌面环境后会带上用户
            // 记忆的语言立即调用 set_tray_lang 覆盖（见 app.js 桥接）。
            let show = MenuItem::with_id(app, MENU_SHOW, "Show", true, None::<&str>)?;
            let auto = CheckMenuItem::with_id(
                app,
                MENU_AUTOSTART,
                "Launch at login",
                true,
                app.autolaunch().is_enabled().unwrap_or(false),
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &auto, &sep, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Weather")
                .menu(&menu)
                // 左键单击托盘 = 显示主窗口；菜单只在右键时弹出
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    MENU_SHOW => show_main_window(app),
                    MENU_AUTOSTART => {
                        // 以插件当前状态为准取反，再把菜单勾选态强制同步
                        // （不依赖菜单系统是否已自动翻转，避免两边脱钩）
                        let autolaunch = app.autolaunch();
                        let enabled = autolaunch.is_enabled().unwrap_or(false);
                        let res = if enabled {
                            autolaunch.disable()
                        } else {
                            autolaunch.enable()
                        };
                        if let Err(e) = res {
                            eprintln!("切换开机自启失败: {e}");
                        }
                        // 取反后重建菜单（同时刷新勾选态）
                        let _ = rebuild_tray(app, &current_lang(app));
                    }
                    MENU_QUIT => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        hide_popup(app);
                        show_main_window(app);
                    }
                    // 悬停托盘图标 → 图标旁弹详情卡片；离开 → 延迟收起
                    TrayIconEvent::Enter { rect, .. } => {
                        show_popup(tray.app_handle(), rect.position, rect.size)
                    }
                    TrayIconEvent::Leave { .. } => schedule_popup_hide(tray.app_handle()),
                    _ => {}
                })
                .build(app)?;

            // 悬停详情弹窗：无边框 / 置顶 / 不进任务栏 / 不抢焦点，初始隐藏，
            // 内容页 tray-popup.html 与主窗口同源，直接读 localStorage('weather:tray')。
            WebviewWindowBuilder::new(
                app,
                "popup",
                WebviewUrl::App("tray-popup.html".into()),
            )
            .title("Weather")
            .inner_size(POPUP_W, POPUP_H)
            .visible(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .shadow(true)
            .build()?;

            // 慢机（VMware 软渲染）上 __TAURI__ 全局对象注入可能晚于前端脚本执行，
            // 前端的 retry 兜底不一定赶上。改为 Rust 侧轮询驱动：定期 eval 一段 JS
            // 从页面 localStorage 读语言/天气并调用对应命令。eval 走 WebKitGTK
            // 主 world（与页面共享 localStorage），已实测可靠。前端侧另有直接 invoke
            // 通道（setLang / 每次天气渲染），此处为低频兜底。
            {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    for _ in 0..600 {
                        std::thread::sleep(Duration::from_secs(2));
                        let Some(win) = app.get_webview_window("main") else { continue };
                        let _ = win.eval(
                            r#"try {
                              var i = window.__TAURI__ && window.__TAURI__.core;
                              if (i && i.invoke) {
                                var l = localStorage.getItem('weather:lang') || 'zh';
                                i.invoke('set_tray_lang', { lang: l });
                                var w = localStorage.getItem('weather:tray');
                                if (w) { i.invoke('set_tray_weather', { snapshot: JSON.parse(w) }); }
                              }
                            } catch (e) {}"#,
                        );
                    }
                });
            }

            // 调试钩子：TRAY_POPUP_TEST=1 时定时在固定位置显示弹窗（Linux 下
            // StatusNotifier 不传悬停事件，用它在本机验证弹窗渲染；正常启动不触发）。
            if std::env::var("TRAY_POPUP_TEST").is_ok() {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(4));
                    if let Some(win) = app.get_webview_window("popup") {
                        let _ = win.set_position(PhysicalPosition::new(160, 140));
                        popup_show_raw(&win);
                    }
                });
            }
            Ok(())
        })
        // 关闭窗口 = 隐藏到托盘（桌面常驻型天气应用的标准行为，退出走托盘菜单）
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
