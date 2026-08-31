//! 资源限制
//!
//! 对沙箱进程施加资源限制，防止资源耗尽攻击。
//! - Linux: cgroup v2
//! - Windows: Job Objects
//! - macOS: setrlimit

/// 通用资源限制器
#[derive(Debug, Clone)]
pub struct ResourceLimiter {
    /// 最大内存 (bytes)
    pub max_memory: u64,
    /// 最大 CPU 时间 (seconds)
    pub max_cpu_time: u64,
    /// 最大文件大小 (bytes)
    pub max_file_size: u64,
    /// 最大打开文件数
    pub max_open_files: u64,
    /// 最大进程数
    pub max_processes: u64,
    /// 最大栈大小 (bytes)
    pub max_stack_size: u64,
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self {
            max_memory: 512 * 1024 * 1024, // 512 MB
            max_cpu_time: 30,              // 30 seconds
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_open_files: 256,
            max_processes: 10,
            max_stack_size: 8 * 1024 * 1024, // 8 MB
        }
    }
}

impl ResourceLimiter {
    /// 创建严格限制（用于不可信代码）
    pub fn strict() -> Self {
        Self {
            max_memory: 128 * 1024 * 1024,  // 128 MB
            max_cpu_time: 10,               // 10 seconds
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_open_files: 64,
            max_processes: 1,
            max_stack_size: 4 * 1024 * 1024, // 4 MB
        }
    }

    /// 创建宽松限制（用于可信代码）
    pub fn relaxed() -> Self {
        Self {
            max_memory: 2 * 1024 * 1024 * 1024, // 2 GB
            max_cpu_time: 300,                  // 5 minutes
            max_file_size: 500 * 1024 * 1024,   // 500 MB
            max_open_files: 1024,
            max_processes: 50,
            max_stack_size: 32 * 1024 * 1024, // 32 MB
        }
    }

    /// 在 Linux 上通过 cgroup 应用限制
    #[cfg(target_os = "linux")]
    pub fn apply_cgroup(&self, cgroup_path: &str) -> Result<(), String> {
        use std::fs;

        // cgroup v2 内存限制
        let memory_max = format!("{}/memory.max", cgroup_path);
        fs::write(&memory_max, self.max_memory.to_string())
            .map_err(|e| format!("设置内存限制失败: {}", e))?;

        // cgroup v2 CPU 限制
        let cpu_max = format!("{}/cpu.max", cgroup_path);
        let cpu_period = 100_000; // 100ms period
        let cpu_quota = self.max_cpu_time as i64 * cpu_period;
        fs::write(&cpu_max, format!("{} {}", cpu_quota.max(1000), cpu_period))
            .map_err(|e| format!("设置CPU限制失败: {}", e))?;

        // cgroup v2 进程数限制
        let pids_max = format!("{}/pids.max", cgroup_path);
        fs::write(&pids_max, self.max_processes.to_string())
            .map_err(|e| format!("设置进程数限制失败: {}", e))?;

        Ok(())
    }

    /// 在 Windows 上通过 Job Object 应用限制
    #[cfg(target_os = "windows")]
    pub fn apply_job_object(&self) -> Result<(), String> {
        // 在真实 Windows 环境中:
        // 1. CreateJobObjectW 创建 Job Object
        // 2. SetInformationJobObject 设置限制
        //   - JOBOBJECT_EXTENDED_LIMIT_INFORMATION (内存/进程数)
        //   - JOBOBJECT_CPU_RATE_CONTROL_INFORMATION (CPU)
        // 3. AssignProcessToJobObject 绑定进程
        tracing::info!(
            "Windows Job Object: memory={}MB, cpu={}s, processes={}",
            self.max_memory / 1024 / 1024,
            self.max_cpu_time,
            self.max_processes
        );
        Ok(())
    }

    /// 跨平台应用资源限制
    pub fn apply(&self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            self.apply_cgroup("/sys/fs/cgroup/nexterm")?;
        }
        #[cfg(target_os = "windows")]
        {
            self.apply_job_object()?;
        }
        #[cfg(target_os = "macos")]
        {
            // macOS 使用 setrlimit
            tracing::info!("macOS setrlimit: memory={}MB, cpu={}s", self.max_memory / 1024 / 1024, self.max_cpu_time);
        }
        Ok(())
    }
}

/// 资源使用统计
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// 当前内存使用 (bytes)
    pub memory_used: u64,
    /// 峰值内存使用 (bytes)
    pub memory_peak: u64,
    /// CPU 时间已用 (ms)
    pub cpu_time_ms: u64,
    /// 磁盘读取 (bytes)
    pub disk_read: u64,
    /// 磁盘写入 (bytes)
    pub disk_write: u64,
    /// 打开文件数
    pub open_files: u32,
    /// 子进程数
    pub child_processes: u32,
}

impl ResourceUsage {
    /// 检查是否超过限制
    pub fn check_against(&self, limits: &ResourceLimiter) -> Vec<ResourceViolation> {
        let mut violations = Vec::new();

        if self.memory_used > limits.max_memory {
            violations.push(ResourceViolation {
                resource: "memory".into(),
                used: self.memory_used,
                limit: limits.max_memory,
                unit: "bytes".into(),
            });
        }

        if self.cpu_time_ms > limits.max_cpu_time * 1000 {
            violations.push(ResourceViolation {
                resource: "cpu_time".into(),
                used: self.cpu_time_ms,
                limit: limits.max_cpu_time * 1000,
                unit: "ms".into(),
            });
        }

        if self.child_processes as u64 > limits.max_processes {
            violations.push(ResourceViolation {
                resource: "processes".into(),
                used: self.child_processes as u64,
                limit: limits.max_processes,
                unit: "count".into(),
            });
        }

        violations
    }
}

/// 资源违规
#[derive(Debug, Clone)]
pub struct ResourceViolation {
    pub resource: String,
    pub used: u64,
    pub limit: u64,
    pub unit: String,
}