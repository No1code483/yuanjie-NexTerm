//! 主题系统 — 对标 VSCode Theme
//!
//! 支持自定义编辑器主题、多主题切换、主题导入/导出。

use serde::{Deserialize, Serialize};

/// 编辑器主题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorTheme {
    /// 主题 ID
    pub id: String,
    /// 主题名称
    pub name: String,
    /// 主题类型 (dark/light)
    pub theme_type: ThemeType,
    /// 基础主题
    pub base: ThemeBase,
    /// 颜色定义
    pub colors: ThemeColors,
    /// Token 颜色规则
    pub token_colors: Vec<TokenColorRule>,
    /// 是否为内置主题
    pub builtin: bool,
}

/// 主题类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeType {
    Dark,
    Light,
    HighContrast,
}

/// 基础主题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThemeBase {
    #[serde(rename = "vs")]
    Vs,
    #[serde(rename = "vs-dark")]
    VsDark,
    #[serde(rename = "hc-black")]
    HcBlack,
    #[serde(rename = "hc-light")]
    HcLight,
}

/// 主题颜色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    /// 编辑器背景
    pub editor_bg: String,
    /// 编辑器前景
    pub editor_fg: String,
    /// 光标颜色
    pub cursor: String,
    /// 选区背景
    pub selection_bg: String,
    /// 行高亮背景
    pub line_highlight: String,
    /// 行号颜色
    pub line_number: String,
    /// 活动行号颜色
    pub active_line_number: String,
    /// 不可见字符颜色
    pub whitespace: String,
    /// 缩进参考线
    pub indent_guide: String,
    /// 活动缩进参考线
    pub active_indent_guide: String,
    /// 括号匹配背景
    pub bracket_match_bg: String,
    /// 括号匹配边框
    pub bracket_match_border: String,
    /// 查找匹配高亮
    pub find_match_bg: String,
    /// 查找匹配高亮边框
    pub find_match_border: String,
    /// 当前查找匹配
    pub find_match_highlight_bg: String,
    /// 悬停高亮
    pub hover_highlight: String,
    /// 滚动条阴影
    pub scrollbar_shadow: String,
    /// 滚动条滑块背景
    pub scrollbar_slider_bg: String,
    /// 滚动条滑块悬停背景
    pub scrollbar_slider_hover_bg: String,
    /// 滚动条滑块活动背景
    pub scrollbar_slider_active_bg: String,
    /// 小地图背景
    pub minimap_bg: String,
    /// 错误前景
    pub error_fg: String,
    /// 警告前景
    pub warning_fg: String,
    /// 信息前景
    pub info_fg: String,
    /// 提示前景
    pub hint_fg: String,
}

/// Token 颜色规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenColorRule {
    /// 规则名称
    pub name: Option<String>,
    /// 作用域
    pub scope: Vec<String>,
    /// 颜色设置
    pub settings: TokenColorSettings,
}

/// Token 颜色设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenColorSettings {
    /// 前景色
    pub foreground: Option<String>,
    /// 字体样式
    pub font_style: Option<String>,
    /// 背景色
    pub background: Option<String>,
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            editor_bg: "#1e1e1e".into(),
            editor_fg: "#d4d4d4".into(),
            cursor: "#aeafad".into(),
            selection_bg: "#264f78".into(),
            line_highlight: "#2a2d2e".into(),
            line_number: "#858585".into(),
            active_line_number: "#c6c6c6".into(),
            whitespace: "#3a3a3a".into(),
            indent_guide: "#404040".into(),
            active_indent_guide: "#707070".into(),
            bracket_match_bg: "#3a3a3a40".into(),
            bracket_match_border: "#888888".into(),
            find_match_bg: "#515c6a".into(),
            find_match_border: "#007acc".into(),
            find_match_highlight_bg: "#90602080".into(),
            hover_highlight: "#264f7840".into(),
            scrollbar_shadow: "#00000033".into(),
            scrollbar_slider_bg: "#79797933".into(),
            scrollbar_slider_hover_bg: "#64646466".into(),
            scrollbar_slider_active_bg: "#bfbfbf66".into(),
            minimap_bg: "#1e1e1e".into(),
            error_fg: "#f48771".into(),
            warning_fg: "#cca700".into(),
            info_fg: "#75beff".into(),
            hint_fg: "#6c6c6c".into(),
        }
    }
}

/// 内置主题列表
pub fn builtin_themes() -> Vec<EditorTheme> {
    vec![
        // NexTerm 暗色主题
        EditorTheme {
            id: "nexterm-dark".into(),
            name: "NexTerm Dark".into(),
            theme_type: ThemeType::Dark,
            base: ThemeBase::VsDark,
            builtin: true,
            colors: ThemeColors {
                editor_bg: "#0A0014".into(),
                editor_fg: "#B8B8D0".into(),
                cursor: "#00F0FF".into(),
                selection_bg: "#3A1A5E".into(),
                line_highlight: "#1A0A2E".into(),
                line_number: "#6A6A8A".into(),
                active_line_number: "#B8B8D0".into(),
                ..Default::default()
            },
            token_colors: vec![
                TokenColorRule {
                    name: Some("Comment".into()),
                    scope: vec!["comment".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#6A6A8A".into()),
                        font_style: Some("italic".into()),
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Keyword".into()),
                    scope: vec!["keyword".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#BD93F9".into()),
                        font_style: Some("bold".into()),
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("String".into()),
                    scope: vec!["string".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#50FA7B".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Number".into()),
                    scope: vec!["constant.numeric".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#FF79C6".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Type".into()),
                    scope: vec!["entity.name.type".into(), "support.type".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#00F0FF".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Function".into()),
                    scope: vec!["entity.name.function".into(), "support.function".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#8BE9FD".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Variable".into()),
                    scope: vec!["variable".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#B8B8D0".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Operator".into()),
                    scope: vec!["keyword.operator".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#FF79C6".into()),
                        font_style: None,
                        background: None,
                    },
                },
            ],
        },
        // NexTerm 亮色主题
        EditorTheme {
            id: "nexterm-light".into(),
            name: "NexTerm Light".into(),
            theme_type: ThemeType::Light,
            base: ThemeBase::Vs,
            builtin: true,
            colors: ThemeColors {
                editor_bg: "#F8F8FF".into(),
                editor_fg: "#2D2D3A".into(),
                cursor: "#9B59B6".into(),
                selection_bg: "#E8D5F5".into(),
                line_highlight: "#F0E6FA".into(),
                line_number: "#6A6A8A".into(),
                active_line_number: "#2D2D3A".into(),
                ..Default::default()
            },
            token_colors: vec![
                TokenColorRule {
                    name: Some("Comment".into()),
                    scope: vec!["comment".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#8A8AAA".into()),
                        font_style: Some("italic".into()),
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Keyword".into()),
                    scope: vec!["keyword".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#7B2D8E".into()),
                        font_style: Some("bold".into()),
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("String".into()),
                    scope: vec!["string".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#2D8B57".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Number".into()),
                    scope: vec!["constant.numeric".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#D63384".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Type".into()),
                    scope: vec!["entity.name.type".into(), "support.type".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#0D6EFD".into()),
                        font_style: None,
                        background: None,
                    },
                },
                TokenColorRule {
                    name: Some("Function".into()),
                    scope: vec!["entity.name.function".into(), "support.function".into()],
                    settings: TokenColorSettings {
                        foreground: Some("#0DCAF0".into()),
                        font_style: None,
                        background: None,
                    },
                },
            ],
        },
    ]
}