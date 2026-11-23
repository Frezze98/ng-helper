#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::borrow::Cow;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

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
const RULES_META_JSON: &str = include_str!("../assets/rules_meta.json");
const REMOTE_RULES_META_URL: &str =
    "https://raw.githubusercontent.com/Frezze98/ng-helper/main/overlay/assets/rules_meta.json";
const REMOTE_RULES_INDEX_URL: &str =
    "https://raw.githubusercontent.com/Frezze98/ng-helper/main/overlay/assets/rules_index.json";

const AI_STUDIO_URL: &str = "https://aistudio.google.com/apikey";
const CREATE_NO_WINDOW: u32 = 0x08000000;
const SINGLE_INSTANCE_ADDR: &str = "127.0.0.1:47631";
const DEFAULT_TOGGLE_HK: &str = "Alt+Q";
const DEFAULT_QUIT_HK: &str = "Alt+Shift+Q";

enum UserEvent {
    ToggleOverlay,
    ShowOverlay,
    HideOverlay,
    DragWindow,
    SetAlwaysOnTop(bool),
    Resize(f64, f64),
    SetHotkeys(String, String),
    UpdateError(String),
    RulesSyncDone {
        updated: bool,
        manual: bool,
        error: Option<String>,
    },
    Uninstall,
    Quit,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn app_data_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("NGHelper")
}

fn webview_data_dir() -> PathBuf {
    app_data_dir().join("webview")
}

fn cached_rules_path() -> PathBuf {
    app_data_dir().join("rules_index.json")
}

fn cached_meta_path() -> PathBuf {
    app_data_dir().join("rules_meta.json")
}

fn sync_state_path() -> PathBuf {
    app_data_dir().join("sync_state.json")
}

fn hotkeys_path() -> PathBuf {
    app_data_dir().join("hotkeys.json")
}

fn escape_js(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    let rest_str = after_colon.strip_prefix('"')?;
    let end = rest_str.find('"')?;
    Some(rest_str[..end].to_string())
}

fn extract_json_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let idx = json.find(&needle)?;
    let rest = &json[idx + needle.len()..];
    let colon = rest.find(':')?;
    let digits: String = rest[colon + 1..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u64>().ok()
}

fn is_cached_rules_active() -> bool {
    if let (Ok(cached_meta), Ok(bytes)) = (
        fs::read_to_string(cached_meta_path()),
        fs::read(cached_rules_path()),
    ) {
        let cached_ts = extract_json_str(&cached_meta, "updated_at").unwrap_or_default();
        let embedded_ts = extract_json_str(RULES_META_JSON, "updated_at").unwrap_or_default();
        return cached_ts >= embedded_ts && bytes.len() > 1024 && bytes.first() == Some(&b'[');
    }
    false
}

fn load_rules_bytes() -> Cow<'static, [u8]> {
    if is_cached_rules_active() {
        if let Ok(bytes) = fs::read(cached_rules_path()) {
            return Cow::Owned(bytes);
        }
    }
    Cow::Borrowed(RULES_JSON)
}

fn save_sync_state(updated: bool, manual: bool, result: &str) {
    let dir = app_data_dir();
    let _ = fs::create_dir_all(&dir);
    let ts = now_unix();
    let mode = if manual { "manual" } else { "auto" };

    let prev = fs::read_to_string(sync_state_path()).unwrap_or_default();
    let prev_upd_ts = extract_json_u64(&prev, "updated_ts").unwrap_or(0);
    let prev_upd_mode = extract_json_str(&prev, "updated_mode").unwrap_or_else(|| "embedded".into());
    let prev_auto_ts = extract_json_u64(&prev, "last_auto_ts").unwrap_or(0);
    let prev_manual_ts = extract_json_u64(&prev, "last_manual_ts").unwrap_or(0);

    let updated_ts = if updated { ts } else { prev_upd_ts };
    let updated_mode = if updated { mode.to_string() } else { prev_upd_mode };
    let last_auto_ts = if !manual && result != "error" { ts } else { prev_auto_ts };
    let last_manual_ts = if manual && result != "error" { ts } else { prev_manual_ts };

    let json = format!(
        r#"{{"updated_ts":{},"updated_mode":"{}","checked_ts":{},"checked_mode":"{}","last_auto_ts":{},"last_manual_ts":{},"result":"{}"}}"#,
        updated_ts, updated_mode, ts, mode, last_auto_ts, last_manual_ts, result
    );
    let _ = fs::write(sync_state_path(), json);
}

