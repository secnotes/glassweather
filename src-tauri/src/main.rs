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
    AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
#[cfg(not(windows))]
use tauri::{Position as DpiPosition, Size as DpiSize};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

const MENU_SHOW: &str = "show";
const MENU_AUTOSTART: &str = "autostart";
const MENU_QUIT: &str = "quit";
#[cfg(windows)]
const MENU_BAR: &str = "bar";
/// 任务栏天气（Win11 deskband）菜单项
#[cfg(windows)]
const MENU_DESKBAND: &str = "deskband";
#[cfg(windows)]
const DESKBAND_LABEL: &str = "deskband";
/// 任务栏天气位置子菜单：取值即 Deskband11Lib 的 TaskbarContentPlacement
#[cfg(windows)]
const MENU_PLACE: &str = "place";
#[cfg(windows)]
const PLACE_AUTO: &str = "place-auto";
#[cfg(windows)]
const PLACE_LEFT: &str = "place-left";
#[cfg(windows)]
const PLACE_BEFORE_NOTIF: &str = "place-before-notif";
#[cfg(windows)]
const PLACE_BEFORE_START: &str = "place-before-start";

/// 任务栏天气位置（持久化于 deskband-place.cfg，值为上述 MENU_PLACE_* id）
#[cfg(windows)]
static DESKBAND_PLACE: OnceLock<Mutex<String>> = OnceLock::new();
#[cfg(windows)]
fn deskband_place_store() -> &'static Mutex<String> {
    DESKBAND_PLACE.get_or_init(|| Mutex::new(PLACE_AUTO.into()))
}
#[cfg(windows)]
fn deskband_place() -> String {
    deskband_place_store().lock().unwrap().clone()
}

#[cfg(windows)]
fn placement_from_id(id: &str) -> Option<tauri_plugin_deskband11lib::TaskbarContentPlacement> {
    use tauri_plugin_deskband11lib::TaskbarContentPlacement;
    match id {
        PLACE_AUTO => Some(TaskbarContentPlacement::Auto),
        PLACE_LEFT => Some(TaskbarContentPlacement::LeftEdge),
        PLACE_BEFORE_NOTIF => Some(TaskbarContentPlacement::BeforeNotificationArea),
        PLACE_BEFORE_START => Some(TaskbarContentPlacement::BeforeStartButton),
        _ => None,
    }
}

#[cfg(windows)]
fn load_place_setting(app: &AppHandle) -> String {
    let id = cfg_path(app, "deskband-place.cfg")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| placement_from_id(s).is_some())
        .unwrap_or_else(|| PLACE_AUTO.into());
    *deskband_place_store().lock().unwrap() = id.clone();
    id
}

#[cfg(windows)]
fn save_place_setting(app: &AppHandle, id: &str) {
    if let Some(p) = cfg_path(app, "deskband-place.cfg") {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, id);
    }
}

/// 应用版本（来自 Cargo.toml，发版时只需改 Cargo.toml 的 version 字段）
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_SITE: &str = "https://github.com/secnotes/glassweather";

const POPUP_W: f64 = 320.0;
const POPUP_H: f64 = 224.0;

/* ============================ 语言 / 托盘菜单 ============================ */

/// 托盘菜单与窗口标题文案（跟随前端语言，前端初始化/切换语言时会调用 set_tray_lang）。
/// 字段增多后由 tuple 重构为 struct，便于增删条目。
struct TrayTexts {
    show: String,
    autostart: String,
    /// 悬浮天气条（Win：任务栏上方的常驻迷你小窗）
    floating_bar: String,
    /// 任务栏天气（Win11 deskband：真正嵌进任务栏内部）
    deskband: String,
    /// 任务栏天气位置子菜单标题
    deskband_place: String,
    /// 位置选项：自动 / 开始按钮旁 / 远端(Widgets 后) / 通知区左侧
    place_auto: String,
    place_left: String,
    place_before_notif: String,
    place_before_start: String,
    quit: String,
    about: String,
    default_tooltip: String,
    title: String,
    about_comments: String,
}

