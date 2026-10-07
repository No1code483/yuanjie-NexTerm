//! boards.profile 的旧 IPC → dispatcher 逻辑命令映射与业务 handler 分发。
//!
//! alias 只做旧名 → 逻辑名的映射登记；真实 handler 在 commands::dispatch_legacy：
//! 闭包仅捕获 AppHandle，运行时经 handle.state::<AppState>() 解析主应用真实
//! AppState（契约 06_Rust代码契约 §8.1：pool / 认证 / 服务全部来源于此，禁止
//! 伪造或第二池）；业务实现复用 profile_commands / auth_commands 原函数，
//! 返回值序列化与旧 IPC 路径一致（V1 输入/输出快照等价）。

pub const PLUGIN_ID: &str = "boards.profile";
pub const SHORT_CODE: &str = "pf";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IpcAlias {
    pub plugin_id: &'static str,
    pub short_code: &'static str,
    pub legacy_command: &'static str,
    pub new_command: &'static str,
}

macro_rules! alias {
    ($command:literal) => {
        IpcAlias {
            plugin_id: PLUGIN_ID,
            short_code: SHORT_CODE,
            legacy_command: $command,
            new_command: concat!("pf:plugin:", $command),
        }
    };
}

pub const IPC_ALIASES: &[IpcAlias] = &[
    alias!("get_profile"),
    alias!("set_profile"),
    alias!("get_resumes"),
    alias!("add_resume"),
    alias!("update_resume"),
    alias!("delete_resume"),
    alias!("get_random_quote"),
    alias!("add_quote"),
    alias!("check_quote_duplicate"),
    alias!("get_all_quotes"),
    alias!("batch_add_quotes"),
    alias!("seed_default_quotes"),
    alias!("delete_quote"),
    alias!("save_personal_info"),
    alias!("get_personal_info"),
    alias!("profile_ai_polish_resume"),
    alias!("profile_ai_spell_check_resume"),
    alias!("profile_ai_generate_resume"),
    alias!("profile_ai_quote_check"),
    alias!("profile_ai_quote_complete"),
    alias!("resume_polish"),
    alias!("export_user_data"),
];

// ========== dispatcher 业务 handler 分发（S2 门禁：dispatcher 状态注入） ==========
//
// 每条 handler 的形态一致：闭包捕获 AppHandle → 运行时 handle.state::<AppState>()
// 取主应用真实状态 → 从 Json args 提取旧 IPC 参数（键名与前端 invoke 完全一致）
// → 直接调用 profile_commands / auth_commands 原函数（State 由 handle.state() 构造）
// → serde 序列化返回值，与旧 Tauri command 路径输出一致。

use serde_json::Value as Json;
use tauri::{AppHandle, Manager, Runtime};

use crate::plugins::_legacy::commands::profile_commands;
use crate::db::connection::AppState;

/// 从 args 提取必填字符串参数；缺失/类型不符按旧 Tauri 反序列化失败语义返回错误。
fn arg_str(args: &Json, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(Json::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("参数 {key} 缺失或不是字符串"))
}

fn arg_i64(args: &Json, key: &str) -> Result<i64, String> {
    args.get(key)
        .and_then(Json::as_i64)
        .ok_or_else(|| format!("参数 {key} 缺失或不是整数"))
}

fn arg_opt_str(args: &Json, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("参数 {key} 不是字符串或 null")),
    }
}

