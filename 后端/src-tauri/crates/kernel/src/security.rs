//! 权限门（契约：06_Rust代码契约.md §七）
//! manifest 六类权限校验 + fs/net 检查桩
//! TODO(阶段3): 收编 safety/sandbox 实际策略（现 fs/net 桩化返回 Ok）

use kernel_api::KernelError;
use kernel_api::{EventScope, Manifest, PluginLevel};

pub struct SecurityGate;

impl Default for SecurityGate {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityGate {
    /// 插件短码表（kernel 维护，来源 05_数据层插件化设计 §2.1；未登记插件回退 id 末段）
    const SHORT_CODES: &[(&str, &str)] = &[
        ("boards.home", "hm"),
        ("home.todo", "td"),
        ("home.journal", "jn"),
        ("home.timer", "ti"),
        ("home.news", "nw"),
        // 批次2b-1：原键 `ai` 改为 manifest id 逐字一致（否则短码解析回退 id 末段）
        ("boards.ai", "ai"),
        // 批次2b-1：L2 子插件短码（ai.models / ai.agent / ai.groupchat）
        ("ai.models", "am"),
        // 批次C1：ai.sessions（AI会话必备子插件，短码 ss）
        ("ai.sessions", "ss"),
        ("ai.agent", "ag"),
        ("ai.groupchat", "gc"),
        ("terminal", "tm"),
        // 批次3a：boards.terminal 改键为 manifest id 逐字一致（原键 `terminal` 保留为历史注释）；
        // L2 terminal.linux 短码 lx（3b 起用）
        ("boards.terminal", "tm"),
        ("terminal.linux", "lx"),
        // 批次3c：terminal.yuancode L2 短码（C2 裁定加插槽同批登记）
        ("terminal.yuancode", "yc"),
        // 批次4a-1：原键 `xin` 改为 manifest id 逐字一致（否则短码解析回退 id 末段得 `xin`）；
        // L2 xin.wellness 短码 xw、xin.realtime 短码 xr 同批登记
        ("boards.xin", "xn"),
        ("xin.wellness", "xw"),
        ("xin.realtime", "xr"),
        ("xin.orchestration", "xo"),
        ("boards.game", "gm"),
        ("boards.profile", "pf"),
        ("customs.intelligence", "sp"),
        ("customs.auth", "au"),
        // 批次2a-1：原键 `knowledge` 改为 manifest id 逐字一致（否则短码解析回退 id 末段）
        ("boards.knowledge", "kb"),
        // 批C3：knowledge.templates L2 短码（前端 kt:plugin:* 消费；原登记遗漏补齐）
        ("knowledge.templates", "kt"),
        ("customs.recycle", "rc"),
        ("customs.search", "se"),
        ("customs.sync", "sy"),
        ("customs.systemtools", "st"),
        ("_hello", "hw"), // 阶段1 示例插件（B8）；id 含下划线前缀，短码仍为 hw（2026-09-07 B7 决策）
    ];

    pub fn new() -> Self {
        Self
    }

    /// init 时总校验：六类权限格式合法 + 约束检查
    /// 约束：① Custom 级才允许跨域订阅 ② fs/net 路径格式合法
    ///       ③ db 表名先做格式校验；前缀与旧表 ownership 在 registry 迁移后校验
    pub fn validate_manifest(&self, m: &Manifest) -> Result<(), KernelError> {
        // 基本字段
        if m.id.is_empty() {
            return Err(KernelError::Config("manifest.id 不能为空".into()));
        }
        if m.kernel_api != "1" {
            return Err(KernelError::Config(format!(
                "插件 {} 的 kernelApi={} 不受支持（当前仅 '1'）",
                m.id, m.kernel_api
            )));
        }

        // ① 事件权限约束
        for f in &m.permissions.events.subscribe {
            if !Self::valid_event_filter(f) {
                return Err(KernelError::Config(format!(
                    "插件 {} subscribe 过滤器非法: {f}",
                    m.id
                )));
            }
            // 跨域订阅：只允许 Custom 级（订阅别人的域）
            let domain = f.split(':').next().unwrap_or("");
            if !domain.is_empty() && domain != m.id && m.level != PluginLevel::Custom {
                return Err(KernelError::Config(format!(
                    "插件 {}（非 Custom 级）试图跨域订阅 {f}",
                    m.id
                )));
            }
        }
        for f in &m.permissions.events.publish {
            if !Self::valid_event_filter(f) {
                return Err(KernelError::Config(format!(
                    "插件 {} publish 过滤器非法: {f}",
                    m.id
                )));
            }
            // 发布白名单必须在自己域内（通配只允许出现在自己域内）
            let domain = f.split(':').next().unwrap_or("");
            if domain != m.id && f != "*" {
                return Err(KernelError::Config(format!(
                    "插件 {} 试图声明非本域发布权限 {f}",
                    m.id
                )));
            }
        }

        // ③ db 表名先做格式校验；短码前缀与旧表 ownership 校验在基线迁移后由 registry 完成。
        for t in &m.permissions.db {
            let valid = !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !valid {
                return Err(KernelError::Config(format!(
                    "插件 {} 的 db 权限表名非法: {t}",
                    m.id
                )));
            }
        }

        // ② fs/net 格式合法性（阶段1 桩：仅查非空格式）
        for p in &m.permissions.fs {
            if p.is_empty() {
                return Err(KernelError::Config(format!("插件 {} fs 权限项为空", m.id)));
            }
        }
        for u in &m.permissions.net {
            if u.is_empty() {
                return Err(KernelError::Config(format!("插件 {} net 权限项为空", m.id)));
            }
        }
        Ok(())
    }