fn tray_texts(lang: &str) -> TrayTexts {
    match lang {
        "zh" => TrayTexts {
            show: "显示主窗口".into(),
            autostart: "开机自启".into(),
            floating_bar: "悬浮天气条".into(),
            deskband: "任务栏天气".into(),
            deskband_place: "任务栏天气位置".into(),
            place_auto: "自动".into(),
            place_left: "远端（Widgets 后）".into(),
            place_before_notif: "通知区左侧".into(),
            place_before_start: "开始按钮旁".into(),
            quit: "退出".into(),
            about: "关于".into(),
            default_tooltip: "天气查询".into(),
            title: "天气查询".into(),
            about_comments: "查询任意城市实时天气与未来 7 天预报，并展示今天这个日期在过去 \
                 30 年的历史天气。数据来自 Open-Meteo。网页版与桌面版共用一套前端。"
                .into(),
        },
        _ => TrayTexts {
            show: "Show".into(),
            autostart: "Launch at login".into(),
            floating_bar: "Floating weather strip".into(),
            deskband: "Taskbar weather".into(),
            deskband_place: "Taskbar weather position".into(),
            place_auto: "Auto".into(),
            place_left: "Far end (after Widgets)".into(),
            place_before_notif: "Before notification area".into(),
            place_before_start: "Next to Start button".into(),
            quit: "Quit".into(),
            about: "About".into(),
            default_tooltip: "GlassWeather".into(),
            title: "GlassWeather".into(),
            about_comments: "Current weather and a 7-day forecast for any city, plus \
                 30 years of on-this-day history. Data from Open-Meteo. \
                 The website and desktop app share one frontend."
                .into(),
        },
    }
}

/// 按语言构建托盘菜单并挂到托盘上。语言切换/自启勾选翻转时调用。
/// tooltip 优先显示天气快照（城市 · 天气 · 温度），无天气数据时用语言默认文案。
fn rebuild_tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let t = tray_texts(lang);

    let show = MenuItem::with_id(app, MENU_SHOW, t.show, true, None::<&str>)?;
    let auto = CheckMenuItem::with_id(
        app,
        MENU_AUTOSTART,
        t.autostart,
        true,
        app.autolaunch().is_enabled().unwrap_or(false),
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, t.quit, true, None::<&str>)?;
    // 关于：PredefinedMenuItem 原生实现（Windows 弹 MessageBox，Linux 弹 GTK
    // About 对话框），自动显示 name/version/authors/license/website/comments。
    let about = PredefinedMenuItem::about(
        app,
        Some(&t.about),
        Some(tauri::menu::AboutMetadataBuilder::new()
            .name(Some("GlassWeather"))
            .version(Some(APP_VERSION))
            .authors(Some(vec!["secnotes".into()]))
            .license(Some("MIT"))
            .website(Some(APP_SITE))
            .website_label(Some("GitHub"))
            .comments(Some(t.about_comments))
            .build()),
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    #[cfg(windows)]
    let menu = {
        use tauri::menu::Submenu;
        let bar = CheckMenuItem::with_id(
            app,
            MENU_BAR,
            t.floating_bar,
            true,
            bar_enabled(),
            None::<&str>,
        )?;
        let deskband = CheckMenuItem::with_id(
            app,
            MENU_DESKBAND,
            t.deskband,
            true,
            deskband_running(),
            None::<&str>,
        )?;
        // 位置子菜单：4 个互斥单选位（勾选态 = 持久化值）
        let current_place = deskband_place();
        let auto_place = CheckMenuItem::with_id(
            app, PLACE_AUTO, t.place_auto, true, current_place == PLACE_AUTO, None::<&str>,
        )?;
        let left = CheckMenuItem::with_id(
            app, PLACE_LEFT, t.place_left, true, current_place == PLACE_LEFT, None::<&str>,
        )?;
        let before_notif = CheckMenuItem::with_id(
            app,
            PLACE_BEFORE_NOTIF,
            t.place_before_notif,
            true,
            current_place == PLACE_BEFORE_NOTIF,
            None::<&str>,
        )?;
        let before_start = CheckMenuItem::with_id(
            app,
            PLACE_BEFORE_START,
            t.place_before_start,
            true,
            current_place == PLACE_BEFORE_START,
            None::<&str>,
        )?;
        let place_menu = Submenu::with_id(
            app,
            MENU_PLACE,
            t.deskband_place,
            true,
        )?;
        place_menu.append(&auto_place)?;
        place_menu.append(&left)?;
        place_menu.append(&before_notif)?;
        place_menu.append(&before_start)?;
        Menu::with_items(
            app,
            &[&show, &auto, &bar, &deskband, &place_menu, &about, &sep, &quit],
        )?
    };
    #[cfg(not(windows))]
    let menu = Menu::with_items(app, &[&show, &auto, &about, &sep, &quit])?;

    // 窗口标题同样跟随语言（配置里的初始标题是英文兜底，前端启动后即被覆盖）
    if let Some(win) = app.get_webview_window("main") {
        win.set_title(&t.title)?;
    }
    let tray = app.tray_by_id("main").expect("tray must be built in setup");
    tray.set_menu(Some(menu))?;
    let tip = weather_store()
        .lock()
        .unwrap()
        .as_ref()
        .map(|s| s.tooltip.clone())
        .unwrap_or(t.default_tooltip);
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

#[cfg(not(windows))]
static POPUP_GEN: OnceLock<Mutex<u64>> = OnceLock::new();
#[cfg(not(windows))]
fn popup_gen() -> &'static Mutex<u64> {
    POPUP_GEN.get_or_init(|| Mutex::new(0))
}