fn build_rules_meta_response() -> Vec<u8> {
    let db_raw = if is_cached_rules_active() {
        fs::read_to_string(cached_meta_path()).unwrap_or_else(|_| RULES_META_JSON.to_string())
    } else {
        RULES_META_JSON.to_string()
    };
    let db_trim = db_raw.trim();
    let db_json = if db_trim.starts_with('{') && db_trim.ends_with('}') {
        db_trim
    } else {
        "{}"
    };

    let local_raw = fs::read_to_string(sync_state_path()).unwrap_or_default();
    let local_trim = local_raw.trim();
    let local_json = if local_trim.starts_with('{') && local_trim.ends_with('}') {
        local_trim
    } else {
        r#"{"updated_ts":0,"updated_mode":"embedded","checked_ts":0,"checked_mode":"none","last_auto_ts":0,"last_manual_ts":0,"result":"idle"}"#
    };

    format!(r#"{{"db":{},"local":{}}}"#, db_json, local_json).into_bytes()
}

fn sync_rules_in_background(proxy: EventLoopProxy<UserEvent>, manual: bool) {
    thread::spawn(move || {
        let output = Command::new("curl.exe")
            .args([
                "-L",
                "-f",
                "-sS",
                "--max-time",
                "12",
                "-H",
                "Cache-Control: no-cache",
                REMOTE_RULES_META_URL,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let remote_meta = match output {
            Ok(o) if o.status.success() => match String::from_utf8(o.stdout) {
                Ok(s) => s,
                Err(_) => {
                    save_sync_state(false, manual, "error");
                    let _ = proxy.send_event(UserEvent::RulesSyncDone {
                        updated: false,
                        manual,
                        error: Some("Некоректна відповідь сервера правил.".into()),
                    });
                    return;
                }
            },
            _ => {
                save_sync_state(false, manual, "error");
                let _ = proxy.send_event(UserEvent::RulesSyncDone {
                    updated: false,
                    manual,
                    error: Some("Не вдалося підключитися до GitHub.".into()),
                });
                return;
            }
        };

        let Some(remote_hash) = extract_json_str(&remote_meta, "sha256") else {
            save_sync_state(false, manual, "error");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: false,
                manual,
                error: Some("Некоректний файл метаданих правил.".into()),
            });
            return;
        };

        let local_meta = if is_cached_rules_active() {
            fs::read_to_string(cached_meta_path()).unwrap_or_else(|_| RULES_META_JSON.to_string())
        } else {
            RULES_META_JSON.to_string()
        };
        let local_hash = extract_json_str(&local_meta, "sha256").unwrap_or_default();

        if remote_hash == local_hash {
            save_sync_state(false, manual, "up_to_date");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: false,
                manual,
                error: None,
            });
            return;
        }

        let dir = app_data_dir();
        let _ = fs::create_dir_all(&dir);
        let tmp_path = dir.join("rules_index.json.tmp");

        let dl = Command::new("curl.exe")
            .args([
                "-L",
                "-f",
                "-sS",
                "--max-time",
                "30",
                "-H",
                "Cache-Control: no-cache",
                "-o",
                tmp_path.to_string_lossy().as_ref(),
                REMOTE_RULES_INDEX_URL,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();

        if !matches!(dl, Ok(s) if s.success()) {
            let _ = fs::remove_file(&tmp_path);
            save_sync_state(false, manual, "error");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: false,
                manual,
                error: Some("Не вдалося завантажити базу правил.".into()),
            });
            return;
        }

        let valid = fs::read(&tmp_path)
            .map(|b| {
                let first = b.iter().find(|c| !c.is_ascii_whitespace());
                let last = b.iter().rev().find(|c| !c.is_ascii_whitespace());
                b.len() > 1024 && first == Some(&b'[') && last == Some(&b']')
            })
            .unwrap_or(false);

        if !valid {
            let _ = fs::remove_file(&tmp_path);
            save_sync_state(false, manual, "error");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: false,
                manual,
                error: Some("Завантажений файл правил пошкоджений.".into()),
            });
            return;
        }

        let target = cached_rules_path();
        let _ = fs::remove_file(&target);
        if fs::rename(&tmp_path, &target).is_ok() {
            let _ = fs::write(cached_meta_path(), &remote_meta);
            save_sync_state(true, manual, "updated");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: true,
                manual,
                error: None,
            });
        } else {
            let _ = fs::remove_file(&tmp_path);
            save_sync_state(false, manual, "error");
            let _ = proxy.send_event(UserEvent::RulesSyncDone {
                updated: false,
                manual,
                error: Some("Не вдалося зберегти базу правил.".into()),
            });
        }
    });
}

