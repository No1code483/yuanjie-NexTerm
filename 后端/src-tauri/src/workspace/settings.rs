//! 工作区级别设置

use serde::{Deserialize, Serialize};

/// 工作区设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// 编辑器设置
    pub editor: EditorSettings,
    /// 文件设置
    pub files: FileSettings,
    /// 搜索设置
    pub search: SearchSettings,
    /// 终端设置
    pub terminal: TerminalSettings,
}

/// 编辑器设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    /// 字体大小
    pub font_size: u16,
    /// 字体族
    pub font_family: String,
    /// 行高
    pub line_height: f32,
    /// Tab 大小
    pub tab_size: u8,
    /// 是否使用空格缩进
    pub insert_spaces: bool,
    /// 是否显示行号
    pub line_numbers: bool,
    /// 是否显示 minimap
    pub minimap: bool,
    /// 是否自动换行
    pub word_wrap: bool,
    /// 是否启用代码折叠
    pub folding: bool,
    /// 是否启用括号匹配
    pub bracket_matching: bool,
    /// 主题
    pub theme: String,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 14,
            font_family: "Consolas, 'Courier New', monospace".into(),
            line_height: 1.5,
            tab_size: 4,
            insert_spaces: true,
            line_numbers: true,
            minimap: true,
            word_wrap: false,
            folding: true,
            bracket_matching: true,
            theme: "vs-dark".into(),
        }
    }
}

/// 文件设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSettings {
    /// 默认编码
    pub encoding: String,
    /// 自动保存延迟 (ms)
    pub auto_save_delay_ms: u64,
    /// 排除模式
    pub exclude_patterns: Vec<String>,
    /// 文件监视排除
    pub watcher_exclude: Vec<String>,
    /// 最大文件大小 (bytes)
    pub max_file_size: u64,
}

impl Default for FileSettings {
    fn default() -> Self {
        Self {
            encoding: "utf-8".into(),
            auto_save_delay_ms: 1000,
            exclude_patterns: vec![
                "**/node_modules/**".into(),
                "**/target/**".into(),
                "**/.git/**".into(),
                "**/dist/**".into(),
                "**/build/**".into(),
            ],
            watcher_exclude: vec![
                "**/node_modules/**".into(),
                "**/.git/objects/**".into(),
                "**/.git/subtree-cache/**".into(),
            ],
            max_file_size: 50 * 1024 * 1024, // 50 MB
        }
    }
}

/// 搜索设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    /// 是否使用正则
    pub use_regex: bool,
    /// 是否区分大小写
    pub case_sensitive: bool,
    /// 是否全词匹配
    pub whole_word: bool,
    /// 是否包含子目录
    pub include_subdirs: bool,
    /// 最大结果数
    pub max_results: u32,
    /// 排除模式
    pub exclude_patterns: Vec<String>,
}

impl Default for SearchSettings {
    fn default() -> Self {
        Self {
            use_regex: false,
            case_sensitive: false,
            whole_word: false,
            include_subdirs: true,
            max_results: 10000,
            exclude_patterns: vec![
                "**/node_modules/**".into(),
                "**/target/**".into(),
                "**/.git/**".into(),
            ],
        }
    }
}

/// 终端设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSettings {
    /// Shell 路径
    pub shell_path: String,
    /// Shell 参数
    pub shell_args: Vec<String>,
    /// 字体大小
    pub font_size: u16,
    /// 光标样式
    pub cursor_style: String,
    /// 滚动行数
    pub scrollback: u32,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            shell_path: if cfg!(windows) {
                "powershell.exe".into()
            } else {
                "/bin/bash".into()
            },
            shell_args: Vec::new(),
            font_size: 14,
            cursor_style: "block".into(),
            scrollback: 5000,
        }
    }
}

impl Default for WorkspaceSettings {
    fn default() -> Self {
        Self {
            editor: EditorSettings::default(),
            files: FileSettings::default(),
            search: SearchSettings::default(),
            terminal: TerminalSettings::default(),
        }
    }
}