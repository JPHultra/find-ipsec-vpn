use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserConfig {
    pub host: String,
    pub remote_identity: String,
    pub username: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FullConfig {
    pub config: UserConfig,
    pub has_psk: bool,
    pub has_password: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct SecretsStore {
    psk: String,
    password: String,
}

pub struct VpnState {
    pub child_stdin: Mutex<Option<ChildStdin>>,
    pub child_process: Mutex<Option<Child>>,
}

fn get_config_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(home).join(".config").join("findmore-vpn")
}

fn save_user_config(config: &UserConfig) -> Result<(), String> {
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("config.json");
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, config).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_user_config() -> Result<UserConfig, String> {
    let path = get_config_dir().join("config.json");
    if !path.exists() {
        return Ok(UserConfig {
            host: "".to_string(),
            remote_identity: "".to_string(),
            username: "".to_string(),
        });
    }
    let file = File::open(path).map_err(|e| e.to_string())?;
    let config: UserConfig = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    Ok(config)
}

fn save_secrets(psk: &str, password: &str) -> Result<(), String> {
    // 1. Try system keyring
    let keyring_ok = (|| {
        let entry_psk = keyring::Entry::new("findmore-vpn", "psk")?;
        entry_psk.set_password(psk)?;
        let entry_pwd = keyring::Entry::new("findmore-vpn", "password")?;
        entry_pwd.set_password(password)?;
        Ok::<(), keyring::Error>(())
    })().is_ok();

    if keyring_ok {
        // Clear fallback file if keyring worked
        let path = get_config_dir().join("secrets.json");
        if path.exists() {
            let _ = fs::remove_file(path);
        }
        return Ok(());
    }

    // 2. Fallback to owner-only readable secrets.json (0600)
    let dir = get_config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("secrets.json");
    
    let secrets = SecretsStore {
        psk: psk.to_string(),
        password: password.to_string(),
    };
    
    let file = File::create(&path).map_err(|e| e.to_string())?;
    serde_json::to_writer(file, &secrets).map_err(|e| e.to_string())?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

fn load_secrets() -> Result<(String, String), String> {
    // 1. Try keyring
    if let (Ok(entry_psk), Ok(entry_pwd)) = (
        keyring::Entry::new("findmore-vpn", "psk"),
        keyring::Entry::new("findmore-vpn", "password"),
    ) {
        if let (Ok(psk), Ok(password)) = (entry_psk.get_password(), entry_pwd.get_password()) {
            return Ok((psk, password));
        }
    }

    // 2. Try file fallback
    let path = get_config_dir().join("secrets.json");
    if !path.exists() {
        return Err("Pre-shared key and Password not found".to_string());
    }
    
    let file = File::open(path).map_err(|e| e.to_string())?;
    let secrets: SecretsStore = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    Ok((secrets.psk, secrets.password))
}

#[tauri::command]
fn get_config() -> Result<FullConfig, String> {
    let config = load_user_config()?;
    let (has_psk, has_password) = match load_secrets() {
        Ok((psk, pwd)) => (!psk.is_empty(), !pwd.is_empty()),
        Err(_) => (false, false),
    };
    Ok(FullConfig {
        config,
        has_psk,
        has_password,
    })
}

#[tauri::command]
fn save_config(config: UserConfig, psk: Option<String>, password: Option<String>) -> Result<(), String> {
    save_user_config(&config)?;
    if let (Some(p), Some(w)) = (psk, password) {
        if !p.is_empty() && !w.is_empty() {
            save_secrets(&p, &w)?;
        }
    }
    Ok(())
}

#[tauri::command]
fn connect_vpn(app_handle: AppHandle, state: State<'_, VpnState>) -> Result<(), String> {
    let mut child_stdin_lock = state.child_stdin.lock().unwrap();
    let mut child_proc_lock = state.child_process.lock().unwrap();
    
    if child_proc_lock.is_some() {
        return Err("VPN connection helper is already running".to_string());
    }

    let config = load_user_config()?;
    let (psk, password) = load_secrets()?;

    // Find findmore-vpn-helper path (check global, then development fallback)
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

    // Send Connection commands immediately to helper in JSON format
    let connect_cmd = serde_json::json!({
        "type": "Connect",
        "host": config.host,
        "remote_identity": config.remote_identity,
        "username": config.username,
        "psk": psk,
        "password": password
    });
    
    let connect_line = serde_json::to_string(&connect_cmd).unwrap();
    writeln!(stdin, "{}", connect_line).map_err(|e| format!("Failed to write to helper stdin: {}", e))?;
    let _ = stdin.flush();

    *child_stdin_lock = Some(stdin);
    *child_proc_lock = Some(child);

    // Read log outputs from helper in background
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(l) = line {
                let _ = app_handle.emit("vpn-event", l);
            }
        }
        // Emit offline event when connection terminates
        let _ = app_handle.emit("vpn-event", "{\"type\":\"Status\",\"state\":\"Disconnected\",\"message\":\"Helper process exited\"}");
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
        // Give the helper 500ms to receive command, kill charon-cmd gracefully, and exit
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = child.kill();
        let _ = child.wait();
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(VpnState {
            child_stdin: Mutex::new(None),
            child_process: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            connect_vpn,
            submit_otp,
            disconnect_vpn
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