fn parse_hotkey(spec: &str) -> Result<HotKey, String> {
    let mut mods = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for raw in spec.split('+') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        match token.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods |= Modifiers::CONTROL,
            "ALT" => mods |= Modifiers::ALT,
            "SHIFT" => mods |= Modifiers::SHIFT,
            k => {
                let code = match k {
                    "A" => Code::KeyA,
                    "B" => Code::KeyB,
                    "C" => Code::KeyC,
                    "D" => Code::KeyD,
                    "E" => Code::KeyE,
                    "F" => Code::KeyF,
                    "G" => Code::KeyG,
                    "H" => Code::KeyH,
                    "I" => Code::KeyI,
                    "J" => Code::KeyJ,
                    "K" => Code::KeyK,
                    "L" => Code::KeyL,
                    "M" => Code::KeyM,
                    "N" => Code::KeyN,
                    "O" => Code::KeyO,
                    "P" => Code::KeyP,
                    "Q" => Code::KeyQ,
                    "R" => Code::KeyR,
                    "S" => Code::KeyS,
                    "T" => Code::KeyT,
                    "U" => Code::KeyU,
                    "V" => Code::KeyV,
                    "W" => Code::KeyW,
                    "X" => Code::KeyX,
                    "Y" => Code::KeyY,
                    "Z" => Code::KeyZ,
                    "0" => Code::Digit0,
                    "1" => Code::Digit1,
                    "2" => Code::Digit2,
                    "3" => Code::Digit3,
                    "4" => Code::Digit4,
                    "5" => Code::Digit5,
                    "6" => Code::Digit6,
                    "7" => Code::Digit7,
                    "8" => Code::Digit8,
                    "9" => Code::Digit9,
                    "F1" => Code::F1,
                    "F2" => Code::F2,
                    "F3" => Code::F3,
                    "F4" => Code::F4,
                    "F5" => Code::F5,
                    "F6" => Code::F6,
                    "F7" => Code::F7,
                    "F8" => Code::F8,
                    "F9" => Code::F9,
                    "F10" => Code::F10,
                    "F11" => Code::F11,
                    "F12" => Code::F12,
                    "SPACE" => Code::Space,
                    "TAB" => Code::Tab,
                    "BACKQUOTE" | "`" | "~" => Code::Backquote,
                    "INSERT" => Code::Insert,
                    "DELETE" => Code::Delete,
                    "HOME" => Code::Home,
                    "END" => Code::End,
                    "PAGEUP" => Code::PageUp,
                    "PAGEDOWN" => Code::PageDown,
                    "MINUS" | "-" => Code::Minus,
                    "EQUAL" | "=" => Code::Equal,
                    "BRACKETLEFT" | "[" => Code::BracketLeft,
                    "BRACKETRIGHT" | "]" => Code::BracketRight,
                    "SEMICOLON" | ";" => Code::Semicolon,
                    "QUOTE" | "'" => Code::Quote,
                    "COMMA" | "," => Code::Comma,
                    "PERIOD" | "." => Code::Period,
                    "SLASH" | "/" => Code::Slash,
                    "BACKSLASH" | "\\" => Code::Backslash,
                    "NUMPAD0" => Code::Numpad0,
                    "NUMPAD1" => Code::Numpad1,
                    "NUMPAD2" => Code::Numpad2,
                    "NUMPAD3" => Code::Numpad3,
                    "NUMPAD4" => Code::Numpad4,
                    "NUMPAD5" => Code::Numpad5,
                    "NUMPAD6" => Code::Numpad6,
                    "NUMPAD7" => Code::Numpad7,
                    "NUMPAD8" => Code::Numpad8,
                    "NUMPAD9" => Code::Numpad9,
                    _ => return Err(format!("Непідтримувана клавіша: {k}")),
                };
                key_code = Some(code);
            }
        }
    }

    let code = key_code.ok_or_else(|| "Не вказано основну клавішу.".to_string())?;
    let opt_mods = if mods.is_empty() { None } else { Some(mods) };
    Ok(HotKey::new(opt_mods, code))
}

