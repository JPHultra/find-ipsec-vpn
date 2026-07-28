use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VpnProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub remote_identity: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfilesConfig {
    pub active_profile_id: String,
    pub profiles: Vec<VpnProfile>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecretsStatus {
    pub has_psk: bool,
    pub has_password: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SecretsStore {
    pub psk: String,
    pub password: String,
}

pub struct VpnState {
    pub child_stdin: Mutex<Option<ChildStdin>>,
    pub child_process: Mutex<Option<Child>>,
    pub status_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
    pub traffic_item: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config").join("findmore-vpn")
}

#[derive(Deserialize)]
struct UserConfigLegacy {
    host: String,
    remote_identity: String,
    username: String,
}

fn load_secrets_legacy() -> Result<(String, String), String> {
    if let (Ok(entry_psk), Ok(entry_pwd)) = (
        keyring::Entry::new("findmore-vpn", "psk"),
        keyring::Entry::new("findmore-vpn", "password"),
    ) {
        if let (Ok(psk), Ok(password)) = (entry_psk.get_password(), entry_pwd.get_password()) {
            return Ok((psk, password));
        }
    }
    let path = get_config_dir().join("secrets.json");
    if path.exists() {
        if let Ok(file) = File::open(path) {
            #[derive(Deserialize)]
            struct SecretsStoreLegacy { psk: String, password: String }
            if let Ok(sec) = serde_json::from_reader::<_, SecretsStoreLegacy>(file) {
                return Ok((sec.psk, sec.password));
            }
        }
    }
    Err("No legacy secrets found".to_string())
}

fn delete_secrets_legacy() -> Result<(), String> {
    if let (Ok(entry_psk), Ok(entry_pwd)) = (
        keyring::Entry::new("findmore-vpn", "psk"),
        keyring::Entry::new("findmore-vpn", "password"),
    ) {
        let _ = entry_psk.delete_credential();
        let _ = entry_pwd.delete_credential();
    }
    Ok(())
}

fn create_default_config() -> ProfilesConfig {
    ProfilesConfig {
        active_profile_id: "".to_string(),
        profiles: vec![],
    }
}

fn load_profiles_config() -> Result<ProfilesConfig, String> {
    let path = get_config_dir().join("config.json");
    if !path.exists() {
        let config = create_default_config();
        let _ = save_profiles_config(&config);
        return Ok(config);
    }
    
    let file = File::open(&path).map_err(|e| e.to_string())?;
    match serde_json::from_reader::<_, ProfilesConfig>(&file) {
        Ok(config) => Ok(config),
        Err(_) => {
            // Parsing failed. Attempt to migrate from legacy UserConfig layout
            if let Ok(file_old) = File::open(&path) {
                if let Ok(old_user_config) = serde_json::from_reader::<_, UserConfigLegacy>(file_old) {
                    let migrated_profile = VpnProfile {
                        id: "default".to_string(),
                        name: "Default Profile".to_string(),
                        host: old_user_config.host,
                        remote_identity: old_user_config.remote_identity,
                        username: old_user_config.username,
                    };
                    
                    // Migrate associated legacy secrets
                    if let Ok((old_psk, old_pwd)) = load_secrets_legacy() {
                        let _ = save_secrets("default", &old_psk, &old_pwd);
                        let _ = delete_secrets_legacy();
                    }
                    
                    let migrated_config = ProfilesConfig {
                        active_profile_id: "default".to_string(),
                        profiles: vec![migrated_profile],
                    };
                    
                    let _ = save_profiles_config(&migrated_config);
                    return Ok(migrated_config);
                }
            }
            
            // Overwrite with clean default configuration if migration fails
            let default_config = create_default_config();
            let _ = save_profiles_config(&default_config);
            Ok(default_config)
        }
    }
}

fn save_profiles_config(config: &ProfilesConfig) -> Result<(), String> {
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("config.json");
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, config).map_err(|e| e.to_string())?;
    Ok(())
}

fn save_secrets(profile_id: &str, psk: &str, password: &str) -> Result<(), String> {
    // 1. Try system keyring
    let _ = (|| {
        let entry_psk = keyring::Entry::new("findmore-vpn", &format!("psk-{}", profile_id))?;
        entry_psk.set_password(psk)?;
        let entry_pwd = keyring::Entry::new("findmore-vpn", &format!("password-{}", profile_id))?;
        entry_pwd.set_password(password)?;
        Ok::<(), keyring::Error>(())
    })();

    // 2. Always persist restricted fallback file secrets.json (0600 mode) for reboot persistence
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("secrets.json");

    let mut secrets_map = if path.exists() {
        if let Ok(file) = File::open(&path) {
            serde_json::from_reader::<_, HashMap<String, SecretsStore>>(file).unwrap_or_default()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    secrets_map.insert(profile_id.to_string(), SecretsStore {
        psk: psk.to_string(),
        password: password.to_string(),
    });

    let file = File::create(&path).map_err(|e| e.to_string())?;
    serde_json::to_writer(file, &secrets_map).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn load_secrets(profile_id: &str) -> Result<(String, String), String> {
    // 1. Try keyring
    if let (Ok(entry_psk), Ok(entry_pwd)) = (
        keyring::Entry::new("findmore-vpn", &format!("psk-{}", profile_id)),
        keyring::Entry::new("findmore-vpn", &format!("password-{}", profile_id)),
    ) {
        if let (Ok(psk), Ok(password)) = (entry_psk.get_password(), entry_pwd.get_password()) {
            return Ok((psk, password));
        }
    }

    // 2. Try fallback file
    let path = get_config_dir().join("secrets.json");
    if path.exists() {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let secrets_map: HashMap<String, SecretsStore> = serde_json::from_reader(file).map_err(|e| e.to_string())?;
        if let Some(secrets) = secrets_map.get(profile_id) {
            return Ok((secrets.psk.clone(), secrets.password.clone()));
        }
    }

    Err("Pre-shared key and Password not found".to_string())
}

fn delete_secrets(profile_id: &str) -> Result<(), String> {
    if let (Ok(entry_psk), Ok(entry_pwd)) = (
        keyring::Entry::new("findmore-vpn", &format!("psk-{}", profile_id)),
        keyring::Entry::new("findmore-vpn", &format!("password-{}", profile_id)),
    ) {
        let _ = entry_psk.delete_credential();
        let _ = entry_pwd.delete_credential();
    }

    let path = get_config_dir().join("secrets.json");
    if path.exists() {
        if let Ok(file) = File::open(&path) {
            if let Ok(mut secrets_map) = serde_json::from_reader::<_, HashMap<String, SecretsStore>>(file) {
                if secrets_map.remove(profile_id).is_some() {
                    if let Ok(writer_file) = File::create(&path) {
                        let _ = serde_json::to_writer(writer_file, &secrets_map);
                    }
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
fn get_profiles() -> Result<ProfilesConfig, String> {
    load_profiles_config()
}

#[tauri::command]
fn save_profile(profile: VpnProfile, psk: Option<String>, password: Option<String>) -> Result<(), String> {
    let mut config = load_profiles_config()?;
    
    if let (Some(p), Some(w)) = (&psk, &password) {
        if !p.is_empty() && !w.is_empty() {
            save_secrets(&profile.id, p, w)?;
        }
    }
    
    if let Some(pos) = config.profiles.iter().position(|p| p.id == profile.id) {
        config.profiles[pos] = profile.clone();
    } else {
        config.profiles.push(profile.clone());
    }
    
    config.active_profile_id = profile.id.clone();
    save_profiles_config(&config)?;
    Ok(())
}

#[tauri::command]
fn delete_profile(profile_id: String) -> Result<ProfilesConfig, String> {
    let mut config = load_profiles_config()?;
    
    config.profiles.retain(|p| p.id != profile_id);
    
    if config.profiles.is_empty() {
        config.active_profile_id = "".to_string();
    } else if config.active_profile_id == profile_id {
        config.active_profile_id = config.profiles[0].id.clone();
    }
    
    let _ = delete_secrets(&profile_id);
    save_profiles_config(&config)?;
    Ok(config)
}

#[tauri::command]
fn set_active_profile(profile_id: String) -> Result<(), String> {
    let mut config = load_profiles_config()?;
    if config.profiles.iter().any(|p| p.id == profile_id) {
        config.active_profile_id = profile_id;
        save_profiles_config(&config)?;
        Ok(())
    } else {
        Err("Profile not found".to_string())
    }
}

#[tauri::command]
fn get_profile_secrets_status(profile_id: String) -> Result<SecretsStatus, String> {
    let has_secrets = load_secrets(&profile_id).is_ok();
    Ok(SecretsStatus {
        has_psk: has_secrets,
        has_password: has_secrets,
    })
}

#[tauri::command]
fn get_profile_secrets(profile_id: String) -> Result<SecretsStore, String> {
    let (psk, password) = load_secrets(&profile_id)?;
    Ok(SecretsStore { psk, password })
}

#[tauri::command]
fn connect_vpn(app_handle: AppHandle, state: State<'_, VpnState>) -> Result<(), String> {
    let mut child_stdin_lock = state.child_stdin.lock().unwrap();
    let mut child_proc_lock = state.child_process.lock().unwrap();
    
    // Clean up stale or exited child process if any
    if let Some(ref mut child) = *child_proc_lock {
        if let Ok(Some(_)) = child.try_wait() {
            *child_proc_lock = None;
            *child_stdin_lock = None;
        }
    }

    if child_proc_lock.is_some() {
        return Err("VPN connection helper is already running".to_string());
    }

    let config = load_profiles_config()?;
    if config.profiles.is_empty() {
        return Err("No profiles configured. Please create a profile in the Profiles tab first.".to_string());
    }
    let active_profile = config.profiles.iter().find(|p| p.id == config.active_profile_id)
        .ok_or_else(|| "Active profile not found".to_string())?;
    
    let (psk, password) = load_secrets(&active_profile.id)?;

    let helper_path = if std::path::Path::new("/usr/bin/findmore-vpn-helper").exists() {
        "/usr/bin/findmore-vpn-helper".to_string()
    } else {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let local_helper = parent.join("findmore-vpn-helper");
                if local_helper.exists() {
                    local_helper.to_string_lossy().to_string()
                } else {
                    "/usr/bin/findmore-vpn-helper".to_string()
                }
            } else {
                "/usr/bin/findmore-vpn-helper".to_string()
            }
        } else {
            "/usr/bin/findmore-vpn-helper".to_string()
        }
    };

    let mut child = Command::new("pkexec")
        .arg(&helper_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start elevated helper via pkexec: {}", e))?;

    let mut stdin = child.stdin.take().ok_or_else(|| "Failed to capture helper stdin".to_string())?;
    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture helper stdout".to_string())?;

    let connect_cmd = serde_json::json!({
        "type": "Connect",
        "host": active_profile.host,
        "remote_identity": active_profile.remote_identity,
        "username": active_profile.username,
        "psk": psk,
        "password": password
    });
    
    let connect_line = serde_json::to_string(&connect_cmd).unwrap();
    if let Err(e) = writeln!(stdin, "{}", connect_line) {
        let _ = child.kill();
        return Err(format!("Failed to write to helper stdin: {}", e));
    }
    let _ = stdin.flush();

    *child_stdin_lock = Some(stdin);
    *child_proc_lock = Some(child);

    let app_handle_clone = app_handle.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = app_handle_clone.emit("vpn-event", l);
            }
        }
        
        // Clean up process state locks when helper process exits or Polkit prompt fails
        let state_ref = app_handle_clone.state::<VpnState>();
        *state_ref.child_stdin.lock().unwrap() = None;
        *state_ref.child_process.lock().unwrap() = None;

        let _ = app_handle_clone.emit("vpn-event", "{\"type\":\"Status\",\"state\":\"Disconnected\",\"message\":\"Helper process exited\"}");
    });

    Ok(())
}

#[tauri::command]
fn submit_otp(otp: String, state: State<'_, VpnState>) -> Result<(), String> {
    let mut stdin_lock = state.child_stdin.lock().unwrap();
    if let Some(ref mut stdin) = *stdin_lock {
        let otp_cmd = serde_json::json!({
            "type": "SubmitOtp",
            "otp": otp
        });
        let otp_line = serde_json::to_string(&otp_cmd).unwrap();
        writeln!(stdin, "{}", otp_line).map_err(|e| format!("Failed to write OTP to helper stdin: {}", e))?;
        let _ = stdin.flush();
        Ok(())
    } else {
        Err("No active VPN session found".to_string())
    }
}

#[tauri::command]
fn disconnect_vpn(state: State<'_, VpnState>) -> Result<(), String> {
    let mut stdin_lock = state.child_stdin.lock().unwrap();
    let mut proc_lock = state.child_process.lock().unwrap();

    if let Some(ref mut stdin) = *stdin_lock {
        let disc_cmd = serde_json::json!({
            "type": "Disconnect"
        });
        let disc_line = serde_json::to_string(&disc_cmd).unwrap();
        let _ = writeln!(stdin, "{}", disc_line);
        let _ = stdin.flush();
    }
    
    *stdin_lock = None;

    if let Some(mut child) = proc_lock.take() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = child.kill();
        let _ = child.wait();
    }

    Ok(())
}

#[tauri::command]
fn show_notification(app: AppHandle, title: String, body: String) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
    Ok(())
}

#[tauri::command]
fn update_tray_status(state: State<'_, VpnState>, status: String, traffic: String) -> Result<(), String> {
    if let Some(ref status_item) = *state.status_item.lock().unwrap() {
        let _ = status_item.set_text(format!("Status: {}", status));
    }
    if let Some(ref traffic_item) = *state.traffic_item.lock().unwrap() {
        let _ = traffic_item.set_text(format!("Traffic: {}", traffic));
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
        Manager, WindowEvent,
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(VpnState {
            child_stdin: Mutex::new(None),
            child_process: Mutex::new(None),
            status_item: Mutex::new(None),
            traffic_item: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_profiles,
            save_profile,
            delete_profile,
            set_active_profile,
            get_profile_secrets_status,
            get_profile_secrets,
            connect_vpn,
            submit_otp,
            disconnect_vpn,
            show_notification,
            update_tray_status
        ])
        .setup(|app| {
            let status_i = MenuItem::with_id(app, "status_info", "Status: Disconnected", false, None::<&str>)?;
            let traffic_i = MenuItem::with_id(app, "traffic_info", "Traffic: 0.00 MB ↓ / 0.00 MB ↑", false, None::<&str>)?;
            let sep_i = PredefinedMenuItem::separator(app)?;
            let show_i = MenuItem::with_id(app, "show", "Show Dashboard", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(app, "hide", "Hide to Tray", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Findmore VPN", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &status_i,
                &traffic_i,
                &sep_i,
                &show_i,
                &hide_i,
                &quit_i,
            ])?;

            let state = app.state::<VpnState>();
            *state.status_item.lock().unwrap() = Some(status_i);
            *state.traffic_item.lock().unwrap() = Some(traffic_i);

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "hide" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