/* ============================ Windows 悬浮天气条 / 任务栏天气 ============================ */

/* 悬浮天气条：任务栏正上方的屏幕角落常驻一个迷你天气条（无边框/置顶/不进
   任务栏的透明 webview 小窗，显示图标 + 温度）。悬停它弹出完整详情卡片
   （光标轮询驱动，150ms），点击打开主窗口；托盘图标则只保留系统原生
   tooltip（一句话天气），避免卡片与 tooltip 双重提示。
   任务栏天气（deskband）：tauri-plugin-deskband11lib 把 webview 小窗真正挂进
   Win11 任务栏内部空余区（任务栏按钮与通知区之间），跟随任务栏移动/换边，
   Explorer 重启自动重建；仅 Win11 可用，Win10 上启动失败只打日志。
   tray-icon 0.25 的 Leave 检测有缺陷：15ms 定时器用 last_position.take() 一次
   性取值，鼠标在图标上静止超一个周期后移出时 Leave 永不触发。因此 Windows
   上弹窗的显示与隐藏完全改用光标轮询判定，不再依赖托盘 Enter/Leave 事件。 */
#[cfg(windows)]
const BAR_W: f64 = 148.0;
#[cfg(windows)]
const BAR_H: f64 = 44.0;

/// 悬浮天气条开关（托盘菜单「悬浮天气条」勾选项），持久化于应用配置目录
#[cfg(windows)]
static BAR_ENABLED: OnceLock<Mutex<bool>> = OnceLock::new();
#[cfg(windows)]
fn bar_enabled_store() -> &'static Mutex<bool> {
    BAR_ENABLED.get_or_init(|| Mutex::new(true))
}
#[cfg(windows)]
fn bar_enabled() -> bool {
    *bar_enabled_store().lock().unwrap()
}

#[cfg(windows)]
fn cfg_path(app: &AppHandle, name: &str) -> Option<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(name))
}

/// 启动时读入持久化开关（无配置文件时默认开启）
#[cfg(windows)]
fn load_bar_setting(app: &AppHandle) -> bool {
    let on = cfg_path(app, "bar.cfg")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "on")
        .unwrap_or(true);
    *bar_enabled_store().lock().unwrap() = on;
    on
}

#[cfg(windows)]
fn save_bar_setting(app: &AppHandle, on: bool) {
    if let Some(p) = cfg_path(app, "bar.cfg") {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, if on { "on" } else { "off" });
    }
}

