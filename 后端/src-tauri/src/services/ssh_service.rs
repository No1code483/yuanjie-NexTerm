use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use ssh2::{Channel, Session};
use tauri::{AppHandle, Emitter};
use tracing;

use crate::error::app_error::AppError;

pub struct SshSession {
    pub channel: Channel,
    pub session: Session,
    pub host: String,
    pub port: u16,
    pub username: String,
}

pub struct SshService {
    pub sessions: Mutex<HashMap<String, SshSession>>,
}

impl SshService {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// 建立 SSH 连接并打开 PTY 通道
    pub fn connect(
        &self,
        session_id: &str,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key_path: Option<&str>,
        _app_handle: &AppHandle,
    ) -> Result<(), AppError> {
        let tcp = std::net::TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| AppError::TerminalError(format!("SSH 连接失败: {}", e)))?;
        tcp.set_nonblocking(false)
            .map_err(|e| AppError::TerminalError(format!("set_nonblocking 失败: {}", e)))?;

        let mut session = Session::new()
            .map_err(|e| AppError::TerminalError(format!("创建 SSH 会话失败: {}", e)))?;
        session.set_tcp_stream(tcp);
        session
            .handshake()
            .map_err(|e| AppError::TerminalError(format!("SSH 握手失败: {}", e)))?;

        // 认证
        if let Some(pk_path) = private_key_path {
            session
                .userauth_pubkey_file(username, None, std::path::Path::new(pk_path), None)
                .map_err(|e| AppError::TerminalError(format!("SSH 密钥认证失败: {}", e)))?;
        } else if let Some(pwd) = password {
            session
                .userauth_password(username, pwd)
                .map_err(|e| AppError::TerminalError(format!("SSH 密码认证失败: {}", e)))?;
        } else {
            return Err(AppError::TerminalError("SSH 认证方式未指定".into()));
        }

        if !session.authenticated() {
            return Err(AppError::TerminalError("SSH 认证失败".into()));
        }

        let mut channel = session
            .channel_session()
            .map_err(|e| AppError::TerminalError(format!("创建 SSH 通道失败: {}", e)))?;
        channel
            .request_pty("xterm-256color", None, Some((80, 24, 0, 0)))
            .map_err(|e| AppError::TerminalError(format!("请求 PTY 失败: {}", e)))?;
        channel
            .shell()
            .map_err(|e| AppError::TerminalError(format!("启动 shell 失败: {}", e)))?;

        let ssh_session = SshSession {
            session,
            channel,
            host: host.to_string(),
            port,
            username: username.to_string(),
        };

        let mut sessions = self.sessions.lock().expect("ssh sessions lock failed");
        sessions.insert(session_id.to_string(), ssh_session);

        tracing::info!(session_id, host, port, username, "SSH 会话已建立");
        Ok(())
    }

    /// 写入数据到 SSH 通道
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().expect("ssh sessions lock failed");
        let ssh_session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::TerminalError("SSH 会话不存在".into()))?;

        ssh_session
            .channel
            .write_all(data)
            .map_err(|e| AppError::TerminalError(format!("SSH 写入失败: {}", e)))?;
        Ok(())
    }

    /// 调整 SSH PTY 尺寸
    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().expect("ssh sessions lock failed");
        let ssh_session = sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::TerminalError("SSH 会话不存在".into()))?;

        ssh_session
            .channel
            .request_pty_size(cols as u32, rows as u32, None, None)
            .map_err(|e| AppError::TerminalError(format!("SSH 调整 PTY 尺寸失败: {}", e)))?;
        tracing::info!(session_id, cols, rows, "SSH PTY 尺寸已调整");
        Ok(())
    }

    /// 断开 SSH 连接
    pub fn disconnect(&self, session_id: &str) -> Result<(), AppError> {
        let mut sessions = self.sessions.lock().expect("ssh sessions lock failed");
        if let Some(mut ssh_session) = sessions.remove(session_id) {
            let _ = ssh_session.channel.close();
            let _ = ssh_session.channel.wait_close();
            tracing::info!(session_id, "SSH 会话已断开");
        }
        Ok(())
    }

    /// 启动 SSH 输出读取循环（需要 Arc<SshService>）
    pub fn start_read_loop(
        self: &Arc<Self>,
        session_id: String,
        app_handle: AppHandle,
    ) {
        let service = Arc::clone(self);
        let sid = session_id.clone();

        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                let mut sessions = match service.sessions.lock() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let ssh_session = match sessions.get_mut(&sid) {
                    Some(s) => s,
                    None => break,
                };

                match ssh_session.channel.read(&mut buf) {
                    Ok(0) => {
                        tracing::info!(sid, "SSH 通道已关闭");
                        drop(sessions);
                        break;
                    }
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        drop(sessions);
                        let _ = app_handle.emit(
                            "terminal-output",
                            serde_json::json!({
                                "session_id": sid,
                                "data": String::from_utf8_lossy(&data)
                            }),
                        );
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        drop(sessions);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        tracing::error!(sid, "SSH 读取错误: {}", e);
                        drop(sessions);
                        break;
                    }
                }
            }
            // 清理
            let _ = service.disconnect(&sid);
        });
    }
}