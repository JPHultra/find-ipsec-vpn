use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::mpsc::{channel, Sender};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum GuiCommand {
    Connect {
        host: String,
        remote_identity: String,
        username: String,
        psk: String,
        password: String,
    },
    SubmitOtp {
        otp: String,
    },
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum HelperMessage {
    Status {
        state: String,
        message: String,
    },
    TunnelInfo {
        vpn_ip: String,
        gateway_ip: String,
        protocol: String,
        encryption: String,
    },
    Stats {
        bytes_sent: u64,
        bytes_received: u64,
        uptime_secs: u64,
    },
    Log {
        message: String,
    },
    Error {
        message: String,
    },
}

fn send_msg(msg: &HelperMessage) {
    if let Ok(json) = serde_json::to_string(msg) {
        println!("{}", json);
        let _ = io::stdout().flush();
    }
}

// POSIX PTY allocation
struct Pty {
    master: File,
    slave_fd: RawFd,
}

fn create_pty() -> io::Result<Pty> {
    unsafe {
        let master_fd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
        if master_fd < 0 {
            return Err(io::Error::last_os_error());
        }
        if libc::grantpt(master_fd) < 0 {
            libc::close(master_fd);
            return Err(io::Error::last_os_error());
        }
        if libc::unlockpt(master_fd) < 0 {
            libc::close(master_fd);
            return Err(io::Error::last_os_error());
        }
        let slave_name_ptr = libc::ptsname(master_fd);
        if slave_name_ptr.is_null() {
            libc::close(master_fd);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to get PTY slave name"));
        }
        let slave_name = std::ffi::CStr::from_ptr(slave_name_ptr).to_owned();
        let slave_fd = libc::open(slave_name.as_ptr(), libc::O_RDWR);
        if slave_fd < 0 {
            libc::close(master_fd);
            return Err(io::Error::last_os_error());
        }
        
        Ok(Pty {
            master: File::from_raw_fd(master_fd),
            slave_fd,
        })
    }
}

fn parse_xfrm_stats(output: &str, gateway_ip: &str) -> (u64, u64) {
    let mut bytes_sent = 0;
    let mut bytes_received = 0;
    
    // Split by "src " to parse each SA entry
    let sas = output.split("src ");
    for sa in sas {
        if sa.is_empty() {
            continue;
        }
        
        let lines: Vec<&str> = sa.lines().map(|l| l.trim()).collect();
        if lines.is_empty() {
            continue;
        }
        
        let first_line = lines[0];
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 3 || parts[1] != "dst" {
            continue;
        }
        
        let src_ip = parts[0];
        let dst_ip = parts[2];
        
        // Outbound SA: destination is VPN gateway IP
        let is_outbound = dst_ip == gateway_ip;
        // Inbound SA: source is VPN gateway IP
        let is_inbound = src_ip == gateway_ip;
        
        if !is_outbound && !is_inbound {
            continue;
        }
        
        for i in 0..lines.len() {
            if lines[i].starts_with("lifetime current:") && i + 1 < lines.len() {
                let bytes_line = lines[i + 1];
                if let Some(bytes_part) = bytes_line.split("bytes ").nth(1) {
                    if let Some(num_str) = bytes_part.split(',').next() {
                        if let Ok(bytes) = num_str.trim().parse::<u64>() {
                            if is_outbound {
                                bytes_sent += bytes;
                            } else {
                                bytes_received += bytes;
                            }
                        }
                    }
                }
            }
        }
    }
    
    (bytes_sent, bytes_received)
}

struct VpnConnection {
    child: Child,
    _master: File,
    gateway_ip: String,
    state: Arc<Mutex<String>>,
}

fn main() {
    let (tx_cmd, rx_cmd) = channel::<GuiCommand>();
    
    // Stdin reading thread
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if let Ok(cmd) = serde_json::from_str::<GuiCommand>(&line) {
                if tx_cmd.send(cmd).is_err() {
                    break;
                }
            }
        }
        // If stdin EOF happens, trigger auto-disconnect
        let _ = tx_cmd.send(GuiCommand::Disconnect);
    });

    let mut current_conn: Option<VpnConnection> = None;
    let mut vpn_ip: Option<String> = None;
    let mut connect_time: Option<Instant> = None;
    
    // Active session OTP channel sender
    let mut current_tx_otp: Option<Sender<String>> = None;

    loop {
        // Handle incoming commands
        if let Ok(cmd) = rx_cmd.try_recv() {
            match cmd {
                GuiCommand::Connect { host, remote_identity, username, psk, password } => {
                    if current_conn.is_some() {
                        send_msg(&HelperMessage::Error { message: "VPN is already running".to_string() });
                        continue;
                    }
                    
                    send_msg(&HelperMessage::Status {
                        state: "Resolving".to_string(),
                        message: format!("Resolving and initiating connection to {}", host),
                    });
                    
                    // Stop background services to free up ports 500/4500
                    let _ = Command::new("systemctl").arg("stop").arg("strongswan").status();
                    let _ = Command::new("systemctl").arg("stop").arg("strongswan-starter").status();
                    
                    // Allocate PTY
                    let pty = match create_pty() {
                        Ok(p) => p,
                        Err(e) => {
                            send_msg(&HelperMessage::Error { message: format!("Failed to create PTY: {}", e) });
                            continue;
                        }
                    };
                    
                    // Setup child process
                    let mut cmd = Command::new("charon-cmd");
                    cmd.arg("--host").arg(&host)
                       .arg("--identity").arg("%any")
                       .arg("--remote-identity").arg(&remote_identity)
                       .arg("--xauth-username").arg(&username)
                       .arg("--profile").arg("ikev1-xauth-psk-am")
                       .arg("--ike-proposal").arg("aes128-sha256-modp2048")
                       .arg("--remote-ts").arg("0.0.0.0/0")
                       .arg("--debug").arg("1");
                    
                    unsafe {
                        let slave_read_fd = libc::dup(pty.slave_fd);
                        let slave_write_fd = libc::dup(pty.slave_fd);
                        let slave_err_fd = libc::dup(pty.slave_fd);
                        
                        cmd.stdin(Stdio::from_raw_fd(slave_read_fd));
                        cmd.stdout(Stdio::from_raw_fd(slave_write_fd));
                        cmd.stderr(Stdio::from_raw_fd(slave_err_fd));
                        
                        cmd.pre_exec(move || {
                            libc::setsid();
                            if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                                return Err(io::Error::last_os_error());
                            }
                            // Disable local terminal echoing
                            let mut t: libc::termios = std::mem::zeroed();
                            if libc::tcgetattr(0, &mut t) == 0 {
                                t.c_lflag &= !libc::ECHO;
                                libc::tcsetattr(0, libc::TCSANOW, &t);
                            }
                            Ok(())
                        });
                    }
                    
                    // Close our copy of the slave fd since child inherits it
                    unsafe { libc::close(pty.slave_fd); }
                    
                    let child = match cmd.spawn() {
                        Ok(c) => c,
                        Err(e) => {
                            send_msg(&HelperMessage::Error { message: format!("Failed to spawn charon-cmd: {}", e) });
                            continue;
                        }
                    };
                    
                    let state = Arc::new(Mutex::new("Connecting".to_string()));
                    let pty_master_clone = match pty.master.try_clone() {
                        Ok(c) => c,
                        Err(e) => {
                            send_msg(&HelperMessage::Error { message: format!("Failed to clone master PTY fd: {}", e) });
                            continue;
                        }
                    };
                    
                    let (tx_otp_sess, rx_otp_sess) = channel::<String>();
                    current_tx_otp = Some(tx_otp_sess);
                    
                    let state_clone = Arc::clone(&state);
                    
                    vpn_ip = None;
                    connect_time = None;
                    
                    // Master PTY reader thread
                    thread::spawn(move || {
                        let mut reader = pty_master_clone;
                        let mut buffer = Vec::new();
                        let mut chunk = [0u8; 1024];
                        
                        let psk_secret = psk;
                        let pwd_secret = password;
                        
                        loop {
                            let bytes_read = match reader.read(&mut chunk) {
                                Ok(0) => break, // EOF
                                Ok(n) => n,
                                Err(_) => break,
                            };
                            
                            buffer.extend_from_slice(&chunk[..bytes_read]);
                            
                            // Process full lines
                            while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                                let line_bytes = buffer.drain(..=pos).collect::<Vec<u8>>();
                                let line = String::from_utf8_lossy(&line_bytes);
                                let trimmed = line.trim();
                                if !trimmed.is_empty() {
                                    send_msg(&HelperMessage::Log { message: trimmed.to_string() });
                                    
                                    // Parse for virtual IP
                                    if let Some(vip_pos) = trimmed.find("installing new virtual IP ") {
                                        let vip_part = &trimmed[vip_pos + "installing new virtual IP ".len()..];
                                        let ip = vip_part.split_whitespace().next().unwrap_or("").to_string();
                                        if !ip.is_empty() {
                                            // Send event that we parsed the IP
                                            send_msg(&HelperMessage::Log { message: format!("Parsed virtual IP: {}", ip) });
                                        }
                                    }
                                }
                            }
                            
                            // Search for non-newline credential prompts in buffer
                            let current_str = String::from_utf8_lossy(&buffer);
                            
                            if current_str.ends_with("Preshared Key: ") {
                                buffer.clear();
                                let _ = reader.write_all(format!("{}\n", psk_secret).as_bytes());
                                let _ = reader.flush();
                                *state_clone.lock().unwrap() = "Authenticating".to_string();
                                send_msg(&HelperMessage::Status {
                                    state: "Authenticating".to_string(),
                                    message: "Sending pre-shared key...".to_string(),
                                });
                            } else if current_str.ends_with("Password: ") || current_str.ends_with("EAP password: ") {
                                buffer.clear();
                                let _ = reader.write_all(format!("{}\n", pwd_secret).as_bytes());
                                let _ = reader.flush();
                                *state_clone.lock().unwrap() = "Authenticating".to_string();
                                send_msg(&HelperMessage::Status {
                                    state: "Authenticating".to_string(),
                                    message: "Sending XAuth password...".to_string(),
                                });
                            } else if current_str.ends_with("PIN: ") {
                                buffer.clear();
                                *state_clone.lock().unwrap() = "WaitingForOtp".to_string();
                                send_msg(&HelperMessage::Status {
                                    state: "WaitingForOtp".to_string(),
                                    message: "FortiGate challenges for separate Email Verification Code".to_string(),
                                });
                                
                                // Block waiting for the OTP code from the GUI
                                if let Ok(otp_code) = rx_otp_sess.recv() {
                                    let _ = reader.write_all(format!("{}\n", otp_code).as_bytes());
                                    let _ = reader.flush();
                                    *state_clone.lock().unwrap() = "EstablishingTunnel".to_string();
                                    send_msg(&HelperMessage::Status {
                                        state: "EstablishingTunnel".to_string(),
                                        message: "Submitting Verification Code...".to_string(),
                                    });
                                }
                            }
                        }
                        
                        *state_clone.lock().unwrap() = "Disconnected".to_string();
                        send_msg(&HelperMessage::Status {
                            state: "Disconnected".to_string(),
                            message: "Tunnel process exited".to_string(),
                        });
                    });
                    
                    current_conn = Some(VpnConnection {
                        child,
                        _master: pty.master,
                        gateway_ip: remote_identity,
                        state,
                    });
                }
                
                GuiCommand::SubmitOtp { otp } => {
                    if let Some(ref tx) = current_tx_otp {
                        let _ = tx.send(otp);
                    }
                }
                
                GuiCommand::Disconnect => {
                    current_tx_otp = None;
                    if let Some(mut conn) = current_conn.take() {
                        send_msg(&HelperMessage::Status {
                            state: "Disconnected".to_string(),
                            message: "Disconnecting VPN...".to_string(),
                        });
                        let _ = conn.child.kill();
                        let _ = conn.child.wait();
                    }
                    vpn_ip = None;
                    connect_time = None;
                }
            }
        }
        
        // Monitor existing connection status and traffic stats
        if let Some(ref mut conn) = current_conn {
            let state = {
                let s = conn.state.lock().unwrap();
                s.clone()
            };
            
            // Check if process has died unexpectedly
            match conn.child.try_wait() {
                Ok(Some(status)) => {
                    send_msg(&HelperMessage::Log {
                        message: format!("charon-cmd exited with status: {}", status),
                    });
                    current_conn = None;
                    vpn_ip = None;
                    connect_time = None;
                    current_tx_otp = None;
                    send_msg(&HelperMessage::Status {
                        state: "Disconnected".to_string(),
                        message: format!("VPN process disconnected: {}", status),
                    });
                    continue;
                }
                Err(e) => {
                    send_msg(&HelperMessage::Error {
                        message: format!("Error polling child process: {}", e),
                    });
                }
                _ => {}
            }
            
            // Query XFRM policies & state to verify active tunnel
            let xfrm_output = Command::new("ip")
                .arg("-s")
                .arg("xfrm")
                .arg("state")
                .output();
                
            let is_connected_in_kernel = if let Ok(out) = xfrm_output {
                let stdout_str = String::from_utf8_lossy(&out.stdout);
                
                // Parse stats
                let (sent, recv) = parse_xfrm_stats(&stdout_str, &conn.gateway_ip);
                
                // Check if any policy exists for our gateway to declare "Connected"
                let has_sa = stdout_str.contains(&conn.gateway_ip);
                
                if has_sa {
                    // Try to parse virtual IP if not found yet
                    if vpn_ip.is_none() {
                        // We can extract virtual IP from xfrm state sel src
                        // e.g. "sel src 10.7.1.11/32 dst 0.0.0.0/0"
                        if let Some(sel_src_idx) = stdout_str.find("sel src ") {
                            let sub = &stdout_str[sel_src_idx + "sel src ".len()..];
                            if let Some(ip_only) = sub.split('/').next() {
                                let ip = ip_only.trim().to_string();
                                if !ip.is_empty() && ip != "0.0.0.0" {
                                    vpn_ip = Some(ip.clone());
                                    connect_time = Some(Instant::now());
                                    
                                    send_msg(&HelperMessage::TunnelInfo {
                                        vpn_ip: ip,
                                        gateway_ip: conn.gateway_ip.clone(),
                                        protocol: "IPsec ESP / NAT-T".to_string(),
                                        encryption: "AES_CBC_128 / SHA256".to_string(),
                                    });
                                    
                                    send_msg(&HelperMessage::Status {
                                        state: "Connected".to_string(),
                                        message: "VPN connection successfully established!".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    
                    if let Some(time) = connect_time {
                        send_msg(&HelperMessage::Stats {
                            bytes_sent: sent,
                            bytes_received: recv,
                            uptime_secs: time.elapsed().as_secs(),
                        });
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            };
            
            // If the kernel state disappeared and we were establishing/connected, trigger disconnect
            if !is_connected_in_kernel && (state == "Connected" || state == "EstablishingTunnel") {
                send_msg(&HelperMessage::Status {
                    state: "Disconnected".to_string(),
                    message: "IPsec security association lost".to_string(),
                });
                let _ = conn.child.kill();
                let _ = conn.child.wait();
                current_conn = None;
                vpn_ip = None;
                connect_time = None;
                current_tx_otp = None;
            }
        }
        
        thread::sleep(Duration::from_millis(1500));
    }
}