    /// 事件发布运行时校验（bus.publish 前由内核调用）
    pub fn check_publish(
        &self,
        m: &Manifest,
        name: &str,
        scope: EventScope,
    ) -> Result<(), KernelError> {
        let domain = name.split(':').next().unwrap_or("");
        if domain != m.id {
            return Err(KernelError::PermissionDenied(format!(
                "插件 {} 试图发布非本域事件 {name}",
                m.id
            )));
        }
        let ok = m.permissions.events.publish.iter().any(|f| {
            f == "*"
                || f == name
                || (f.ends_with(":*") && name.starts_with(f.trim_end_matches('*')))
        });
        if !ok {
            return Err(KernelError::PermissionDenied(format!(
                "插件 {} 未声明事件 {name} 的发布权限",
                m.id
            )));
        }
        let _ = scope;
        Ok(())
    }

    /// 事件订阅运行时校验
    pub fn check_subscribe(&self, m: &Manifest, filter: &str) -> Result<(), KernelError> {
        let ok = m.permissions.events.subscribe.iter().any(|f| {
            f == "*"
                || f == filter
                || (f.ends_with(":*") && filter.starts_with(f.trim_end_matches('*')))
                || (filter.ends_with(":*") && f.starts_with(filter.trim_end_matches('*')))
        });
        if !ok {
            return Err(KernelError::PermissionDenied(format!(
                "插件 {} 未声明订阅 {filter} 的权限",
                m.id
            )));
        }
        Ok(())
    }

    /// IPC 前缀校验（命令名是否在插件 ipc 白名单内）
    pub fn check_ipc(&self, m: &Manifest, command: &str) -> Result<(), KernelError> {
        let ok = m.permissions.ipc.iter().any(|p| {
            if let Some(prefix) = p.strip_suffix('*') {
                command.starts_with(prefix)
            } else {
                p == command
            }
        });
        if !ok {
            return Err(KernelError::PermissionDenied(format!(
                "命令 {command} 不在插件 {} 的 ipc 白名单内",
                m.id
            )));
        }
        Ok(())
    }

    /// fs 运行时检查（阶段1 桩）
    /// TODO(阶段3): 收编 safety/sandbox 实际策略
    pub fn check_fs(&self, _plugin: &str, _path: &str) -> Result<(), KernelError> {
        Ok(())
    }

    /// net 运行时检查（阶段1 桩）
    /// TODO(阶段3): 收编 safety/sandbox 实际策略
    pub fn check_net(&self, _plugin: &str, _url: &str) -> Result<(), KernelError> {
        Ok(())
    }

    /// 插件短码：查表；未登记回退 id 末段（如 "boards.home" -> "home"）
    pub fn short_code(id: &str) -> String {
        if let Some((_, sc)) = Self::SHORT_CODES.iter().find(|(pid, _)| *pid == id) {
            return sc.to_string();
        }
        id.rsplit('.').next().unwrap_or(id).to_string()
    }

