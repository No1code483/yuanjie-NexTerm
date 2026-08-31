use sqlx::SqlitePool;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use tokio::sync::RwLock;

use crate::db::repositories::chat_repo;
use crate::error::app_error::AppError;
use crate::models::chat::{OrchestratorConfig, OrchestratorResult};
use crate::services::ai_model_service::AiModelService;
use crate::crypto::mek_manager::MekManager;

pub struct OrchestratorState {
    pub current_round: u32,
    pub total_tokens_used: i64,
    pub convergence_counter: u32,
    pub is_force_stopped: bool,
    /// 上一轮各参与者的响应内容，用于跨轮相似度比较
    pub prev_round_responses: Vec<String>,
    /// 累计信息增益（每轮新增内容的字符数），用于检测边际递减
    pub cumulative_info_gain: Vec<usize>,
}

/// 单轮分析结果
struct RoundAnalysis {
    /// 该轮是否有实质新观点（基于关键词+语义）
    has_novelty: bool,
    /// 与上一轮的平均文本相似度 (0~1)，越高说明在重复
    avg_cross_round_sim: f64,
    /// 参与者间的一致性比例 (0~1)
    agreement_ratio: f64,
    /// 该轮估计的新增信息量（字符数）
    info_gain: usize,
}

pub struct AiOrchestrator;

impl AiOrchestrator {
    pub async fn run_group_chat(
        pool: &SqlitePool,
        conversation_id: i64,
        user_id: i64,
        user_message: &str,
        config: &OrchestratorConfig,
        app_handle: &tauri::AppHandle,
        mek_manager: &Arc<RwLock<MekManager>>,
        force_stop: Arc<AtomicBool>,
    ) -> Result<OrchestratorResult, AppError> {
        let ai_service = AiModelService::new();
        
        let mut state = OrchestratorState {
            current_round: 0,
            total_tokens_used: 0,
            convergence_counter: 0,
            is_force_stopped: false,
            prev_round_responses: Vec::new(),
            cumulative_info_gain: Vec::new(),
        };

        state.total_tokens_used += Self::estimate_tokens(user_message);

        let participants = chat_repo::get_participants(pool, conversation_id, user_id).await?;

        for round in 1..=config.max_rounds {
            // 检查是否被外部停止
            if force_stop.load(Ordering::Relaxed) {
                state.is_force_stopped = true;
                let _ = app_handle.emit(
                    "ai-orchestrator",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "force_stopped",
                    }),
                );
                break;
            }

            state.current_round = round;