fn load_saved_hotkeys() -> (String, String) {
    if let Ok(raw) = fs::read_to_string(hotkeys_path()) {
        let t = extract_json_str(&raw, "toggle").unwrap_or_else(|| DEFAULT_TOGGLE_HK.into());
        let q = extract_json_str(&raw, "quit").unwrap_or_else(|| DEFAULT_QUIT_HK.into());
        if parse_hotkey(&t).is_ok() && parse_hotkey(&q).is_ok() && t != q {
            return (t, q);
        }
    }
    (DEFAULT_TOGGLE_HK.into(), DEFAULT_QUIT_HK.into())
}

fn save_hotkeys(toggle: &str, quit: &str) {
    let dir = app_data_dir();
    let _ = fs::create_dir_all(&dir);
    let json = format!(
        r#"{{"toggle":"{}","quit":"{}"}}"#,
        escape_js(toggle),
        escape_js(quit)
    );
    let _ = fs::write(hotkeys_path(), json);
}

fn log_error(msg: &str) {
    let dir = app_data_dir();
    let _ = fs::create_dir_all(&dir);
    let ts = now_unix();
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
        || url.starts_with(&format!("https://github.com/{}", GITHUB_REPO))
        || url.starts_with("https://discord.com/users/")
        || url.starts_with("https://console.groq.com/")
        || url.starts_with("https://openrouter.ai/")
        || url.starts_with("https://platform.openai.com/")
        || url.starts_with("https://console.anthropic.com/")
        || url.starts_with("https://platform.deepseek.com/")
        || url.starts_with("https://console.x.ai/");
    if !allowed {
        return;
    }
    let _ = Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

fn find_uninstaller() -> Option<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(mut exe_dir) = std::env::current_exe() {
        exe_dir.pop();
        dirs.push(exe_dir);
    }
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local_app).join("Programs").join("NG Helper"));
    }

    for dir in dirs {
        let direct = dir.join("unins000.exe");
        if direct.exists() {
            return Some(direct);
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("unins") && name.ends_with(".exe") {
                    return Some(entry.path());
                }
            }
        }
    }
    None
}