    /// 事件过滤器合法性："domain" / "domain:*" / "domain:obj.action"
    fn valid_event_filter(f: &str) -> bool {
        if f.is_empty() || f == "*" {
            return true;
        }
        let (domain, rest) = match f.split_once(':') {
            Some((d, r)) => (d, r),
            None => (f, ""),
        };
        let domain_valid = domain.split('.').all(|segment| {
            !segment.is_empty()
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        if !domain_valid {
            return false;
        }
        rest == "*"
            || rest.is_empty()
            || rest.split('.').all(|segment| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_api::{EventPerms, Permissions};

    fn manifest(level: PluginLevel, id: &str, perms: Permissions) -> Manifest {
        Manifest {
            id: id.into(),
            name: "t".into(),
            level,
            parent: None,
            slot: None,
            version: "0.1.0".into(),
            kernel_api: "1".into(),
            permissions: perms,
            slots: vec![],
            i18n_namespace: id.into(),
        }
    }

    // 必测 1：未声明权限拒绝（发布未声明的事件）
    #[test]
    fn undeclared_publish_rejected() {
        let m = manifest(
            PluginLevel::Board,
            "hello",
            Permissions {
                events: EventPerms {
                    subscribe: vec![],
                    publish: vec!["hello:*".to_string()],
                },
                ..Default::default()
            },
        );
        let g = SecurityGate::new();
        g.check_publish(&m, "hello:ping", EventScope::Live).unwrap();
        let err = g
            .check_publish(&m, "home:todo.updated", EventScope::Live)
            .unwrap_err();
        assert!(matches!(err, KernelError::PermissionDenied(_)), "{err:?}");
    }

    // 必测 2：Custom 级才允许跨域订阅
    #[test]
    fn cross_domain_subscribe_only_custom() {
        let perms = |sub: Vec<&str>| Permissions {
            events: EventPerms {
                subscribe: sub.into_iter().map(String::from).collect(),
                publish: vec![],
            },
            ..Default::default()
        };
        let g = SecurityGate::new();

        // Board 级跨域订阅 → manifest 校验拒绝
        let board = manifest(PluginLevel::Board, "hello", perms(vec!["home:*"]));
        let err = g.validate_manifest(&board).unwrap_err();
        assert!(matches!(err, KernelError::Config(_)), "{err:?}");

        // Custom 级跨域订阅 → 放行
        let custom = manifest(PluginLevel::Custom, "recycle", perms(vec!["home:*"]));
        g.validate_manifest(&custom).unwrap();
        g.check_subscribe(&custom, "home:todo.updated").unwrap();
    }

    // 必测 3：db 表名格式校验；前缀/旧表归属由 registry 在迁移后校验
    #[test]
    fn db_table_name_format_constraint() {
        let g = SecurityGate::new();
        let prefixed = manifest(
            PluginLevel::Board,
            "_hello",
            Permissions {
                db: vec!["hw_notes".into()],
                ..Default::default()
            },
        );
        g.validate_manifest(&prefixed).unwrap();

        let legacy = manifest(
            PluginLevel::Board,
            "boards.profile",
            Permissions {
                db: vec!["resumes".into()],
                ..Default::default()
            },
        );
        g.validate_manifest(&legacy).unwrap();

        let bad = manifest(
            PluginLevel::Board,
            "_hello",
            Permissions {
                db: vec!["other-table".into()],
                ..Default::default()
            },
        );
        let err = g.validate_manifest(&bad).unwrap_err();
        assert!(matches!(err, KernelError::Config(_)), "{err:?}");

        // kernel_ 前缀放行
        let kb = manifest(
            PluginLevel::Board,
            "_hello",
            Permissions {
                db: vec!["kernel_domain_events".into()],
                ..Default::default()
            },
        );
        g.validate_manifest(&kb).unwrap();
    }

    #[test]
    fn dotted_event_domains_are_validated_by_segment() {
        let g = SecurityGate::new();

        for filter in ["boards.profile:*", "boards.profile:resume.deleted"] {
            let valid = manifest(
                PluginLevel::Board,
                "boards.profile",
                Permissions {
                    events: EventPerms {
                        subscribe: vec![],
                        publish: vec![filter.into()],
                    },
                    ..Default::default()
                },
            );
            g.validate_manifest(&valid).unwrap();
        }

        for filter in [
            ".boards.profile:*",
            "boards..profile:*",
            "boards.profile.:*",
            "boards.profile:resume..deleted",
        ] {
            let invalid = manifest(
                PluginLevel::Board,
                "boards.profile",
                Permissions {
                    events: EventPerms {
                        subscribe: vec![],
                        publish: vec![filter.into()],
                    },
                    ..Default::default()
                },
            );
            let err = g.validate_manifest(&invalid).unwrap_err();
            assert!(matches!(err, KernelError::Config(_)), "{filter}: {err:?}");
        }
    }

    // 必测 4：IPC 白名单通配
    #[test]
    fn ipc_whitelist_wildcard() {
        let m = manifest(
            PluginLevel::Board,
            "hello",
            Permissions {
                ipc: vec!["_hello_*".into()],
                ..Default::default()
            },
        );
        let g = SecurityGate::new();
        g.check_ipc(&m, "_hello_ping").unwrap();
        let err = g.check_ipc(&m, "home_list").unwrap_err();
        assert!(matches!(err, KernelError::PermissionDenied(_)), "{err:?}");
    }

    #[test]
    fn formal_plugin_id_resolves_to_profile_short_code() {
        assert_eq!(SecurityGate::short_code("boards.profile"), "pf");
        assert_eq!(SecurityGate::short_code("boards.game"), "gm");
        assert_eq!(SecurityGate::short_code("customs.systemtools"), "st");
        assert_eq!(SecurityGate::short_code("customs.sync"), "sy");
        assert_eq!(SecurityGate::short_code("customs.intelligence"), "sp");
        assert_eq!(SecurityGate::short_code("custom.example"), "example");
    }

    // fs/net 桩放行
    #[test]
    fn fs_net_stub_ok() {
        let g = SecurityGate::new();
        assert!(g.check_fs("hello", "C:/anything").is_ok());
        assert!(g.check_net("hello", "https://anything").is_ok());
    }
}