/// 逐条分发到旧命令函数。legacy 为 IPC_ALIASES.legacy_command 值。
pub(crate) async fn dispatch_legacy<R: Runtime>(
    handle: AppHandle<R>,
    legacy: &'static str,
    args: Json,
) -> Result<Json, String> {
    let state = handle.state::<AppState>();
    // delete_resume 的旧实现不返回 rows_affected；执行前只读确认，避免不存在记录也发 deleted。
    let deleted_resume_existed = if legacy == "delete_resume" {
        resume_exists(&state, args.get("id").and_then(Json::as_i64)).await
    } else {
        false
    };

    let response = match legacy {
        "get_profile" => {
            let key = arg_str(&args, "key")?;
            to_json(profile_commands::get_profile(state, key).await?)
        }
        "set_profile" => {
            let key = arg_str(&args, "key")?;
            let value = arg_str(&args, "value")?;
            to_json(profile_commands::set_profile(state, key, value).await?)
        }
        "get_resumes" => to_json(profile_commands::get_resumes(state).await?),
        "add_resume" => {
            let title = arg_str(&args, "title")?;
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::add_resume(state, title, content).await?)
        }
        "update_resume" => {
            let id = arg_i64(&args, "id")?;
            let title = arg_str(&args, "title")?;
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::update_resume(state, id, title, content).await?)
        }
        "delete_resume" => {
            let id = arg_i64(&args, "id")?;
            to_json(profile_commands::delete_resume(state, id).await?)
        }
        "get_random_quote" => to_json(profile_commands::get_random_quote(state).await?),
        "add_quote" => {
            let content = arg_str(&args, "content")?;
            let source = arg_opt_str(&args, "source")?;
            let quote_type = arg_opt_str(&args, "quote_type")?;
            to_json(profile_commands::add_quote(state, content, source, quote_type).await?)
        }
        "check_quote_duplicate" => {
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::check_quote_duplicate(state, content).await?)
        }
        "get_all_quotes" => to_json(profile_commands::get_all_quotes(state).await?),
        "batch_add_quotes" => {
            let quotes = parse_batch_quotes(&args)?;
            to_json(profile_commands::batch_add_quotes(state, quotes).await?)
        }
        "seed_default_quotes" => to_json(profile_commands::seed_default_quotes(state).await?),
        "delete_quote" => {
            let id = arg_i64(&args, "id")?;
            to_json(profile_commands::delete_quote(state, id).await?)
        }
        "save_personal_info" => {
            let json = arg_str(&args, "json")?;
            to_json(profile_commands::save_personal_info(state, json).await?)
        }
        "get_personal_info" => to_json(profile_commands::get_personal_info(state).await?),
        "profile_ai_polish_resume" => {
            let resume_id = arg_i64(&args, "resume_id")?;
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::profile_ai_polish_resume(state, resume_id, content).await?)
        }
        "profile_ai_spell_check_resume" => {
            let resume_id = arg_i64(&args, "resume_id")?;
            let content = arg_str(&args, "content")?;
            to_json(
                profile_commands::profile_ai_spell_check_resume(state, resume_id, content).await?,
            )
        }
        "profile_ai_generate_resume" => {
            let resume_id = arg_i64(&args, "resume_id")?;
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::profile_ai_generate_resume(state, resume_id, content).await?)
        }
        "profile_ai_quote_check" => {
            let quote_id = arg_i64(&args, "quote_id")?;
            to_json(profile_commands::profile_ai_quote_check(state, quote_id).await?)
        }
        "profile_ai_quote_complete" => {
            let content = arg_str(&args, "content")?;
            to_json(profile_commands::profile_ai_quote_complete(state, content).await?)
        }
        "resume_polish" => {
            let text = arg_str(&args, "text")?;
            let section = arg_opt_str(&args, "section")?;
            to_json(profile_commands::resume_polish(state, text, section).await?)
        }
        "export_user_data" => to_json(profile_commands::export_user_data(state).await?),
        _ => Err(format!("未知 profile 逻辑命令: {legacy}")),
    }?;

    publish_domain_events(
        &handle,
        domain_event_specs(legacy, &args, &response, deleted_resume_existed),
    );
    Ok(response)
}

#[derive(Debug, PartialEq)]
struct DomainEventSpec {
    name: &'static str,
    payload: Json,
}

async fn resume_exists(state: &AppState, id: Option<i64>) -> bool {
    let (Some(id), Some(user_id)) = (id, *state.current_user.read().await) else {
        return false;
    };
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM resumes WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .is_ok_and(|count| count > 0)
}