/// 显示/隐藏任务栏天气条（关闭时连带收起详情弹窗）。显示/隐藏都走原生
/// SW_SHOWNOACTIVATE / SW_HIDE，与 poll_bar_hover 的路径对称。
#[cfg(windows)]
fn set_bar_visible(app: &AppHandle, on: bool) {
    let Some(bar) = app.get_webview_window("bar") else { return };
    if on {
        place_bar(app);
        popup_show_raw(&bar);
    } else {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Ok(hwnd) = bar.hwnd() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
        } else {
            let _ = bar.hide();
        }
        hide_popup(app);
    }
}

/// 把天气条放到主显示器工作区（已扣除任务栏）的右下角
#[cfg(windows)]
fn place_bar(app: &AppHandle) {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };

    let Some(bar) = app.get_webview_window("bar") else { return };
    let sf = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let (bw, bh) = ((BAR_W * sf) as i32, (BAR_H * sf) as i32);
    unsafe {
        let mut wa = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut wa as *mut _ as _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            let x = wa.right - bw - 10;
            let y = wa.bottom - bh - 4;
            let _ = bar.set_position(PhysicalPosition::new(x, y));
        }
    }
}

/// 在天气条正上方（右对齐）显示详情弹窗
#[cfg(windows)]
fn show_popup_above_bar(app: &AppHandle, bar_rect: &windows::Win32::Foundation::RECT) {
    let Some(win) = app.get_webview_window("popup") else { return };
    let sf = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let (pw, ph) = ((POPUP_W * sf) as i32, (POPUP_H * sf) as i32);
    let x = bar_rect.right - pw;
    let y = bar_rect.top - ph - 6;
    let _ = win.set_position(PhysicalPosition::new(x, y));
    popup_show_raw(&win);
}

/// 光标轮询状态机（每 150ms），覆盖两个天气入口：
/// 悬停悬浮天气条（外扩 6px）→ 弹窗显示在其正上方；
/// 悬停任务栏天气 deskband（外扩 2px，容差小以免波及任务栏邻居）→ 弹窗显示在
/// 其贴边一侧（任务栏在底部→上方，顶部→下方，左侧→右侧，右侧→左侧）；
/// 离开两者且弹窗可见时，光标也不在弹窗上（外扩 10px 盖缝隙）→ 隐藏弹窗；
/// 光标停在弹窗上 → 保持显示（用户在读卡片）。
#[cfg(windows)]
fn poll_hover(app: &AppHandle) {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowRect, IsWindowVisible,
    };

    let Some(popup) = app.get_webview_window("popup") else { return };
    let Ok(popup_hwnd) = popup.hwnd() else { return };

    unsafe {
        let mut cursor = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut cursor).is_err() {
            return;
        }

        // 入口一：悬浮天气条（可见时才参与判定）
        if let Some(bar) = app.get_webview_window("bar") {
            if let Ok(bar_hwnd) = bar.hwnd() {
                if IsWindowVisible(bar_hwnd).as_bool() {
                    let mut br = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if GetWindowRect(bar_hwnd, &mut br).is_ok()
                        && cursor.x >= br.left - 6 && cursor.x <= br.right + 6
                        && cursor.y >= br.top - 6 && cursor.y <= br.bottom + 6
                    {
                        if !IsWindowVisible(popup_hwnd).as_bool() {
                            show_popup_above_bar(app, &br);
                        }
                        return;
                    }
                }
            }
        }

        // 入口二：任务栏天气（deskband）
        if let Some(db) = app.get_webview_window(DESKBAND_LABEL) {
            if let Ok(db_hwnd) = db.hwnd() {
                if IsWindowVisible(db_hwnd).as_bool() {
                    let mut dr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    if GetWindowRect(db_hwnd, &mut dr).is_ok()
                        && cursor.x >= dr.left - 2 && cursor.x <= dr.right + 2
                        && cursor.y >= dr.top - 2 && cursor.y <= dr.bottom + 2
                    {
                        if !IsWindowVisible(popup_hwnd).as_bool() {
                            show_popup_beside_deskband(app, &dr);
                        }
                        return;
                    }
                }
            }
        }

        // 两个入口都不在：弹窗不可见就无事可做
        if !IsWindowVisible(popup_hwnd).as_bool() {
            return;
        }
        // 光标在弹窗上（外扩 10px）→ 保持
        let mut pr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if GetWindowRect(popup_hwnd, &mut pr).is_ok()
            && cursor.x >= pr.left - 10 && cursor.x <= pr.right + 10
            && cursor.y >= pr.top - 10 && cursor.y <= pr.bottom + 10
        {
            return;
        }
    }
    hide_popup(app);
}

