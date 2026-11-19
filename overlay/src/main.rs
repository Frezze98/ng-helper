#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::fs;
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

enum UserEvent {
    ToggleOverlay,
    HideOverlay,
    DragWindow,
    SetAlwaysOnTop(bool),
    Resize(f64, f64),
    UpdateError(String),
    Quit,
}

fn webview_data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("NGHelper").join("webview")
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

fn run_self_update(url: String, proxy: EventLoopProxy<UserEvent>) {
    let allowed_prefix = format!("https://github.com/{}/releases/download/", GITHUB_REPO);
    if !url.starts_with(&allowed_prefix) || !url.ends_with(".exe") {
        let _ = proxy.send_event(UserEvent::UpdateError(
            "Некорректная ссылка обновления.".into(),
        ));
        return;
    }

    thread::spawn(move || {
        let cur_exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(e) => {
                let _ = proxy.send_event(UserEvent::UpdateError(format!(
                    "Не удалось определить путь к exe: {e}"
                )));
                return;
            }
        };

        let mut new_os = cur_exe.as_os_str().to_os_string();
        new_os.push(".new");
        let new_exe = PathBuf::from(new_os);

        let status = Command::new("curl.exe")
            .args([
                "-L",
                "-f",
                "-sS",
                "-o",
                new_exe.to_string_lossy().as_ref(),
                &url,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        match status {
            Ok(s) if s.success() => {}
            _ => {
                let _ = fs::remove_file(&new_exe);
                let _ = proxy.send_event(UserEvent::UpdateError(
                    "Не удалось скачать обновление с GitHub.".into(),
                ));
                return;
            }
        }

        let size_ok = fs::metadata(&new_exe)
            .map(|m| m.len() > 100_000)
            .unwrap_or(false);
        if !size_ok {
            let _ = fs::remove_file(&new_exe);
            let _ = proxy.send_event(UserEvent::UpdateError(
                "Скачанный файл повреждён или пустой.".into(),
            ));
            return;
        }

        let bat_path = std::env::temp_dir().join("ng_helper_updater.bat");
        let cur_str = cur_exe.to_string_lossy();
        let new_str = new_exe.to_string_lossy();

        let bat_script = format!(
            "@echo off\r\n\
             chcp 65001 >nul\r\n\
             set \"CUR={cur_str}\"\r\n\
             set \"NEW={new_str}\"\r\n\
             for /L %%i in (1,1,15) do (\r\n\
                 move /Y \"%NEW%\" \"%CUR%\" >nul 2>&1\r\n\
                 if not errorlevel 1 goto launch\r\n\
                 timeout /t 1 /nobreak >nul\r\n\
             )\r\n\
             :launch\r\n\
             start \"\" \"%CUR%\"\r\n\
             (goto) 2>nul & del \"%~f0\"\r\n"
        );

        if let Err(e) = fs::write(&bat_path, bat_script) {
            let _ = fs::remove_file(&new_exe);
            let _ = proxy.send_event(UserEvent::UpdateError(format!(
                "Ошибка подготовки обновления: {e}"
            )));
            return;
        }

        let launched = Command::new("cmd")
            .args(["/C", bat_path.to_string_lossy().as_ref()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();

        if launched.is_ok() {
            let _ = proxy.send_event(UserEvent::Quit);
        } else {
            let _ = proxy.send_event(UserEvent::UpdateError(
                "Не удалось запустить установщик обновления.".into(),
            ));
        }
    });
}

fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

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
    hotkey_manager
        .register(toggle_hotkey)
        .expect("failed to register Alt+Q hotkey");

    let proxy_hk = event_loop.create_proxy();
    let hk_id = toggle_hotkey.id();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.id == hk_id && event.state == HotKeyState::Pressed {
            let _ = proxy_hk.send_event(UserEvent::ToggleOverlay);
        }
    }));

    let item_toggle = MenuItem::new("Открыть / Скрыть (Alt+Q)", true, None);
    let item_quit = MenuItem::new("Выход", true, None);

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&item_toggle);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let _ = tray_menu.append(&item_quit);

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(format!("NG Helper v{} (Alt+Q)", APP_VERSION))
        .with_icon(make_tray_icon())
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
                    window.set_always_on_top(always_on_top);
                    window.set_focus();
                }
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
            Event::UserEvent(UserEvent::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}