fn domain_event_specs(
    legacy: &str,
    args: &Json,
    response: &Json,
    deleted_resume_existed: bool,
) -> Vec<DomainEventSpec> {
    if response.get("code").and_then(Json::as_i64) != Some(0) {
        return Vec::new();
    }

    let data = response.get("data").unwrap_or(&Json::Null);
    let item_id = || data.get("id").cloned().unwrap_or(Json::Null);
    let deleted_payload = |item_type: &str, id: Json| {
        serde_json::json!({
            "itemId": id,
            "itemType": item_type,
            "deletedAt": chrono::Utc::now().timestamp_millis(),
            "softDelete": true,
            "originPlugin": PLUGIN_ID,
        })
    };

    match legacy {
        "set_profile" => vec![DomainEventSpec {
            name: "boards.profile:profile.updated",
            payload: serde_json::json!({
                "itemId": item_id(),
                "itemType": "profile",
                "changedFields": [args.get("key").cloned().unwrap_or(Json::Null)],
            }),
        }],
        "save_personal_info" => vec![DomainEventSpec {
            name: "boards.profile:profile.updated",
            payload: serde_json::json!({
                "itemId": "personal_info",
                "itemType": "profile",
                "changedFields": ["personal_info"],
            }),
        }],
        "add_resume" => vec![DomainEventSpec {
            name: "boards.profile:resume.created",
            payload: serde_json::json!({"itemId": item_id(), "itemType": "resume"}),
        }],
        "update_resume" => vec![DomainEventSpec {
            name: "boards.profile:resume.updated",
            payload: serde_json::json!({
                "itemId": item_id(),
                "itemType": "resume",
                "changedFields": ["title", "content"],
            }),
        }],
        "delete_resume" if deleted_resume_existed => vec![DomainEventSpec {
            name: "boards.profile:resume.deleted",
            payload: deleted_payload("resume", args.get("id").cloned().unwrap_or(Json::Null)),
        }],
        "add_quote" => vec![DomainEventSpec {
            name: "boards.profile:quote.created",
            payload: serde_json::json!({"itemId": item_id(), "itemType": "quote"}),
        }],
        "batch_add_quotes" => data
            .as_array()
            .into_iter()
            .flatten()
            .map(|quote| DomainEventSpec {
                name: "boards.profile:quote.created",
                payload: serde_json::json!({
                    "itemId": quote.get("id").cloned().unwrap_or(Json::Null),
                    "itemType": "quote",
                    "source": "batch",
                }),
            })
            .collect(),
        "seed_default_quotes" if data.as_u64().unwrap_or(0) > 0 => vec![DomainEventSpec {
            name: "boards.profile:quote.created",
            payload: serde_json::json!({
                "itemType": "quote",
                "count": data,
                "source": "seed",
            }),
        }],
        "delete_quote" if data.as_u64().unwrap_or(0) > 0 => vec![DomainEventSpec {
            name: "boards.profile:quote.deleted",
            payload: deleted_payload("quote", args.get("id").cloned().unwrap_or(Json::Null)),
        }],
        _ => Vec::new(),
    }
}

fn publish_domain_events<R: Runtime>(handle: &AppHandle<R>, specs: Vec<DomainEventSpec>) {
    if specs.is_empty() {
        return;
    }
    let kernel = handle.state::<kernel::tauri_glue::KernelState>();
    let origin = PLUGIN_ID.to_string();
    for spec in specs {
        if let Err(error) = kernel.security.check_publish(
            super::manifest::manifest(),
            spec.name,
            kernel_api::EventScope::Domain,
        ) {
            eprintln!("[boards.profile] 领域事件权限校验失败: {error}");
            continue;
        }
        let event = kernel_api::Event {
            name: spec.name.to_string(),
            origin: origin.clone(),
            domain: origin.clone(),
            scope: kernel_api::EventScope::Domain,
            payload: spec.payload,
            at: chrono::Utc::now().timestamp_millis(),
        };
        if let Err(error) = kernel.bus.publish(&origin, event) {
            eprintln!("[boards.profile] 领域事件发布失败: {error}");
        }
    }
}

fn to_json<T: serde::Serialize>(value: T) -> Result<Json, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

fn parse_request<T: serde::de::DeserializeOwned>(args: &Json) -> Result<T, String> {
    args.get("request")
        .cloned()
        .ok_or_else(|| "参数 request 缺失".to_string())
        .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
}