/// 弹窗显示在任务栏天气（deskband）旁边：按任务栏所在屏幕边缘选择方向，
/// 底部 → 上方，顶部 → 下方，左侧 → 右侧，右侧 → 左侧，并对屏幕内收 6px。
#[cfg(windows)]
fn show_popup_beside_deskband(app: &AppHandle, db_rect: &windows::Win32::Foundation::RECT) {
    let Some(win) = app.get_webview_window("popup") else { return };
    let sf = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    let (pw, ph) = ((POPUP_W * sf) as i32, (POPUP_H * sf) as i32);

    let cx = (db_rect.left + db_rect.right) as f64 / 2.0;
    let cy = (db_rect.top + db_rect.bottom) as f64 / 2.0;
    let mon = app
        .monitor_from_point(cx, cy)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());

    let (x, y) = if let Some(m) = mon {
        let mp = m.position();
        let ms = m.size();
        let (mx, my, mw, mh) = (mp.x, mp.y, ms.width as i32, ms.height as i32);
        // deskband 在任务栏里，距离哪条屏幕边最近即任务栏所在边
        let d_bottom = my + mh - db_rect.bottom;
        let d_top = db_rect.top - my;
        let d_left = db_rect.left - mx;
        let d_right = mx + mw - db_rect.right;
        let (x, y) = if d_bottom <= d_top && d_bottom <= d_left && d_bottom <= d_right {
            // 底部任务栏：弹窗在 deskband 上方
            let x = (db_rect.left + db_rect.right) / 2 - pw / 2;
            (x, db_rect.top - ph - 6)
        } else if d_top <= d_left && d_top <= d_right {
            // 顶部任务栏：弹窗在 deskband 下方
            let x = (db_rect.left + db_rect.right) / 2 - pw / 2;
            (x, db_rect.bottom + 6)
        } else if d_left <= d_right {
            // 左侧任务栏：弹窗在 deskband 右侧
            let y = (db_rect.top + db_rect.bottom) / 2 - ph / 2;
            (db_rect.right + 6, y)
        } else {
            // 右侧任务栏：弹窗在 deskband 左侧
            let y = (db_rect.top + db_rect.bottom) / 2 - ph / 2;
            (db_rect.left - pw - 6, y)
        };
        // 屏幕内收 6px 收边
        (
            x.clamp(mx + 6, mx + mw - pw - 6),
            y.clamp(my + 6, my + mh - ph - 6),
        )
    } else {
        // 无显示器信息时兜底：deskband 上方
        (
            (db_rect.left + db_rect.right) / 2 - pw / 2,
            db_rect.top - ph - 6,
        )
    };

    let _ = win.set_position(PhysicalPosition::new(x, y));
    popup_show_raw(&win);
}

/* ============================ Windows 11 任务栏天气（deskband） ============================ */

/* tauri-plugin-deskband11lib：把 webview 小窗挂进 Win11 任务栏空余区。
   控制器持有"窗口工厂"闭包，任务栏布局变化 / Explorer 重启 / DPI 变更时按需
   重建窗口；因此窗口内不放状态（页面自身从 localStorage 读快照渲染）。
   开关持久化于 deskband.cfg（与悬浮条的 bar.cfg 同一目录、同 on/off 格式）。 */

#[cfg(windows)]
type DeskbandHandle = tauri_plugin_deskband11lib::TaskbarWindowHandle<tauri::Wry>;

// TaskbarWindowHandle 非 Send（内含 event loop 弱引用），不能进全局 static。
// 所有启停调用点（setup、托盘菜单事件）都在 Tauri 主线程，用 thread_local 即可。
#[cfg(windows)]
thread_local! {
    static DESKBAND_HANDLE: std::cell::RefCell<Option<DeskbandHandle>> =
        const { std::cell::RefCell::new(None) };
}
#[cfg(windows)]
fn deskband_running() -> bool {
    DESKBAND_HANDLE.with(|h| h.borrow().is_some())
}

