#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    platform::windows::WindowBuilderExtWindows,
    window::WindowBuilder,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};
use wry::{http::Response, WebContext, WebViewBuilder};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const GITHUB_REPO: &str = "Frezze98/ng-helper";

const UI_HTML: &[u8] = include_bytes!("../assets/ui.html");
const RULES_JSON: &[u8] = include_bytes!("../assets/rules_index.json");
const AI_STUDIO_URL: &str = "https://aistudio.google.com/apikey";
const CREATE_NO_WINDOW: u32 = 0x08000000;
// Порт для проверки «уже запущено»: второй экземпляр просто будит первый.
const SINGLE_INSTANCE_ADDR: &str = "127.0.0.1:47631";

enum UserEvent {
    ToggleOverlay,
    ShowOverlay,
    HideOverlay,
    DragWindow,
    SetAlwaysOnTop(bool),
    Resize(f64, f64),
    UpdateError(String),
    Uninstall,
    Quit,
}

fn webview_data_dir() -> PathBuf {
    app_data_dir().join("webview")
}

fn app_data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("NGHelper")
}

/// Пишет сообщение в %LOCALAPPDATA%\NGHelper\error.log
fn log_error(msg: &str) {
    let dir = app_data_dir();
    let _ = fs::create_dir_all(&dir);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("error.log"))
    {
        let _ = writeln!(f, "[{ts}] v{APP_VERSION}: {msg}");
    }
}

fn install_panic_log() {
    std::panic::set_hook(Box::new(|info| {
        log_error(&format!("PANIC: {info}"));
    }));
}

fn load_tray_icon() -> Icon {
    // 1 — ординал иконки, вшитой через build.rs (winresource)
    Icon::from_resource(1, Some((32, 32))).unwrap_or_else(|_| make_tray_icon())
}

fn make_tray_icon() -> Icon {
    let size = 32usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let border = x < 3 || x >= size - 3 || y < 3 || y >= size - 3;
            let inner_dot = (x >= 10 && x < 22) && (y >= 10 && y < 22);
            let (r, g, b, a) = if border || inner_dot {
                (38, 255, 174, 255)
            } else {
                (11, 17, 17, 255)
            };
            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = a;
        }
    }
    Icon::from_rgba(rgba, size as u32, size as u32).expect("failed to create tray icon")
}

fn open_external_url(url: &str) {
    let allowed = url.starts_with("https://aistudio.google.com/")
        || url.starts_with(&format!("https://github.com/{}", GITHUB_REPO));
    if !allowed {
        return;
    }
    let _ = Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn run_uninstall() {
    let mut exe_dir = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };
    exe_dir.pop();
    let uninst = exe_dir.join("unins000.exe");
    if uninst.exists() {
        let _ = Command::new(&uninst).spawn();
        std::process::exit(0);
    }
}

