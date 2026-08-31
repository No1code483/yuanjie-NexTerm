use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc;
use tokio::time::Instant;
use uuid::Uuid;

use crate::error::app_error::AppError;
use crate::models::io_control::{
    ExecuteWithStreamRequest, IoControlStats, OutputConfig, OutputDelta, OutputDeltaEvent,
    OutputStream, StreamedOutput,
};

struct ActiveExecution {
    cancel_tx: mpsc::Sender<()>,
    child_handle: Option<tokio::process::Child>,
    stdin_tx: Option<mpsc::Sender<Vec<u8>>>,
}

pub struct IoControlManager {
    config: OutputConfig,
    stats: IoControlStats,
    active_executions: HashMap<String, ActiveExecution>,
}

impl IoControlManager {
    pub fn new(config: Option<OutputConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            stats: IoControlStats::new(),
            active_executions: HashMap::new(),
        }
    }

    pub fn update_config(&mut self, config: OutputConfig) {
        self.config = config;
    }

    pub fn get_config(&self) -> &OutputConfig {
        &self.config
    }

    pub fn get_stats(&self) -> &IoControlStats {
        &self.stats
    }

    pub async fn execute_with_stream(
        &mut self,
        req: ExecuteWithStreamRequest,
        delta_sender: mpsc::Sender<OutputDeltaEvent>,
    ) -> Result<StreamedOutput, AppError> {
        let config = req.config.unwrap_or_else(|| self.config.clone());
        let execution_id = Uuid::new_v4().to_string();
        let start = Instant::now();

        let (cancel_tx, cancel_rx) = mpsc::channel::<()>(1);
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(8);
        self.active_executions
            .insert(execution_id.clone(), ActiveExecution {
                cancel_tx,
                child_handle: None,
                stdin_tx: Some(stdin_tx),
            });

        let mut cmd = TokioCommand::new(&req.command);
        cmd.args(&req.args);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(dir) = &req.working_dir {
            cmd.current_dir(dir);
        }
        if let Some(env) = &req.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Internal(format!("启动进程失败: {}", e)))?;

        // Store child handle for kill support
        if let Some(exec) = self.active_executions.get_mut(&execution_id) {
            exec.child_handle = Some(child);
        }

        // Spawn stdin feeding task
        let child_stdin = if let Some(exec) = self.active_executions.get_mut(&execution_id) {
            exec.child_handle.as_mut().and_then(|c| c.stdin.take())
        } else {
            None
        };
        if let Some(mut child_stdin) = child_stdin {
            let exec_id = execution_id.clone();
            tokio::spawn(async move {
                while let Some(data) = stdin_rx.recv().await {
                    if child_stdin.write_all(&data).await.is_err() {
                        break;
                    }
                }
                let _ = exec_id;
            });
        }

        if let Some(stdin_data) = &req.stdin {
            if let Some(exec) = self.active_executions.get(&execution_id) {
                if let Some(stdin_tx) = &exec.stdin_tx {
                    let _ = stdin_tx.send(stdin_data.as_bytes().to_vec()).await;
                }
            }
        }

        let child_stdout = if let Some(exec) = self.active_executions.get_mut(&execution_id) {
            exec.child_handle.as_mut().and_then(|c| c.stdout.take())
        } else {
            None
        };
        let child_stderr = if let Some(exec) = self.active_executions.get_mut(&execution_id) {
            exec.child_handle.as_mut().and_then(|c| c.stderr.take())
        } else {
            None
        };

        let max_bytes = config.max_output_bytes;
        let chunk_size = config.chunk_size;
        let timeout_ms = config.timeout_ms;
        let separate_streams = config.separate_streams;
        let exec_id = execution_id.clone();
        let sender = delta_sender.clone();

        let (stdout_total, stderr_total, stdout_trunc, stderr_trunc) = if separate_streams {
            let (stdout_tx, mut stdout_rx) = mpsc::channel::<Vec<u8>>(32);
            let (stderr_tx, mut stderr_rx) = mpsc::channel::<Vec<u8>>(32);

            let stdout_task = tokio::spawn(read_stream_capped(
                child_stdout,
                max_bytes,
                chunk_size,
                stdout_tx,
                cancel_rx,
            ));
            let stderr_cancel_rx = mpsc::channel::<()>(1).1;
            let stderr_task = tokio::spawn(read_stream_capped(
                child_stderr,
                max_bytes,
                chunk_size,
                stderr_tx,
                stderr_cancel_rx,
            ));

            let mut stdout_total = 0usize;
            let mut stderr_total = 0usize;
            let mut stdout_trunc = false;
            let mut stderr_trunc = false;

            let mut stdout_offset = 0usize;
            let mut stderr_offset = 0usize;

            let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);

            if let Ok(Ok((bytes, trunc))) = stdout_result {
                stdout_total = bytes;
                stdout_trunc = trunc;
            }

            if let Ok(Ok((bytes, trunc))) = stderr_result {
                stderr_total = bytes;
                stderr_trunc = trunc;
            }

            while let Ok(chunk) = stdout_rx.try_recv() {
                let data = String::from_utf8_lossy(&chunk).to_string();
                let byte_len = chunk.len();
                stdout_offset += byte_len;
                let _ = sender
                    .send(OutputDeltaEvent {
                        execution_id: exec_id.clone(),
                        delta: OutputDelta {
                            stream: OutputStream::Stdout,
                            data,
                            byte_offset: stdout_offset,
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_last: false,
                        },
                        total_bytes_read: (stdout_offset + stderr_offset) as u64,
                        truncated: stdout_trunc || stderr_trunc,
                    })
                    .await;
            }

            while let Ok(chunk) = stderr_rx.try_recv() {
                let data = String::from_utf8_lossy(&chunk).to_string();
                let byte_len = chunk.len();
                stderr_offset += byte_len;
                let _ = sender
                    .send(OutputDeltaEvent {
                        execution_id: exec_id.clone(),
                        delta: OutputDelta {
                            stream: OutputStream::Stderr,
                            data,
                            byte_offset: stderr_offset,
                            timestamp_ms: chrono::Utc::now().timestamp_millis(),
                            is_last: false,
                        },
                        total_bytes_read: (stdout_offset + stderr_offset) as u64,
                        truncated: stdout_trunc || stderr_trunc,
                    })
                    .await;
            }

            (stdout_total, stderr_total, stdout_trunc, stderr_trunc)
        } else {
            (0, 0, false, false)
        };

        let is_timed_out;
        let exit_code = {
            let child = self.active_executions.get_mut(&execution_id)
                .and_then(|exec| exec.child_handle.as_mut());
            match child {
                None => {
                    is_timed_out = false;
                    -1
                }
                Some(child) => {
                    is_timed_out = tokio::time::timeout(
                        std::time::Duration::from_millis(timeout_ms),
                        child.wait(),
                    )
                    .await
                    .is_err();
                    if is_timed_out {
                        let _ = child.kill().await;
                        -1
                    } else {
                        child
                            .wait()
                            .await
                            .map(|s| s.code().unwrap_or(-1))
                            .unwrap_or(-1)
                    }
                }
            }
        };

        let _ = sender
            .send(OutputDeltaEvent {
                execution_id: exec_id.clone(),
                delta: OutputDelta {
                    stream: OutputStream::Stdout,
                    data: String::new(),
                    byte_offset: 0,
                    timestamp_ms: chrono::Utc::now().timestamp_millis(),
                    is_last: true,
                },
                total_bytes_read: (stdout_total + stderr_total) as u64,
                truncated: stdout_trunc || stderr_trunc,
            })
            .await;

        self.active_executions.remove(&execution_id);

        let duration_ms = start.elapsed().as_millis() as u64;

        self.stats.total_executions += 1;
        self.stats.total_bytes_processed += (stdout_total + stderr_total) as u64;
        if stdout_trunc || stderr_trunc {
            self.stats.total_truncations += 1;
        }

        let prev_total = self.stats.avg_duration_ms * (self.stats.total_executions - 1) as f64;
        self.stats.avg_duration_ms =
            (prev_total + duration_ms as f64) / self.stats.total_executions as f64;

        if (stdout_total + stderr_total) > self.stats.max_bytes_ever {
            self.stats.max_bytes_ever = stdout_total + stderr_total;
        }

        Ok(StreamedOutput {
            execution_id,
            deltas: Vec::new(),
            exit_code,
            duration_ms,
            stdout_total_bytes: stdout_total,
            stderr_total_bytes: stderr_total,
            stdout_truncated: stdout_trunc,
            stderr_truncated: stderr_trunc,
            timed_out: is_timed_out,
            truncated: stdout_trunc || stderr_trunc,
        })
    }

    pub fn cancel_execution(&mut self, execution_id: &str) -> bool {
        if let Some(exec) = self.active_executions.remove(execution_id) {
            let _ = exec.cancel_tx.try_send(());
            true
        } else {
            false
        }
    }

    pub async fn kill_execution(&mut self, execution_id: &str) -> bool {
        if let Some(exec) = self.active_executions.get_mut(execution_id) {
            let _ = exec.cancel_tx.try_send(());
            if let Some(ref mut child) = exec.child_handle {
                let _ = child.kill().await;
            }
            true
        } else {
            false
        }
    }

    pub async fn send_stdin(&self, execution_id: &str, data: &str) -> bool {
        if let Some(exec) = self.active_executions.get(execution_id) {
            if let Some(ref stdin_tx) = exec.stdin_tx {
                let mut bytes = data.as_bytes().to_vec();
                if !bytes.ends_with(b"\n") {
                    bytes.push(b'\n');
                }
                return stdin_tx.try_send(bytes).is_ok();
            }
        }
        false
    }

    pub fn cancel_all(&mut self) -> usize {
        let count = self.active_executions.len();
        for (_, exec) in self.active_executions.drain() {
            let _ = exec.cancel_tx.try_send(());
        }
        count
    }
}

async fn read_stream_capped(
    stream: Option<impl AsyncReadExt + Unpin>,
    max_bytes: usize,
    chunk_size: usize,
    sender: mpsc::Sender<Vec<u8>>,
    mut cancel: mpsc::Receiver<()>,
) -> Result<(usize, bool), AppError> {
    let mut reader = match stream {
        Some(r) => r,
        None => return Ok((0, false)),
    };

    let mut total_read = 0usize;
    let mut truncated = false;
    let mut buf = vec![0u8; chunk_size];

    loop {
        tokio::select! {
            _ = cancel.recv() => {
                return Ok((total_read, truncated));
            }
            read_result = reader.read(&mut buf) => {
                match read_result {
                    Ok(0) => break,
                    Ok(n) => {
                        let remaining = max_bytes.saturating_sub(total_read);
                        if remaining == 0 {
                            truncated = true;
                            continue;
                        }
                        let to_read = n.min(remaining);
                        let chunk = buf[..to_read].to_vec();
                        total_read += to_read;
                        if to_read < n {
                            truncated = true;
                        }
                        if sender.send(chunk).await.is_err() {
                            break;
                        }
                        if truncated {
                            continue;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    Ok((total_read, truncated))
}