/// 任务栏天气窗口工厂：紧凑单行（图标 + 温度 + 今日范围），页面 tray-deskband.html
#[cfg(windows)]
fn create_deskband_window(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    tauri_plugin_deskband11lib::taskbar_window_builder(
        app,
        DESKBAND_LABEL,
        WebviewUrl::App("tray-deskband.html".into()),
    )
    .inner_size(150.0, 48.0)
    .transparent(true)
    .build()
}

/// 启动任务栏天气（placement 取用户持久化的选择；Win10 上启动失败仅打日志）
#[cfg(windows)]
fn start_deskband(app: &AppHandle) -> bool {
    if deskband_running() {
        return true;
    }
    let placement = placement_from_id(&deskband_place())
        .unwrap_or(tauri_plugin_deskband11lib::TaskbarContentPlacement::Auto);
    match tauri_plugin_deskband11lib::TaskbarWindowController::new(
        app,
        create_deskband_window,
    )
    .options(tauri_plugin_deskband11lib::TaskbarContentHostOptions {
        preferred_width: Some(150.0),
        placement: Some(placement),
        ..Default::default()
    })
    .start()
    {
        Ok(handle) => {
            DESKBAND_HANDLE.with(|h| *h.borrow_mut() = Some(handle));
            // 重建托盘菜单：勾选态从"启动前(false)"同步为实态。首启时初始
            // 菜单写死 false，启动成功后前端 set_tray_lang 的重建时机不可靠
            // （可能早于启动完成，或语言未变被短路跳过），这里主动刷新。
            let _ = rebuild_tray(app, &current_lang(app));
            true
        }
        Err(e) => {
            eprintln!("任务栏天气启动失败（需 Windows 11）: {e}");
            false
        }
    }
}

#[cfg(windows)]
fn stop_deskband() {
    DESKBAND_HANDLE.with(|h| {
        if let Some(handle) = h.borrow_mut().take() {
            handle.stop();
        }
    });
}

/// 启动时读入任务栏天气开关（无配置文件时默认开启——Win11 新旗舰位）
#[cfg(windows)]
fn load_deskband_setting(app: &AppHandle) -> bool {
    cfg_path(app, "deskband.cfg")
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim() == "on")
        .unwrap_or(true)
}

#[cfg(windows)]
fn save_deskband_setting(app: &AppHandle, on: bool) {
    if let Some(p) = cfg_path(app, "deskband.cfg") {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&p, if on { "on" } else { "off" });
    }
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

/* Windows 11 系统级窗口圆角（与任务栏/开始菜单同款效果）。WebView2 透明
   背景在 Tauri 下不可靠（CSS 圆角会被方形窗口裁掉），改用 DWM 圆角。
   Win10 无此属性时静默失败，窗口保持直角，不影响功能。 */
#[cfg(windows)]
fn apply_rounded_corners(win: &WebviewWindow) {
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DWM_WINDOW_CORNER_PREFERENCE,
    };
    if let Ok(hwnd) = win.hwnd() {
        let pref = DWMWCP_ROUND;
        unsafe {
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &pref as *const _ as _,
                std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
            );
        }
    }
}

/// 依据托盘图标矩形定位并显示弹窗：底部任务栏显示在图标上方，顶部任务栏显示在下方。
/// 托盘事件给的是 dpi::Position/Size（可能是逻辑坐标），先统一换物理坐标。
/// （仅非 Windows：Windows 上弹窗由任务栏天气条悬停驱动，托盘图标只出原生 tooltip。）
#[cfg(not(windows))]
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

/* Windows 上弹窗的显示走原生 SW_SHOWNOACTIVATE（绕过框架），框架内部的
   可见状态仍记为"隐藏"，其 hide() 会误判无事可做直接跳过——弹窗因此永远
   关不掉。隐藏同样走原生 SW_HIDE，显示/隐藏对称，不经过框架状态跟踪。 */
