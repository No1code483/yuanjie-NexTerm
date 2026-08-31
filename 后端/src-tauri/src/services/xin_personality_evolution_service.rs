use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::models::xin::{PersonaTrait, SpeakingStyle};
use crate::services::xin_michelin_service::{EvalDimension, EvalResult};

pub struct XinPersonalityEvolutionService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityVariant {
    pub id: String,
    pub name: String,
    pub base_persona_id: String,
    pub speaking_style: SpeakingStyle,
    pub traits: Vec<PersonaTrait>,
    pub created_at: String,
    pub is_active: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionRule {
    pub name: String,
    pub eval_dimension: EvalDimension,
    pub condition: RuleCondition,
    pub action: ParameterAdjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleCondition {
    BelowThreshold {
        threshold: f64,
        consecutive_count: u32,
    },
    TrendDeclining {
        window_size: u32,
        decline_rate: f64,
    },
    TrendImproving {
        window_size: u32,
        improve_rate: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterAdjustment {
    pub target_param: String,
    pub direction: f64,
    pub step_size: f64,
    pub max_change: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSnapshot {
    pub id: String,
    pub persona_id: String,
    pub variant_id: Option<String>,
    pub speaking_style: SpeakingStyle,
    pub traits: Vec<PersonaTrait>,
    pub saved_at: String,
    pub label: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionDecision {
    pub should_evolve: bool,
    pub adjustments: Vec<AppliedAdjustment>,
    pub reason: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedAdjustment {
    pub param: String,
    pub old_value: f64,
    pub new_value: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionTimelineEntry {
    pub id: String,
    pub timestamp: String,
    pub persona_id: String,
    pub previous_style: SpeakingStyle,
    pub new_style: SpeakingStyle,
    pub reason: String,
    pub eval_snapshot: Option<EvalMoment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMoment {
    pub avg_accuracy: f64,
    pub avg_empathy: f64,
    pub avg_completeness: f64,
    pub avg_creativity: f64,
    pub avg_timeliness: f64,
    pub overall_score: f64,
    pub sample_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantCompareResult {
    pub variant_a: VariantScore,
    pub variant_b: VariantScore,
    pub winner: Option<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantScore {
    pub variant_name: String,
    pub avg_overall: f64,
    pub avg_empathy: f64,
    pub avg_accuracy: f64,
    pub sample_count: usize,
}

impl XinPersonalityEvolutionService {
    pub fn builtin_rules() -> Vec<EvolutionRule> {
        vec![
            EvolutionRule {
                name: "共情下降→提升共情参数".into(),
                eval_dimension: EvalDimension::Empathy,
                condition: RuleCondition::BelowThreshold {
                    threshold: 35.0,
                    consecutive_count: 5,
                },
                action: ParameterAdjustment {
                    target_param: "empathy".into(),
                    direction: 1.0,
                    step_size: 0.05,
                    max_change: 0.25,
                },
            },
            EvolutionRule {
                name: "完整性持续低→增加详细度".into(),
                eval_dimension: EvalDimension::Completeness,
                condition: RuleCondition::BelowThreshold {
                    threshold: 40.0,
                    consecutive_count: 5,
                },
                action: ParameterAdjustment {
                    target_param: "verbosity".into(),
                    direction: 1.0,
                    step_size: 0.05,
                    max_change: 0.2,
                },
            },
            EvolutionRule {
                name: "创意性下降→提升幽默参数".into(),
                eval_dimension: EvalDimension::Creativity,
                condition: RuleCondition::TrendDeclining {
                    window_size: 10,
                    decline_rate: 5.0,
                },
                action: ParameterAdjustment {
                    target_param: "humor".into(),
                    direction: 1.0,
                    step_size: 0.05,
                    max_change: 0.2,
                },
            },
            EvolutionRule {
                name: "准确度持续高→收紧正式度".into(),
                eval_dimension: EvalDimension::Accuracy,
                condition: RuleCondition::TrendImproving {
                    window_size: 10,
                    improve_rate: 3.0,
                },
                action: ParameterAdjustment {
                    target_param: "formality".into(),
                    direction: -1.0,
                    step_size: 0.03,
                    max_change: 0.1,
                },
            },
            EvolutionRule {
                name: "共情持续高→降低技术深度".into(),
                eval_dimension: EvalDimension::Empathy,
                condition: RuleCondition::TrendImproving {
                    window_size: 10,
                    improve_rate: 5.0,
                },
                action: ParameterAdjustment {
                    target_param: "technical_depth".into(),
                    direction: -1.0,
                    step_size: 0.03,
                    max_change: 0.15,
                },
            },
        ]
    }

    pub fn analyze_evolution(results: &[EvalResult], current_style: &SpeakingStyle) -> EvolutionDecision {
        if results.len() < 5 {
            return EvolutionDecision {
                should_evolve: false,
                adjustments: Vec::new(),
                reason: format!("数据不足：需要至少 5 次评估，当前 {} 次", results.len()),
                confidence: 0.0,
            };
        }

        let rules = Self::builtin_rules();
        let mut adjustments = Vec::new();
        let mut reasons = Vec::new();

        if let Some(emp_score) = Self::dimension_average(results, &EvalDimension::Empathy) {
            if emp_score < 35.0 {
                if let Some(count) = Self::consecutive_below(results, &EvalDimension::Empathy, 35.0) {
                    if count >= 5 {
                        let rule = &rules[0];
                        let new_val = Self::apply_adjustment(&mut adjustments, current_style, &rule.action, &rule.name);
                        if new_val.is_some() {
                            reasons.push(format!("共情评分持续低于35（最近{}次），建议提升共情参数", count));
                        }
                    }
                }
            }
        }

        if let Some(comp_score) = Self::dimension_average(results, &EvalDimension::Completeness) {
            if comp_score < 40.0 {
                if let Some(count) = Self::consecutive_below(results, &EvalDimension::Completeness, 40.0) {
                    if count >= 5 {
                        let rule = &rules[1];
                        let new_val = Self::apply_adjustment(&mut adjustments, current_style, &rule.action, &rule.name);
                        if new_val.is_some() {
                            reasons.push(format!("完整性评分持续低于40（最近{}次），建议增加详细度", count));
                        }
                    }
                }
            }
        }

        if let Some(trend) = Self::trend_analysis(results, &EvalDimension::Creativity) {
            if trend < -5.0 {
                let rule = &rules[2];
                let new_val = Self::apply_adjustment(&mut adjustments, current_style, &rule.action, &rule.name);
                if new_val.is_some() {
                    reasons.push(format!("创意性呈下降趋势（斜率{:.1}），建议提升幽默参数", trend));
                }
            }
        }

        if let Some(trend) = Self::trend_analysis(results, &EvalDimension::Accuracy) {
            if trend > 3.0 {
                let rule = &rules[3];
                let new_val = Self::apply_adjustment(&mut adjustments, current_style, &rule.action, &rule.name);
                if new_val.is_some() {
                    reasons.push(format!("准确度持续提升（斜率{:.1}），可适度降低正式度", trend));
                }
            }
        }

        if let Some(trend) = Self::trend_analysis(results, &EvalDimension::Empathy) {
            if trend > 5.0 {
                if current_style.technical_depth > 0.4 {
                    let rule = &rules[4];
                    let new_val = Self::apply_adjustment(&mut adjustments, current_style, &rule.action, &rule.name);
                    if new_val.is_some() {
                        reasons.push(format!("共情持续提升（斜率{:.1}），可降低技术深度增加亲和力", trend));
                    }
                }
            }
        }

        let should_evolve = !adjustments.is_empty();
        let confidence = if should_evolve {
            (adjustments.len() as f64 / 3.0).min(0.95)
        } else {
            0.0
        };

        EvolutionDecision {
            should_evolve,
            adjustments,
            reason: if reasons.is_empty() {
                "当前人格参数与评估结果匹配良好，暂无需调整".into()
            } else {
                reasons.join("；")
            },
            confidence,
        }
    }

    fn apply_adjustment(
        adjustments: &mut Vec<AppliedAdjustment>,
        style: &SpeakingStyle,
        action: &ParameterAdjustment,
        reason: &str,
    ) -> Option<f64> {
        let current = Self::get_style_param(style, &action.target_param);
        let delta = action.step_size * action.direction;
        let new_val = (current + delta).clamp(0.1, 1.0);

        let max_allowed = if action.direction > 0.0 {
            (current + action.max_change).min(1.0)
        } else {
            (current - action.max_change).max(0.1)
        };

        let capped = if action.direction > 0.0 {
            new_val.min(max_allowed)
        } else {
            new_val.max(max_allowed)
        };

        if (capped - current).abs() < 0.01 {
            return None;
        }

        adjustments.push(AppliedAdjustment {
            param: action.target_param.clone(),
            old_value: current,
            new_value: capped,
            reason: reason.to_string(),
        });

        Some(capped)
    }

    fn get_style_param(style: &SpeakingStyle, param: &str) -> f64 {
        match param {
            "formality" => style.formality,
            "verbosity" => style.verbosity,
            "humor" => style.humor,
            "technical_depth" => style.technical_depth,
            "empathy" => style.empathy,
            _ => 0.5,
        }
    }

    pub fn apply_decision(style: &SpeakingStyle, decision: &EvolutionDecision) -> SpeakingStyle {
        let mut new_style = style.clone();
        for adj in &decision.adjustments {
            match adj.param.as_str() {
                "formality" => new_style.formality = adj.new_value,
                "verbosity" => new_style.verbosity = adj.new_value,
                "humor" => new_style.humor = adj.new_value,
                "technical_depth" => new_style.technical_depth = adj.new_value,
                "empathy" => new_style.empathy = adj.new_value,
                _ => {}
            }
        }
        new_style
    }

    pub fn create_variant(
        base_persona_id: &str,
        name: &str,
        base_style: &SpeakingStyle,
        base_traits: &[PersonaTrait],
        adjustments: &[AppliedAdjustment],
        description: &str,
    ) -> PersonalityVariant {
        let decision = EvolutionDecision {
            should_evolve: true,
            adjustments: adjustments.to_vec(),
            reason: String::new(),
            confidence: 1.0,
        };
        let new_style = Self::apply_decision(base_style, &decision);
        PersonalityVariant {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            base_persona_id: base_persona_id.to_string(),
            speaking_style: new_style,
            traits: base_traits.to_vec(),
            created_at: Utc::now().to_rfc3339(),
            is_active: false,
            description: description.to_string(),
        }
    }

    pub fn compare_variants(
        variant_a: &PersonalityVariant,
        variant_b: &PersonalityVariant,
        eval_a: &[EvalResult],
        eval_b: &[EvalResult],
    ) -> VariantCompareResult {
        let score_a = VariantScore {
            variant_name: variant_a.name.clone(),
            avg_overall: eval_a.iter().map(|r| r.overall_score as f64).sum::<f64>()
                / eval_a.len().max(1) as f64,
            avg_empathy: Self::dimension_average(eval_a, &EvalDimension::Empathy).unwrap_or(0.0),
            avg_accuracy: Self::dimension_average(eval_a, &EvalDimension::Accuracy).unwrap_or(0.0),
            sample_count: eval_a.len(),
        };
        let score_b = VariantScore {
            variant_name: variant_b.name.clone(),
            avg_overall: eval_b.iter().map(|r| r.overall_score as f64).sum::<f64>()
                / eval_b.len().max(1) as f64,
            avg_empathy: Self::dimension_average(eval_b, &EvalDimension::Empathy).unwrap_or(0.0),
            avg_accuracy: Self::dimension_average(eval_b, &EvalDimension::Accuracy).unwrap_or(0.0),
            sample_count: eval_b.len(),
        };

        let winner = if score_a.avg_overall > score_b.avg_overall + 3.0 {
            Some(score_a.variant_name.clone())
        } else if score_b.avg_overall > score_a.avg_overall + 3.0 {
            Some(score_b.variant_name.clone())
        } else {
            None
        };

        let recommendation = match &winner {
            Some(w) => format!("推荐使用「{}」，综合评分更高", w),
            None => {
                if score_a.avg_overall >= score_b.avg_overall {
                    format!("两个变体评分接近，「{}」略优", score_a.variant_name)
                } else {
                    format!("两个变体评分接近，「{}」略优", score_b.variant_name)
                }
            }
        };

        VariantCompareResult {
            variant_a: score_a,
            variant_b: score_b,
            winner,
            recommendation,
        }
    }

    pub fn snapshot(
        persona_id: &str,
        variant_id: Option<&str>,
        style: &SpeakingStyle,
        traits: &[PersonaTrait],
        label: &str,
    ) -> EvolutionSnapshot {
        EvolutionSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            persona_id: persona_id.to_string(),
            variant_id: variant_id.map(|s| s.to_string()),
            speaking_style: style.clone(),
            traits: traits.to_vec(),
            saved_at: Utc::now().to_rfc3339(),
            label: label.to_string(),
            trigger: "manual".to_string(),
        }
    }

    pub fn make_eval_moment(results: &[EvalResult]) -> EvalMoment {
        let n = results.len().max(1);
        EvalMoment {
            avg_accuracy: Self::dimension_average(results, &EvalDimension::Accuracy).unwrap_or(0.0),
            avg_empathy: Self::dimension_average(results, &EvalDimension::Empathy).unwrap_or(0.0),
            avg_completeness: Self::dimension_average(results, &EvalDimension::Completeness).unwrap_or(0.0),
            avg_creativity: Self::dimension_average(results, &EvalDimension::Creativity).unwrap_or(0.0),
            avg_timeliness: Self::dimension_average(results, &EvalDimension::Timeliness).unwrap_or(0.0),
            overall_score: results.iter().map(|r| r.overall_score as f64).sum::<f64>() / n as f64,
            sample_count: results.len() as u32,
        }
    }

    fn dimension_average(results: &[EvalResult], dim: &EvalDimension) -> Option<f64> {
        let scores: Vec<u32> = results
            .iter()
            .filter_map(|r| {
                r.dimensions
                    .iter()
                    .find(|d| d.dimension == *dim)
                    .map(|d| d.score)
            })
            .collect();
        if scores.is_empty() {
            return None;
        }
        Some(scores.iter().sum::<u32>() as f64 / scores.len() as f64)
    }

    fn consecutive_below(results: &[EvalResult], dim: &EvalDimension, threshold: f64) -> Option<u32> {
        let mut count = 0u32;
        for r in results.iter().rev() {
            if let Some(d) = r.dimensions.iter().find(|d| d.dimension == *dim) {
                if (d.score as f64) < threshold {
                    count += 1;
                } else {
                    break;
                }
            }
        }
        Some(count)
    }

    fn trend_analysis(results: &[EvalResult], dim: &EvalDimension) -> Option<f64> {
        if results.len() < 5 {
            return None;
        }
        let recent: Vec<u32> = results
            .iter()
            .rev()
            .take(10)
            .filter_map(|r| r.dimensions.iter().find(|d| d.dimension == *dim).map(|d| d.score))
            .collect();
        if recent.len() < 5 {
            return None;
        }
        let n = recent.len() as f64;
        let mean_x = (n - 1.0) / 2.0;
        let mean_y = recent.iter().sum::<u32>() as f64 / n;
        let mut num = 0.0;
        let mut den = 0.0;
        for (i, &score) in recent.iter().enumerate() {
            let x = i as f64;
            num += (x - mean_x) * (score as f64 - mean_y);
            den += (x - mean_x).powi(2);
        }
        if den == 0.0 {
            return Some(0.0);
        }
        Some(num / den)
    }

    pub fn create_timeline_entry(
        persona_id: &str,
        previous: &SpeakingStyle,
        new: &SpeakingStyle,
        reason: &str,
        eval: Option<&[EvalResult]>,
    ) -> EvolutionTimelineEntry {
        EvolutionTimelineEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now().to_rfc3339(),
            persona_id: persona_id.to_string(),
            previous_style: previous.clone(),
            new_style: new.clone(),
            reason: reason.to_string(),
            eval_snapshot: eval.map(|r| Self::make_eval_moment(r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::xin_michelin_service::{DimensionScore, EvalDimension, EvalResult};

    fn make_result(empathy: u32, completeness: u32, accuracy: u32, creativity: u32) -> EvalResult {
        EvalResult {
            id: uuid::Uuid::new_v4().to_string(),
            dimensions: vec![
                DimensionScore { dimension: EvalDimension::Empathy, score: empathy, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Completeness, score: completeness, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Accuracy, score: accuracy, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Creativity, score: creativity, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Timeliness, score: 70, reason: "".into() },
            ],
            overall_score: (empathy + completeness + accuracy + creativity + 70) / 5,
            stars: 2,
            star_label: "⭐⭐".into(),
            strengths: vec![],
            weaknesses: vec![],
            improvement_suggestion: "".into(),
            evaluated_at: Utc::now().to_rfc3339(),
            response_time_ms: 1000,
        }
    }

    #[test]
    fn test_no_evolution_with_good_scores() {
        let results: Vec<EvalResult> = (0..10)
            .map(|_| make_result(70, 70, 80, 60))
            .collect();
        let style = SpeakingStyle::default();
        let decision = XinPersonalityEvolutionService::analyze_evolution(&results, &style);
        assert!(!decision.should_evolve);
    }

    #[test]
    fn test_evolution_low_empathy() {
        let results: Vec<EvalResult> = (0..8)
            .map(|_| make_result(25, 60, 70, 50))
            .collect();
        let style = SpeakingStyle::default();
        let decision = XinPersonalityEvolutionService::analyze_evolution(&results, &style);
        assert!(decision.should_evolve);
        assert!(decision.adjustments.iter().any(|a| a.param == "empathy"));
    }

    #[test]
    fn test_evolution_low_completeness() {
        let results: Vec<EvalResult> = (0..8)
            .map(|_| make_result(70, 30, 70, 50))
            .collect();
        let style = SpeakingStyle::default();
        let decision = XinPersonalityEvolutionService::analyze_evolution(&results, &style);
        assert!(decision.should_evolve);
        assert!(decision.adjustments.iter().any(|a| a.param == "verbosity"));
    }

    #[test]
    fn test_not_enough_data() {
        let results: Vec<EvalResult> = (0..3)
            .map(|_| make_result(20, 30, 40, 30))
            .collect();
        let style = SpeakingStyle::default();
        let decision = XinPersonalityEvolutionService::analyze_evolution(&results, &style);
        assert!(!decision.should_evolve);
        assert!(decision.reason.contains("数据不足"));
    }

    #[test]
    fn test_apply_decision() {
        let style = SpeakingStyle::default();
        let decision = EvolutionDecision {
            should_evolve: true,
            adjustments: vec![AppliedAdjustment {
                param: "empathy".into(),
                old_value: 0.5,
                new_value: 0.7,
                reason: "测试".into(),
            }],
            reason: "test".into(),
            confidence: 0.8,
        };
        let new_style = XinPersonalityEvolutionService::apply_decision(&style, &decision);
        assert!((new_style.empathy - 0.7).abs() < 0.01);
        assert_eq!(new_style.formality, style.formality);
    }

    #[test]
    fn test_create_variant() {
        let style = SpeakingStyle::default();
        let traits = vec![PersonaTrait { name: "逻辑".into(), value: 0.9 }];
        let adj = vec![AppliedAdjustment {
            param: "humor".into(),
            old_value: 0.3,
            new_value: 0.6,
            reason: "创意提升".into(),
        }];
        let variant = XinPersonalityEvolutionService::create_variant(
            "code_assistant", "幽默版代码助手", &style, &traits, &adj, "增加幽默感的变体",
        );
        assert_eq!(variant.base_persona_id, "code_assistant");
        assert!((variant.speaking_style.humor - 0.6).abs() < 0.01);
        assert!(!variant.is_active);
    }

    #[test]
    fn test_snapshot() {
        let style = SpeakingStyle::default();
        let traits = vec![];
        let snap = XinPersonalityEvolutionService::snapshot(
            "code_assistant", None, &style, &traits, "手动快照",
        );
        assert_eq!(snap.persona_id, "code_assistant");
        assert_eq!(snap.label, "手动快照");
        assert_eq!(snap.trigger, "manual");
    }

    #[test]
    fn test_compare_variants() {
        let style_a = SpeakingStyle::default();
        let style_b = SpeakingStyle { empathy: 0.9, ..SpeakingStyle::default() };
        let traits = vec![];
        let va = XinPersonalityEvolutionService::create_variant(
            "test", "A", &style_a, &traits, &[], "A",
        );
        let vb = PersonalityVariant {
            id: "b".into(),
            name: "B".into(),
            base_persona_id: "test".into(),
            speaking_style: style_b,
            traits: vec![],
            created_at: Utc::now().to_rfc3339(),
            is_active: false,
            description: "B".into(),
        };
        let eval_a: Vec<EvalResult> = (0..5).map(|_| make_result(60, 60, 70, 60)).collect();
        let eval_b: Vec<EvalResult> = (0..5).map(|_| make_result(80, 70, 80, 70)).collect();
        let result = XinPersonalityEvolutionService::compare_variants(&va, &vb, &eval_a, &eval_b);
        assert_eq!(result.winner, Some("B".into()));
    }

    #[test]
    fn test_trend_analysis() {
        let results: Vec<EvalResult> = vec![
            make_result(60, 60, 80, 70),
            make_result(60, 60, 78, 68),
            make_result(60, 60, 75, 65),
            make_result(60, 60, 72, 62),
            make_result(60, 60, 70, 60),
            make_result(60, 60, 68, 58),
        ];
        let trend = XinPersonalityEvolutionService::trend_analysis(&results, &EvalDimension::Creativity);
        assert!(trend.is_some());
    }

    #[test]
    fn test_consecutive_below() {
        let results: Vec<EvalResult> = vec![
            make_result(20, 60, 70, 50),
            make_result(25, 60, 70, 50),
            make_result(15, 60, 70, 50),
            make_result(30, 60, 70, 50),
            make_result(10, 60, 70, 50),
            make_result(20, 60, 70, 50),
            make_result(18, 60, 70, 50),
        ];
        let count = XinPersonalityEvolutionService::consecutive_below(&results, &EvalDimension::Empathy, 35.0);
        assert_eq!(count, Some(7));
    }
}