            if state.total_tokens_used >= config.token_budget {
                let _ = app_handle.emit(
                    "ai-orchestrator",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "token_exhausted",
                    }),
                );
                break;
            }

            if (state.total_tokens_used as f64)
                >= (config.token_budget as f64 * config.token_warn_ratio)
            {
                let _ = app_handle.emit(
                    "ai-orchestrator",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "token_warning",
                    }),
                );
            }

            // 收集本轮所有响应，用于收敛分析
            let mut round_responses: Vec<String> = Vec::new();

            for participant in &participants {
                if state.is_force_stopped {
                    break;
                }

                let _ = app_handle.emit(
                    "ai-orchestrator",
                    serde_json::json!({
                        "conversation_id": conversation_id,
                        "event": "round_start",
                        "round": round,
                        "participant": participant.role,
                    }),
                );

                if let Some(model_id) = participant.model_id {
                    // Phase 3 §2.2.6 流式改造：原 call_model 改为 call_model_stream，
                    // 逐 chunk emit `ai-stream` 事件（前端可实时渲染 token）。
                    // 下方 message_complete 逻辑保持不变，确保收敛分析与状态机不受影响。
                    match ai_service.call_model_stream(
                        pool,
                        mek_manager,
                        user_id,
                        model_id,
                        user_message,
                        None,
                        app_handle,
                        conversation_id,
                        Some(participant.id),
                    ).await {
                        Ok(response) => {
                            let now = chrono::Utc::now().timestamp_millis();
                            let msg = chat_repo::add_message(
                                pool,
                                user_id,
                                conversation_id,
                                "model",
                                Some(participant.id),
                                &response,
                                round as i32,
                                now,
                            )
                            .await?;

                            let tokens = Self::estimate_tokens(&response);
                            state.total_tokens_used += tokens;

                            let _ = app_handle.emit(
                                "ai-orchestrator",
                                serde_json::json!({
                                    "conversation_id": conversation_id,
                                    "event": "message_complete",
                                    "message_id": msg.id,
                                    "sender_id": participant.id,
                                    "participant": participant.role,
                                    "round": round,
                                    "tokens": tokens,
                                    "content": &response,
                                    "created_at": now,
                                }),
                            );

                            round_responses.push(response);
                        }
                        Err(e) => {
                            tracing::error!(error = %e, participant = %participant.role, "AI 模型调用失败");
                            
                            let error_msg = format!("[错误] {} 调用失败: {}", participant.role, e);
                            let now = chrono::Utc::now().timestamp_millis();
                            let err_msg = chat_repo::add_message(
                                pool,
                                user_id,
                                conversation_id,
                                "system",
                                None,
                                &error_msg,
                                round as i32,
                                now,
                            )
                            .await?;

                            let _ = app_handle.emit(
                                "ai-orchestrator",
                                serde_json::json!({
                                    "conversation_id": conversation_id,
                                    "event": "message_complete",
                                    "message_id": err_msg.id,
                                    "sender_id": serde_json::Value::Null,
                                    "participant": format!("{} (错误)", participant.role),
                                    "round": round,
                                    "tokens": 0,
                                    "content": &error_msg,
                                    "created_at": now,
                                }),
                            );
                        }
                    }
                } else {
                    let response = format!(
                        "[{} 第{}轮] 参与者 {} 的回复（模拟模式 - 未配置模型）",
                        chrono::Utc::now().format("%H:%M:%S"),
                        round,
                        participant.role
                    );

                    let now = chrono::Utc::now().timestamp_millis();
                    let msg = chat_repo::add_message(
                        pool,
                        user_id,
                        conversation_id,
                        "model",
                        Some(participant.id),
                        &response,
                        round as i32,
                        now,
                    )
                    .await?;

                    let tokens = Self::estimate_tokens(&response);
                    state.total_tokens_used += tokens;

                    let _ = app_handle.emit(
                        "ai-orchestrator",
                        serde_json::json!({
                            "conversation_id": conversation_id,
                            "event": "message_complete",
                            "message_id": msg.id,
                            "sender_id": participant.id,
                            "participant": participant.role,
                            "round": round,
                            "tokens": tokens,
                            "content": &response,
                            "created_at": now,
                        }),
                    );

                    round_responses.push(response);
                }
            }

            // ── 多维度收敛检测（基于 RoundTable 论文 + Adaptive Stability Detection）──
            let analysis = Self::analyze_round(&round_responses, &state.prev_round_responses);

            tracing::info!(
                round = round,
                novelty = analysis.has_novelty,
                cross_sim = analysis.avg_cross_round_sim,
                agreement = analysis.agreement_ratio,
                info_gain = analysis.info_gain,
                conv_counter = state.convergence_counter,
                "收敛分析"
            );

            // 更新状态
            state.prev_round_responses = round_responses;
            state.cumulative_info_gain.push(analysis.info_gain);

            // 判断是否应收敛（多条件任一满足即触发）
            let should_converge = if !analysis.has_novelty {
                true
            } else if analysis.avg_cross_round_sim > 0.65 {
                tracing::info!(sim = analysis.avg_cross_round_sim, "跨轮相似度高，触发收敛");
                true
            } else if analysis.agreement_ratio > 0.75 && round >= 2 {
                tracing::info!(agreement = analysis.agreement_ratio, "参与者高度一致，触发收敛");
                true
            } else if !state.cumulative_info_gain.is_empty() && Self::is_diminishing_returns(&state.cumulative_info_gain) {
                tracing::info!("信息增益边际递减，触发收敛");
                true
            } else {
                false
            };

            if should_converge {
                state.convergence_counter += 1;
                if state.convergence_counter >= config.convergence_rounds {
                    let _ = app_handle.emit(
                        "ai-orchestrator",
                        serde_json::json!({
                            "conversation_id": conversation_id,
                            "event": "converged",
                            "round": round,
                        }),
                    );
                    break;
                }
            } else {
                state.convergence_counter = 0;
            }
        }

        if state.current_round >= config.max_rounds {
            let _ = app_handle.emit(
                "ai-orchestrator",
                serde_json::json!({
                    "conversation_id": conversation_id,
                    "event": "round_limit",
                }),
            );
        }

        let summary = if state.current_round >= config.max_rounds {
            format!(
                "群聊达到最大轮次{}，共消耗{} tokens",
                config.max_rounds, state.total_tokens_used
            )
        } else {
            format!(
                "群聊在第{}轮收敛，共消耗{} tokens",
                state.current_round, state.total_tokens_used
            )
        };

        let now = chrono::Utc::now().timestamp_millis();
        chat_repo::add_message(
            pool,
            user_id,
            conversation_id,
            "system",
            None,
            &summary,
            state.current_round as i32 + 1,
            now,
        )
        .await?;

        chat_repo::update_conversation_timestamp(pool, conversation_id, user_id, now).await?;

        Ok(OrchestratorResult {
            total_rounds: state.current_round,
            total_tokens: state.total_tokens_used,
            summary,
        })
    }

    fn estimate_tokens(text: &str) -> i64 {
        (text.len() as i64) / 4
    }

    /// ── 多维度收敛分析方法（基于 RoundTable + Adaptive Stability Detection 论文）──

    /// 分析单轮响应，返回多维度指标
    fn analyze_round(responses: &[String], prev_responses: &[String]) -> RoundAnalysis {
        if responses.is_empty() {
            return RoundAnalysis {
                has_novelty: false,
                avg_cross_round_sim: 0.0,
                agreement_ratio: 0.0,
                info_gain: 0,
            };
        }

        // 1. 新观点检测（改进版关键词 + 长度启发式）
        let has_novelty = Self::contains_novelty(responses);

        // 2. 跨轮相似度（Information Distance 思路：Jaccard 字符 n-gram）
        let avg_cross_round_sim = if prev_responses.is_empty() || responses.is_empty() {
            0.0
        } else {
            let sims: Vec<f64> = responses
                .iter()
                .map(|r| Self::max_similarity_to_prev(r, prev_responses))
                .collect();
            sims.iter().sum::<f64>() / sims.len() as f64
        };

        // 3. 参与者间一致性（Dialogue Act 思路）
        let agreement_ratio = Self::compute_agreement_ratio(responses);

        // 4. 信息增益：与历史所有轮次比较，估算新增内容量
        let info_gain = Self::estimate_info_gain(responses, prev_responses);

        RoundAnalysis {
            has_novelty,
            avg_cross_round_sim,
            agreement_ratio,
            info_gain,
        }
    }

    /// 改进版新观点检测：结合关键词、内容多样性、回复独立性
    fn contains_novelty(responses: &[String]) -> bool {
        let new_arg_keywords = [
            "但是", "然而", "不过", "另一方面", "补充", "另外",
            "还值得注意的是", "更重要的是", "新的观点", "不同看法",
            "我认为", "建议", "反对", "不同意", "另一种思路",
            "可以尝试", "换个角度", "值得考虑", "其实", "实际上",
            "关键是", "核心在于", "从根本上",
            "but", "however", "different", "alternative", "another",
            "suggest", "disagree", "contrary", "instead", "crucial",
        ];

        let strong_agreement_keywords = [
            "同意", "没错", "确实如此", "完全赞同", "支持", "好主意",
            "没问题", "就这样", "可以",
            "agree", "exactly", "support", "good idea", "sure",
        ];

        for response in responses {
            let lower = response.to_lowercase();
            let new_count = new_arg_keywords.iter()
                .filter(|kw| lower.contains(&kw.to_lowercase()))
                .count();
            let agree_count = strong_agreement_keywords.iter()
                .filter(|kw| lower.contains(&kw.to_lowercase()))
                .count();

            // 有明确的新观点关键词 → 有 novelty
            if new_count > 0 { return true; }

            // 纯同意且短 → 无 novelty
            if agree_count > 0 && response.len() < 150 { continue; }

            // 足够长的回复可能包含实质内容
            if response.len() > 250 { return true; }
        }

        // 检查回复间多样性：如果各参与者回复差异大，说明有不同观点
        if responses.len() >= 2 {
            let diversity = Self::compute_response_diversity(responses);
            if diversity > 0.5 { return true; }
        }

        false
    }

    /// 计算 Jaccard 字符级 n-gram 相似度 (n=3)
    fn text_jaccard_sim(a: &str, b: &str) -> f64 {
        if a.is_empty() || b.is_empty() { return 0.0; }
        let n = 3;
        let get_ngrams = |s: &str| -> std::collections::HashSet<String> {
            s.chars().collect::<Vec<_>>().windows(n)
                .map(|w| w.iter().collect())
                .collect()
        };
        let set_a: std::collections::HashSet<String> = get_ngrams(a);
        let set_b: std::collections::HashSet<String> = get_ngrams(b);
        if set_a.is_empty() && set_b.is_empty() { return 1.0 }
        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();
        if union == 0 { return 0.0 }
        intersection as f64 / union as f64
    }

    /// 计算与上一轮所有响应的最大相似度
    fn max_similarity_to_prev(response: &str, prev_responses: &[String]) -> f64 {
        prev_responses.iter()
            .map(|p| Self::text_jaccard_sim(response, p))
            .fold(0.0_f64, f64::max)
    }

    /// 计算参与者间一致性比例（有多少比例的回复显示强一致信号）
    fn compute_agreement_ratio(responses: &[String]) -> f64 {
        if responses.is_empty() { return 0.0 }
        let agree_signals = [
            "同意", "没错", "确实", "赞同", "支持", "好主意", "没问题",
            "agree", "exactly", "support", "good idea", "sure", "ok",
            "是的", "对", "正确",
        ];
        let agree_count = responses.iter()
            .filter(|r| {
                let lower = r.to_lowercase();
                agree_signals.iter().any(|s| lower.contains(&s.to_lowercase()))
            })
            .count();
        agree_count as f64 / responses.len() as f64
    }

    /// 计算回复间多样性（平均 pairwise Jaccard distance）
    fn compute_response_diversity(responses: &[String]) -> f64 {
        if responses.len() < 2 { return 0.0 }
        let mut total_dist = 0.0_f64;
        let mut count = 0usize;
        for i in 0..responses.len() {
            for j in (i+1)..responses.len() {
                let sim = Self::text_jaccard_sim(&responses[i], &responses[j]);
                total_dist += 1.0 - sim;
                count += 1;
            }
        }
        if count == 0 { 0.0 } else { total_dist / count as f64 }
    }

    /// 估算信息增益：当前轮相对历史轮次的新增内容（字符数）
    fn estimate_info_gain(responses: &[String], prev_responses: &[String]) -> usize {
        if prev_responses.is_empty() {
            return responses.iter().map(|r| r.len()).sum();
        }
        // 将历史所有响应合并为参考文本
        let prev_combined = prev_responses.join(" ");
        responses.iter()
            .map(|r| {
                // 用简单方法：计算当前响应中不常出现在历史中的字符片段比例
                let sim = Self::text_jaccard_sim(r, &prev_combined);
                ((1.0 - sim) * r.len() as f64) as usize
            })
            .sum()
    }

    /// 检测信息边际递减：最近 N 轮的信息增益是否持续下降
    fn is_diminishing_returns(gains: &[usize]) -> bool {
        if gains.len() < 3 { return false }
        let recent = &gains[gains.len().saturating_sub(3)..];
        // 连续下降或持平
        recent.windows(2).all(|w| w[0] <= w[1] + 20)
            && *recent.last().unwrap_or(&0) < gains[..gains.len()-3].iter().sum::<usize>() / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.convergence_rounds, 2);
        assert_eq!(config.token_budget, 50000);
        assert!((config.token_warn_ratio - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        let tokens = AiOrchestrator::estimate_tokens("");
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_estimate_tokens_simple() {
        let text = "Hello, World!";
        let tokens = AiOrchestrator::estimate_tokens(text);
        assert_eq!(tokens, 3); // "Hello, World!" is 13 chars / 4 = 3
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let text = "你好世界";
        let tokens = AiOrchestrator::estimate_tokens(text);
        // Chinese characters are multi-byte
        assert!(tokens > 0);
    }

    #[test]
    fn test_contains_novelty_with_keyword() {
        let responses = vec!["我认为这个方案有三个优点：第一...".to_string()];
        assert!(AiOrchestrator::contains_novelty(&responses));
    }

    #[test]
    fn test_contains_novelty_agreement_only() {
        let responses = vec!["同意上述观点。".to_string()];
        // 纯同意且短 → 无 novelty
        assert!(!AiOrchestrator::contains_novelty(&responses));
    }

    #[test]
    fn test_text_jaccard_sim() {
        let sim = AiOrchestrator::text_jaccard_sim("你好世界", "你好世界");
        assert!(sim > 0.9);
        let sim_diff = AiOrchestrator::text_jaccard_sim("你好世界", "完全不同的内容");
        assert!(sim_diff < 0.5);
    }
}