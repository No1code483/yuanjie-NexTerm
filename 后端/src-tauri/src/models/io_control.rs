use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub max_output_bytes: usize,
    pub chunk_size: usize,
    pub timeout_ms: u64,
    pub stream_enabled: bool,
    pub separate_streams: bool,
    pub truncation_notice: bool,
    pub notify_on_truncate: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            max_output_bytes: 100_000,
            chunk_size: 4096,
            timeout_ms: 30_000,
            stream_enabled: true,
            separate_streams: true,
            truncation_notice: true,
            notify_on_truncate: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputStream {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDelta {
    pub stream: OutputStream,
    pub data: String,
    pub byte_offset: usize,
    pub timestamp_ms: i64,
    pub is_last: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDeltaEvent {
    pub execution_id: String,
    pub delta: OutputDelta,
    pub total_bytes_read: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamedOutput {
    pub execution_id: String,
    pub deltas: Vec<OutputDelta>,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub stdout_total_bytes: usize,
    pub stderr_total_bytes: usize,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub timed_out: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteWithStreamRequest {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub stdin: Option<String>,
    pub env: Option<Vec<(String, String)>>,
    pub config: Option<OutputConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CappedReadResult {
    pub data: String,
    pub bytes_read: usize,
    pub truncated: bool,
    pub stream: OutputStream,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoControlStats {
    pub total_executions: usize,
    pub total_bytes_processed: u64,
    pub total_truncations: usize,
    pub avg_duration_ms: f64,
    pub max_bytes_ever: usize,
}

impl IoControlStats {
    pub fn new() -> Self {
        Self {
            total_executions: 0,
            total_bytes_processed: 0,
            total_truncations: 0,
            avg_duration_ms: 0.0,
            max_bytes_ever: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoKillRequest {
    pub execution_id: String,
    #[serde(default)]
    pub signal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoStdinRequest {
    pub execution_id: String,
    pub data: String,
}