fn run_uninstall() -> Result<(), String> {
    let uninst = find_uninstaller().ok_or_else(|| {
        "Деінсталятор не знайдено (працює лише після встановлення через Setup-NG-Helper.exe)."
            .to_string()
    })?;

    let started = Command::new(&uninst).spawn().or_else(|_| {
        Command::new("cmd")
            .args(["/C", "start", "", uninst.to_string_lossy().as_ref()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    });

    match started {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Не вдалося запустити деінсталятор: {e}")),
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
        .with_inner_size(LogicalSize::new(590.0, 770.0))
        .with_decorations(false)
        .with_always_on_top(true)
        .with_skip_taskbar(true)
        .with_visible(true)
        .build(&event_loop)
        .expect("failed to build overlay window");

    let (init_toggle_str, init_quit_str) = load_saved_hotkeys();
    let proxy_ipc = event_loop.create_proxy();

    let mut web_context = WebContext::new(Some(webview_data_dir()));
    let webview = WebViewBuilder::new_with_web_context(&mut web_context)
        .with_custom_protocol("ng".into(), |_webview_id, request| {
            let path = request.uri().path();
            let (mime, body): (&str, Cow<'static, [u8]>) = match path {
                "/rules_index.json" => ("application/json; charset=utf-8", load_rules_bytes()),
                "/rules_meta.json" => (
                    "application/json; charset=utf-8",
                    Cow::Owned(build_rules_meta_response()),
                ),
                "/app_info.json" => {
                    let (t_hk, q_hk) = load_saved_hotkeys();
                    (
                        "application/json; charset=utf-8",
                        Cow::Owned(
                            format!(
                                r#"{{"version":"{}","repo":"{}","toggleHotkey":"{}","quitHotkey":"{}"}}"#,
                                APP_VERSION,
                                GITHUB_REPO,
                                escape_js(&t_hk),
                                escape_js(&q_hk)
                            )
                            .into_bytes(),
                        ),
                    )
                }
                _ => ("text/html; charset=utf-8", Cow::Borrowed(UI_HTML)),
            };
            Response::builder()
                .status(200)
                .header("Content-Type", mime)
                .header("Cache-Control", "no-store")
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
            } else if msg == "sync_rules_manual" {
                sync_rules_in_background(proxy_ipc.clone(), true);
            } else if msg == "sync_rules_auto" || msg == "sync_rules" {
                sync_rules_in_background(proxy_ipc.clone(), false);
            } else if let Some(pair) = msg.strip_prefix("set_hotkeys:") {
                let parts: Vec<&str> = pair.split('|').collect();
                if parts.len() == 2 {
                    let _ = proxy_ipc.send_event(UserEvent::SetHotkeys(
                        parts[0].trim().to_string(),
                        parts[1].trim().to_string(),
                    ));
                }
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

    sync_rules_in_background(event_loop.create_proxy(), false);

    let hotkey_manager = GlobalHotKeyManager::new().expect("failed to init hotkey manager");
    let mut current_toggle_hk = parse_hotkey(&init_toggle_str)
        .unwrap_or_else(|_| HotKey::new(Some(Modifiers::ALT), Code::KeyQ));
    let mut current_quit_hk = parse_hotkey(&init_quit_str)
        .unwrap_or_else(|_| HotKey::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyQ));

    if let Err(e) = hotkey_manager.register(current_toggle_hk) {
        log_error(&format!("Toggle hotkey register failed: {e}"));
    }
    if let Err(e) = hotkey_manager.register(current_quit_hk) {
        log_error(&format!("Quit hotkey register failed: {e}"));
    }

    let toggle_id_atom = Arc::new(AtomicU32::new(current_toggle_hk.id()));
    let quit_id_atom = Arc::new(AtomicU32::new(current_quit_hk.id()));

    let proxy_hk = event_loop.create_proxy();
    let t_atom_cb = Arc::clone(&toggle_id_atom);
    let q_atom_cb = Arc::clone(&quit_id_atom);
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state == HotKeyState::Pressed {
            if event.id == t_atom_cb.load(Ordering::Relaxed) {
                let _ = proxy_hk.send_event(UserEvent::ToggleOverlay);
            } else if event.id == q_atom_cb.load(Ordering::Relaxed) {
                let _ = proxy_hk.send_event(UserEvent::Quit);
            }
        }
    }));

    let item_toggle = MenuItem::new("Відкрити / Сховати", true, None);
    let item_quit = MenuItem::new("Вихід", true, None);

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&item_toggle);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let _ = tray_menu.append(&item_quit);

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip(format!("NG Helper v{} ({})", APP_VERSION, init_toggle_str))
        .with_icon(load_tray_icon())
        .with_menu_on_left_click(false)
        .build()
        .expect("failed to build tray icon");

    let toggle_menu_id = item_toggle.id().clone();
    let quit_menu_id = item_quit.id().clone();

    let proxy_menu = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if event.id == toggle_menu_id {
            let _ = proxy_menu.send_event(UserEvent::ToggleOverlay);
        } else if event.id == quit_menu_id {
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
            Event::UserEvent(UserEvent::SetHotkeys(toggle_str, quit_str)) => {
                let res = (|| -> Result<(), String> {
                    let new_t = parse_hotkey(&toggle_str)?;
                    let new_q = parse_hotkey(&quit_str)?;
                    if new_t.id() == new_q.id() {
                        return Err("Комбінації не можуть співпадати.".into());
                    }
                    let _ = hotkey_manager.unregister(current_toggle_hk);
                    let _ = hotkey_manager.unregister(current_quit_hk);

                    if let Err(e) = hotkey_manager.register(new_t) {
                        let _ = hotkey_manager.register(current_toggle_hk);
                        let _ = hotkey_manager.register(current_quit_hk);
                        return Err(format!("Клавіша {toggle_str} зайнята системою ({e})."));
                    }
                    if let Err(e) = hotkey_manager.register(new_q) {
                        let _ = hotkey_manager.unregister(new_t);
                        let _ = hotkey_manager.register(current_toggle_hk);
                        let _ = hotkey_manager.register(current_quit_hk);
                        return Err(format!("Клавіша {quit_str} зайнята системою ({e})."));
                    }

                    current_toggle_hk = new_t;
                    current_quit_hk = new_q;
                    toggle_id_atom.store(new_t.id(), Ordering::Relaxed);
                    quit_id_atom.store(new_q.id(), Ordering::Relaxed);
                    save_hotkeys(&toggle_str, &quit_str);
                    let _ = _tray_icon
                        .set_tooltip(Some(format!("NG Helper v{} ({})", APP_VERSION, toggle_str)));
                    Ok(())
                })();

                match res {
                    Ok(()) => {
                        let _ = webview.evaluate_script(&format!(
                            "window.onHotkeysResult && window.onHotkeysResult(true, \"{}\", \"{}\", \"\");",
                            escape_js(&toggle_str),
                            escape_js(&quit_str)
                        ));
                    }
                    Err(err) => {
                        let _ = webview.evaluate_script(&format!(
                            "window.onHotkeysResult && window.onHotkeysResult(false, \"\", \"\", \"{}\");",
                            escape_js(&err)
                        ));
                    }
                }
            }
            Event::UserEvent(UserEvent::UpdateError(msg)) => {
                let safe = escape_js(&msg);
                let _ = webview.evaluate_script(&format!(
                    "window.onUpdateError && window.onUpdateError(\"{safe}\");"
                ));
            }
            Event::UserEvent(UserEvent::RulesSyncDone {
                updated,
                manual,
                error,
            }) => {
                let err_str = error.map(|e| escape_js(&e)).unwrap_or_default();
                let _ = webview.evaluate_script(&format!(
                    "window.onRulesSyncDone && window.onRulesSyncDone({}, {}, \"{}\");",
                    updated, manual, err_str
                ));
            }
            Event::UserEvent(UserEvent::Uninstall) => match run_uninstall() {
                Ok(()) => *control_flow = ControlFlow::Exit,
                Err(msg) => {
                    let safe = escape_js(&msg);
                    let _ = webview.evaluate_script(&format!(
                        "window.onUpdateError && window.onUpdateError(\"{safe}\");"
                    ));
                }
            },
            Event::UserEvent(UserEvent::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}