fn hide_popup(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("popup") {
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            if let Ok(hwnd) = win.hwnd() {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
                return;
            }
        }
        let _ = win.hide();
    }
}

/// 鼠标离开托盘图标后延迟隐藏（经过图标与弹窗之间缝隙、短暂划过都不会闪）；
/// 期间再次 Enter 会令本次计划作废。（仅非 Windows。）
#[cfg(not(windows))]
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
            #[cfg(windows)]
            let bar_on = load_bar_setting(&app.handle().clone());
            // 任务栏天气启动意图：首次默认开启，也用于初始菜单勾选态（而非写死
            // false——首启时 start_deskband 尚未执行，勾选与实际状态脱节）
            #[cfg(windows)]
            let deskband_on = load_deskband_setting(&app.handle().clone());
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
            let about = PredefinedMenuItem::about(
                app,
                Some("About"),
                Some(tauri::menu::AboutMetadataBuilder::new()
                    .name(Some("GlassWeather"))
                    .version(Some(APP_VERSION))
                    .authors(Some(vec!["secnotes".into()]))
                    .license(Some("MIT"))
                    .website(Some(APP_SITE))
                    .website_label(Some("GitHub"))
                    .comments(Some(
                        "Current weather and a 7-day forecast for any city, plus \
                         30 years of on-this-day history. Data from Open-Meteo.",
                    ))
                    .build()),
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            #[cfg(windows)]
            let menu = {
                use tauri::menu::Submenu;
                let bar = CheckMenuItem::with_id(
                    app,
                    MENU_BAR,
                    "Floating weather strip",
                    true,
                    bar_on,
                    None::<&str>,
                )?;
                let deskband = CheckMenuItem::with_id(
                    app,
                    MENU_DESKBAND,
                    "Taskbar weather",
                    true,
                    deskband_on, // 启动意图；启动后 start_deskband 会按实态重建菜单
                    None::<&str>,
                )?;
                // 位置子菜单（英文兜底；set_tray_lang 会带持久化值重建勾选态）
                let current_place = load_place_setting(&app.handle().clone());
                let p_auto = CheckMenuItem::with_id(
                    app, PLACE_AUTO, "Auto", true, current_place == PLACE_AUTO, None::<&str>,
                )?;
                let p_left = CheckMenuItem::with_id(
                    app,
                    PLACE_LEFT,
                    "Far end (after Widgets)",
                    true,
                    current_place == PLACE_LEFT,
                    None::<&str>,
                )?;
                let p_notif = CheckMenuItem::with_id(
                    app,
                    PLACE_BEFORE_NOTIF,
                    "Before notification area",
                    true,
                    current_place == PLACE_BEFORE_NOTIF,
                    None::<&str>,
                )?;
                let p_start = CheckMenuItem::with_id(
                    app,
                    PLACE_BEFORE_START,
                    "Next to Start button",
                    true,
                    current_place == PLACE_BEFORE_START,
                    None::<&str>,
                )?;
                let place_menu = Submenu::with_id(
                    app,
                    MENU_PLACE,
                    "Taskbar weather position",
                    true,
                )?;
                place_menu.append(&p_auto)?;
                place_menu.append(&p_left)?;
                place_menu.append(&p_notif)?;
                place_menu.append(&p_start)?;
                Menu::with_items(
                    app,
                    &[&show, &auto, &bar, &deskband, &place_menu, &about, &sep, &quit],
                )?
            };
            #[cfg(not(windows))]
            let menu = Menu::with_items(app, &[&show, &auto, &about, &sep, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("GlassWeather")
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
                    #[cfg(windows)]
                    MENU_BAR => {
                        // 悬浮天气条开关：翻转 + 持久化 + 立即生效（关则连带收起弹窗）
                        let on = !bar_enabled();
                        *bar_enabled_store().lock().unwrap() = on;
                        save_bar_setting(app, on);
                        set_bar_visible(app, on);
                        let _ = rebuild_tray(app, &current_lang(app));
                    }
                    #[cfg(windows)]
                    MENU_DESKBAND => {
                        // 任务栏天气开关：翻转 + 持久化 + 立即启停（Win11 deskband）
                        let on = !deskband_running();
                        if on {
                            if !start_deskband(app) {
                                // 启动失败（如 Win10）保持关态
                                save_deskband_setting(app, false);
                            } else {
                                save_deskband_setting(app, true);
                            }
                        } else {
                            stop_deskband();
                            hide_popup(app);
                            save_deskband_setting(app, false);
                        }
                        let _ = rebuild_tray(app, &current_lang(app));
                    }
                    // 任务栏天气位置：切换持久化值；正在运行则停后按新位置重启。
                    // 注意：handle.stop() 里窗口销毁（window.destroy）是经 Tauri
                    // 主线程异步执行的，同一 label "deskband" 的窗口尚未销毁完就
                    // 同步重建会撞 label 冲突使 start 失败（天气消失、勾选丢失）。
                    // 因此重启延后约 600ms，并切回主线程执行（controller 与窗口
                    // 创建都要求在主线程）。
                    #[cfg(windows)]
                    id if id == PLACE_AUTO
                        || id == PLACE_LEFT
                        || id == PLACE_BEFORE_NOTIF
                        || id == PLACE_BEFORE_START =>
                    {
                        if deskband_place() != *id {
                            let restarting = deskband_running();
                            *deskband_place_store().lock().unwrap() = id.to_string();
                            save_place_setting(app, id);
                            if restarting {
                                stop_deskband();
                                let app = app.clone();
                                std::thread::spawn(move || {
                                    std::thread::sleep(Duration::from_millis(600));
                                    let app2 = app.clone();
                                    let _ = app.run_on_main_thread(move || {
                                        start_deskband(&app2);
                                    });
                                });
                            }
                        }
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
                    // （仅非 Windows；Windows 上托盘图标只出原生 tooltip，
                    //  详情卡片由悬浮天气条 / 任务栏天气悬停触发，见 poll_hover）
                    #[cfg(not(windows))]
                    TrayIconEvent::Enter { rect, .. } => {
                        show_popup(tray.app_handle(), rect.position, rect.size)
                    }
                    #[cfg(not(windows))]
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
            .title("GlassWeather")
            .inner_size(POPUP_W, POPUP_H)
            .visible(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .shadow(true)
            .build()?;
            #[cfg(windows)]
            if let Some(popup_win) = app.get_webview_window("popup") {
                apply_rounded_corners(&popup_win);
            }

            // Windows 悬浮天气条：常驻迷你小窗（图标+温度），悬停弹详情卡片，
            // 点击打开主窗口（页面 tray-bar.html，读同一份 localStorage 快照）。
            #[cfg(windows)]
            {
                let bar = WebviewWindowBuilder::new(
                    app,
                    "bar",
                    WebviewUrl::App("tray-bar.html".into()),
                )
                .title("GlassWeather")
                .inner_size(BAR_W, BAR_H)
                .visible(false)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(false)
                .focused(false)
                .shadow(false)
                .build()?;
                let app_handle = app.handle().clone();
                apply_rounded_corners(&bar);
                if bar_on {
                    place_bar(&app_handle);
                    popup_show_raw(&bar);
                }

                // Windows 11 任务栏天气（deskband）：webview 小窗挂进任务栏内部
                // 空余区，跟随任务栏布局；悬停弹详情卡片（同下方轮询）。
                if load_deskband_setting(&app_handle) {
                    start_deskband(&app_handle);
                }

                // 悬停判定 + 弹窗显示/隐藏：光标轮询驱动（150ms，纯 Win32 调用，
                // 不碰 Tauri 异步 API，规避后台线程限制），覆盖悬浮条与 deskband。
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_millis(150));
                    poll_hover(&app_handle);
                });
            }

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
        // 关闭主窗口 = 隐藏到托盘（桌面常驻型天气应用的标准行为，退出走托盘菜单）。
        // 仅作用于主窗口：任务栏 deskband 窗口会被任务栏按需销毁/重建（Explorer
        // 重启、DPI 变更），prevent_close 会卡住该生命周期。
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