fn run_self_update(url: String, proxy: EventLoopProxy<UserEvent>) {
    let allowed_prefix = format!("https://github.com/{}/releases/download/", GITHUB_REPO);
    if !url.starts_with(&allowed_prefix) || !url.ends_with(".exe") {
        let _ = proxy.send_event(UserEvent::UpdateError(
            "Некоректне посилання оновлення.".into(),
        ));
        return;
    }

    thread::spawn(move || {
        let temp_setup = std::env::temp_dir().join("Setup-NG-Helper.exe");

        let status = Command::new("curl.exe")
            .args([
                "-L",
                "-f",
                "-sS",
                "-o",
                temp_setup.to_string_lossy().as_ref(),
                &url,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => {
                let _ = fs::remove_file(&temp_setup);
                let _ = proxy.send_event(UserEvent::UpdateError(
                    "Не вдалося завантажити оновлення з GitHub.".into(),
                ));
                return;
            }
        }

        let launched = Command::new(&temp_setup)
            .args(["/SP-", "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"])
            .spawn();

        if launched.is_ok() {
            let _ = proxy.send_event(UserEvent::Quit);
        } else {
            let _ = fs::remove_file(&temp_setup);
            let _ = proxy.send_event(UserEvent::UpdateError(
                "Не вдалося запустити інсталятор оновлення.".into(),
            ));
        }
    });
}

fn main() {
    install_panic_log();

    // Единственный экземпляр: если приложение уже запущено (например, свёрнуто
    // в трей), будим его и выходим. Раньше второй запуск молча падал на
    // регистрации Alt+Q, и казалось, что программа «не запускается».
    let single_listener = match TcpListener::bind(SINGLE_INSTANCE_ADDR) {
        Ok(l) => Some(l),
        Err(_) => {
            if let Ok(mut s) = TcpStream::connect(SINGLE_INSTANCE_ADDR) {
                let _ = s.write_all(b"show");
                return;
            }
            None
        }
    };

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    if let Some(listener) = single_listener {
        let proxy_single = event_loop.create_proxy();
        thread::spawn(move || {
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    let mut buf = [0u8; 8];
                    let _ = s.read(&mut buf);
                    let _ = proxy_single.send_event(UserEvent::ShowOverlay);
                }
            }
        });
    }

    let window = WindowBuilder::new()
        .with_title("NG Helper")
        .with_inner_size(LogicalSize::new(560.0, 740.0))
        .with_decorations(false)
        .with_always_on_top(true)
        .with_skip_taskbar(true)
        .with_visible(true)
        .build(&event_loop)
        .expect("failed to build overlay window");

    let proxy_ipc = event_loop.create_proxy();

    let mut web_context = WebContext::new(Some(webview_data_dir()));
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_custom_protocol("ng".into(), |_webview_id, request| {
            let path = request.uri().path();
            let (mime, body): (&str, Cow<'static, [u8]>) = match path {
                "/rules_index.json" => (
                    "application/json; charset=utf-8",
                    Cow::Borrowed(RULES_JSON),
                ),
                "/app_info.json" => (
                    "application/json; charset=utf-8",
                    Cow::Owned(
                        format!(
                            r#"{{"version":"{}","repo":"{}"}}"#,
                            APP_VERSION, GITHUB_REPO
                        )
                        .into_bytes(),
                    ),
                ),
                _ => ("text/html; charset=utf-8", Cow::Borrowed(UI_HTML)),
            };
            Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .body(body)
                .unwrap()
        })
        .with_ipc_handler(move |req| {
            let msg = req.body().as_str();
            if msg == "open_key_url" {
                open_external_url(AI_STUDIO_URL);
            } else if let Some(url) = msg.strip_prefix("open_url:") {
                open_external_url(url);
            } else if let Some(url) = msg.strip_prefix("update:") {
                run_self_update(url.to_string(), proxy_ipc.clone());
            } else if msg == "drag" {
                let _ = proxy_ipc.send_event(UserEvent::DragWindow);
            } else if msg == "hide" {
                let _ = proxy_ipc.send_event(UserEvent::HideOverlay);
            } else if msg == "uninstall" {
                let _ = proxy_ipc.send_event(UserEvent::Uninstall);
            } else if msg == "quit" {
                let _ = proxy_ipc.send_event(UserEvent::Quit);
            } else if let Some(val) = msg.strip_prefix("aot:") {
                let _ = proxy_ipc.send_event(UserEvent::SetAlwaysOnTop(val == "true"));
            } else if let Some(dims) = msg.strip_prefix("resize:") {
                let parts: Vec<&str> = dims.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        let _ = proxy_ipc.send_event(UserEvent::Resize(w, h));
                    }
                }
            }
        })
        .with_url("http://ng.localhost/index.html")
        .build(&window)
        .expect("failed to build WebView2");

    let hotkey_manager = GlobalHotKeyManager::new().expect("failed to init hotkey manager");
    let toggle_hotkey = HotKey::new(Some(Modifiers::ALT), Code::KeyQ);
    let quit_hotkey = HotKey::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyQ);
    // Если Alt+Q занят другой программой — не падаем, работаем через трей.
    if let Err(e) = hotkey_manager.register(toggle_hotkey) {
        log_error(&format!("Alt+Q register failed: {e}"));
    }
    let _ = hotkey_manager.register(quit_hotkey);

    let proxy_hk = event_loop.create_proxy();
    let toggle_hk_id = toggle_hotkey.id();
    let quit_hk_id = quit_hotkey.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            if event.id == toggle_hk_id {
                let _ = proxy_hk.send_event(UserEvent::ToggleOverlay);
            } else if event.id == quit_hk_id {
                let _ = proxy_hk.send_event(UserEvent::Quit);
            }
        }
    }));

    let item_toggle = MenuItem::new("Відкрити / Сховати (Alt+Q)", true, None);
    let item_quit = MenuItem::new("Вихід (Alt+Shift+Q)", true, None);

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&item_toggle);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let _ = tray_menu.append(&item_quit);

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(format!("NG Helper v{} (Alt+Q)", APP_VERSION))
        .with_icon(load_tray_icon())
        .with_menu_on_left_click(false)
        .build()
        .expect("failed to build tray icon");

    let toggle_id = item_toggle.id().clone();
    let quit_id = item_quit.id().clone();

    let proxy_menu = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == toggle_id {
            let _ = proxy_menu.send_event(UserEvent::ToggleOverlay);
        } else if event.id == quit_id {
            let _ = proxy_menu.send_event(UserEvent::Quit);
        }
    }));

    let proxy_tray = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let _ = proxy_tray.send_event(UserEvent::ToggleOverlay);
        }
    }));

    let mut is_visible = true;
    let mut always_on_top = true;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let _ = (&hotkey_manager, &_tray_icon);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window.set_visible(false);
                is_visible = false;
            }
            Event::UserEvent(UserEvent::ToggleOverlay) => {
                is_visible = !is_visible;
                window.set_visible(is_visible);
                if is_visible {
                    window.set_always_on_top(false);
                    window.set_always_on_top(always_on_top);
                    let _ = window.set_focus();
                    window.request_redraw();
                }
            }
            Event::UserEvent(UserEvent::ShowOverlay) => {
                is_visible = true;
                window.set_visible(true);
                window.set_always_on_top(false);
                window.set_always_on_top(always_on_top);
                let _ = window.set_focus();
                window.request_redraw();
            }
            Event::UserEvent(UserEvent::HideOverlay) => {
                window.set_visible(false);
                is_visible = false;
            }
            Event::UserEvent(UserEvent::DragWindow) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::SetAlwaysOnTop(val)) => {
                always_on_top = val;
                window.set_always_on_top(val);
            }
            Event::UserEvent(UserEvent::Resize(w, h)) => {
                window.set_inner_size(LogicalSize::new(w, h));
            }
            Event::UserEvent(UserEvent::UpdateError(msg)) => {
                let safe = msg.replace('\\', "\\\\").replace('"', "\\\"");
                let _ = webview.evaluate_script(&format!(
                    "window.onUpdateError && window.onUpdateError(\"{safe}\");"
                ));
            }
            Event::UserEvent(UserEvent::Uninstall) => {
                run_uninstall();
            }
            Event::UserEvent(UserEvent::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}