fn parse_batch_quotes(args: &Json) -> Result<Vec<(String, Option<String>, String)>, String> {
    let raw = args
        .get("quotes")
        .and_then(Json::as_array)
        .ok_or_else(|| "参数 quotes 缺失或不是数组".to_string())?;
    raw.iter()
        .map(|item| {
            let content = item
                .get(0)
                .and_then(Json::as_str)
                .ok_or_else(|| "quotes 项缺少 content".to_string())?
                .to_string();
            let source = item.get(1).and_then(Json::as_str).map(String::from);
            let quote_type = item
                .get(2)
                .and_then(Json::as_str)
                .ok_or_else(|| "quotes 项缺少 quote_type".to_string())?
                .to_string();
            Ok((content, source, quote_type))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    const EXPECTED_LEGACY_COMMANDS: &[&str] = &[
        "get_profile",
        "set_profile",
        "get_resumes",
        "add_resume",
        "update_resume",
        "delete_resume",
        "get_random_quote",
        "add_quote",
        "check_quote_duplicate",
        "get_all_quotes",
        "batch_add_quotes",
        "seed_default_quotes",
        "delete_quote",
        "save_personal_info",
        "get_personal_info",
        "profile_ai_polish_resume",
        "profile_ai_spell_check_resume",
        "profile_ai_generate_resume",
        "profile_ai_quote_check",
        "profile_ai_quote_complete",
        "resume_polish",
        "export_user_data",
    ];

    #[test]
    fn alias_map_covers_all_legacy_commands_once() {
        let actual: HashSet<_> = IPC_ALIASES
            .iter()
            .map(|alias| alias.legacy_command)
            .collect();
        let expected: HashSet<_> = EXPECTED_LEGACY_COMMANDS.iter().copied().collect();

        assert_eq!(IPC_ALIASES.len(), 22);
        assert_eq!(actual.len(), IPC_ALIASES.len(), "legacy_command 存在重复");
        assert_eq!(actual, expected);
    }

    #[test]
    fn alias_map_uses_formal_id_and_unique_short_code_commands() {
        let new_commands: HashSet<_> = IPC_ALIASES.iter().map(|alias| alias.new_command).collect();

        assert_eq!(
            new_commands.len(),
            IPC_ALIASES.len(),
            "new_command 存在重复"
        );
        for alias in IPC_ALIASES {
            assert_eq!(alias.plugin_id, PLUGIN_ID);
            assert_eq!(alias.short_code, SHORT_CODE);
            assert_eq!(
                alias.new_command,
                format!("{SHORT_CODE}:plugin:{}", alias.legacy_command)
            );
        }
    }

    fn success(data: Json) -> Json {
        serde_json::json!({"code": 0, "message": "success", "data": data})
    }

    #[test]
    fn profile_updated_event_is_built_for_profile_writes() {
        for (command, args, data) in [
            (
                "set_profile",
                serde_json::json!({"key": "theme"}),
                serde_json::json!({"id": 7}),
            ),
            ("save_personal_info", serde_json::json!({}), Json::Null),
        ] {
            let events = domain_event_specs(command, &args, &success(data), false);
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].name, "boards.profile:profile.updated");
            assert_eq!(events[0].payload["itemType"], "profile");
        }
    }

    #[test]
    fn resume_event_types_and_deleted_contract_are_built() {
        let created = domain_event_specs(
            "add_resume",
            &serde_json::json!({}),
            &success(serde_json::json!({"id": 11})),
            false,
        );
        let updated = domain_event_specs(
            "update_resume",
            &serde_json::json!({}),
            &success(serde_json::json!({"id": 11})),
            false,
        );
        let deleted = domain_event_specs(
            "delete_resume",
            &serde_json::json!({"id": 11}),
            &success(Json::Null),
            true,
        );

        assert_eq!(created[0].name, "boards.profile:resume.created");
        assert_eq!(updated[0].name, "boards.profile:resume.updated");
        assert_deleted_contract(&deleted[0], "resume", 11);
        assert!(domain_event_specs(
            "delete_resume",
            &serde_json::json!({"id": 999}),
            &success(Json::Null),
            false
        )
        .is_empty());
    }

    #[test]
    fn quote_event_types_cover_single_batch_seed_and_real_delete() {
        let created = domain_event_specs(
            "add_quote",
            &serde_json::json!({}),
            &success(serde_json::json!({"id": 21})),
            false,
        );
        let batch = domain_event_specs(
            "batch_add_quotes",
            &serde_json::json!({}),
            &success(serde_json::json!([{"id": 22}, {"id": 23}])),
            false,
        );
        let seeded = domain_event_specs(
            "seed_default_quotes",
            &serde_json::json!({}),
            &success(serde_json::json!(2)),
            false,
        );
        let deleted = domain_event_specs(
            "delete_quote",
            &serde_json::json!({"id": 21}),
            &success(serde_json::json!(1)),
            false,
        );

        assert_eq!(created[0].name, "boards.profile:quote.created");
        assert_eq!(batch.len(), 2);
        assert!(batch
            .iter()
            .all(|event| event.name == "boards.profile:quote.created"));
        assert_eq!(seeded[0].name, "boards.profile:quote.created");
        assert_deleted_contract(&deleted[0], "quote", 21);
        assert!(domain_event_specs(
            "delete_quote",
            &serde_json::json!({"id": 999}),
            &success(serde_json::json!(0)),
            false
        )
        .is_empty());
    }

    #[test]
    fn failed_or_read_only_commands_do_not_build_domain_events() {
        let failed = serde_json::json!({"code": 500, "message": "failed", "data": null});
        assert!(domain_event_specs("add_resume", &Json::Null, &failed, false).is_empty());
        assert!(domain_event_specs(
            "get_resumes",
            &Json::Null,
            &success(serde_json::json!([])),
            false
        )
        .is_empty());
    }

    fn assert_deleted_contract(event: &DomainEventSpec, item_type: &str, item_id: i64) {
        assert!(event.name.ends_with(".deleted"));
        assert_eq!(event.payload["itemId"], item_id);
        assert_eq!(event.payload["itemType"], item_type);
        assert_eq!(event.payload["softDelete"], true);
        assert_eq!(event.payload["originPlugin"], PLUGIN_ID);
        assert!(event.payload["deletedAt"].as_i64().is_some_and(|at| at > 0));
    }
}
