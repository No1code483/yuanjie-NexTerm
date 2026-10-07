use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvalDimension {
    Accuracy,
    Empathy,
    Completeness,
    Creativity,
    Timeliness,
}

impl EvalDimension {
    pub fn label(&self) -> &str {
        match self {
            EvalDimension::Accuracy => "准确性",
            EvalDimension::Empathy => "共情度",
            EvalDimension::Completeness => "完整性",
            EvalDimension::Creativity => "创意性",
            EvalDimension::Timeliness => "时效性",
        }
    }

    pub fn all() -> Vec<EvalDimension> {
        vec![
            EvalDimension::Accuracy,
            EvalDimension::Empathy,
            EvalDimension::Completeness,
            EvalDimension::Creativity,
            EvalDimension::Timeliness,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub dimension: EvalDimension,
    pub score: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    pub id: String,
    pub dimensions: Vec<DimensionScore>,
    pub overall_score: u32,
    pub stars: u32,
    pub star_label: String,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub improvement_suggestion: String,
    pub evaluated_at: String,
    pub response_time_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalStats {
    pub total_evals: usize,
    pub average_score: f64,
    pub average_stars: f64,
    pub dimension_averages: Vec<DimensionAverage>,
    pub star_distribution: StarDistribution,
    pub trend: String,
    pub trend_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionAverage {
    pub dimension: EvalDimension,
    pub label: String,
    pub average: f64,
    pub recent_trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarDistribution {
    pub one_star: usize,
    pub two_star: usize,
    pub three_star: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub generated_at: String,
    pub stats: EvalStats,
    pub weakest_dimensions: Vec<String>,
    pub strongest_dimensions: Vec<String>,
    pub recent_improvements: Vec<String>,
    pub recent_declines: Vec<String>,
    pub overall_assessment: String,
    pub action_recommendations: Vec<String>,
}

pub struct XinMichelinService;

impl XinMichelinService {
    pub fn evaluate_dialogue(
        user_msg: &str,
        assistant_msg: &str,
        response_time_ms: u32,
    ) -> EvalResult {
        let accuracy = Self::eval_accuracy(user_msg, assistant_msg);
        let empathy = Self::eval_empathy(user_msg, assistant_msg);
        let completeness = Self::eval_completeness(user_msg, assistant_msg);
        let creativity = Self::eval_creativity(assistant_msg);
        let timeliness = Self::eval_timeliness(response_time_ms);

        let dimensions = vec![accuracy, empathy, completeness, creativity, timeliness];

        let overall_score = dimensions.iter().map(|d| d.score).sum::<u32>() / dimensions.len() as u32;
        let (stars, star_label) = Self::star_rating(overall_score);

        let strengths: Vec<String> = dimensions
            .iter()
            .filter(|d| d.score >= 70)
            .map(|d| d.dimension.label().to_string())
            .collect();

        let weaknesses: Vec<String> = dimensions
            .iter()
            .filter(|d| d.score < 40)
            .map(|d| d.dimension.label().to_string())
            .collect();

        let improvement_suggestion = Self::build_suggestion(&dimensions);

        EvalResult {
            id: uuid::Uuid::new_v4().to_string(),
            dimensions,
            overall_score,
            stars,
            star_label,
            strengths,
            weaknesses,
            improvement_suggestion,
            evaluated_at: Utc::now().to_rfc3339(),
            response_time_ms,
        }
    }

    pub fn aggregate_stats(results: &[EvalResult]) -> EvalStats {
        let total_evals = results.len();
        if total_evals == 0 {
            return EvalStats {
                total_evals: 0,
                average_score: 0.0,
                average_stars: 0.0,
                dimension_averages: vec![],
                star_distribution: StarDistribution { one_star: 0, two_star: 0, three_star: 0 },
                trend: "stable".to_string(),
                trend_score: 0.0,
            };
        }

        let average_score = results.iter().map(|r| r.overall_score as f64).sum::<f64>() / total_evals as f64;
        let average_stars = results.iter().map(|r| r.stars as f64).sum::<f64>() / total_evals as f64;

        let dimension_averages: Vec<DimensionAverage> = EvalDimension::all()
            .iter()
            .map(|dim| {
                let scores: Vec<u32> = results
                    .iter()
                    .flat_map(|r| r.dimensions.iter().filter(|d| d.dimension == *dim).map(|d| d.score))
                    .collect();
                let avg = if scores.is_empty() {
                    0.0
                } else {
                    scores.iter().map(|s| *s as f64).sum::<f64>() / scores.len() as f64
                };
                let recent_trend = if scores.len() >= 2 {
                    let recent: Vec<u32> = scores.iter().rev().take(3).copied().collect();
                    if recent.len() >= 2 && recent[0] > recent[1] {
                        "improving".to_string()
                    } else if recent.len() >= 2 && recent[0] < recent[1] {
                        "declining".to_string()
                    } else {
                        "stable".to_string()
                    }
                } else {
                    "stable".to_string()
                };
                DimensionAverage {
                    dimension: dim.clone(),
                    label: dim.label().to_string(),
                    average: avg,
                    recent_trend,
                }
            })
            .collect();

        let one_star = results.iter().filter(|r| r.stars == 1).count();
        let two_star = results.iter().filter(|r| r.stars == 2).count();
        let three_star = results.iter().filter(|r| r.stars >= 3).count();

        let (trend, trend_score) = Self::calc_trend(results);

        EvalStats {
            total_evals,
            average_score,
            average_stars,
            dimension_averages,
            star_distribution: StarDistribution { one_star, two_star, three_star },
            trend,
            trend_score,
        }
    }

    pub fn generate_report(results: &[EvalResult]) -> EvalReport {
        let stats = Self::aggregate_stats(results);

        let mut dim_scores: Vec<(&DimensionAverage, f64)> = stats
            .dimension_averages
            .iter()
            .map(|d| (d, d.average))
            .collect();
        dim_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let weakest_dimensions: Vec<String> = dim_scores.iter().take(2).map(|(d, _)| d.label.clone()).collect();
        let strongest_dimensions: Vec<String> = dim_scores.iter().rev().take(2).map(|(d, _)| d.label.clone()).collect();

        let recent_improvements: Vec<String> = stats
            .dimension_averages
            .iter()
            .filter(|d| d.recent_trend == "improving")
            .map(|d| format!("{} 近期呈上升趋势", d.label))
            .collect();

        let recent_declines: Vec<String> = stats
            .dimension_averages
            .iter()
            .filter(|d| d.recent_trend == "declining")
            .map(|d| format!("{} 近期呈下降趋势", d.label))
            .collect();

        let overall_assessment = Self::build_overall(&stats);
        let action_recommendations = Self::build_recommendations(&stats);

        EvalReport {
            generated_at: Utc::now().to_rfc3339(),
            stats,
            weakest_dimensions,
            strongest_dimensions,
            recent_improvements,
            recent_declines,
            overall_assessment,
            action_recommendations,
        }
    }

    fn eval_accuracy(user_msg: &str, assistant_msg: &str) -> DimensionScore {
        let mut score: u32 = 50;
        let mut reasons: Vec<&str> = Vec::new();

        let user_has_question = user_msg.contains('?')
            || user_msg.contains('？')
            || user_msg.ends_with('吗')
            || user_msg.ends_with('呢')
            || user_msg.contains("什么")
            || user_msg.contains("怎么")
            || user_msg.contains("如何")
            || user_msg.contains("为什么")
            || user_msg.contains("谁");

        if user_has_question {
            if !assistant_msg.is_empty() && assistant_msg.len() > 20 {
                score += 20;
                reasons.push("对用户提问有实质性回答");
            } else {
                score -= 20;
                reasons.push("对用户问题的回答偏短");
            }
        } else {
            score += 5;
            reasons.push("用户未明确提问");
        }

        let knowledge_indicators = ["是", "有", "可以", "需要", "应该", "会", "能", "因为", "所以", "根据", "按照"];
        let match_count = knowledge_indicators.iter().filter(|kw| assistant_msg.contains(*kw)).count();
        if match_count >= 3 {
            score += 15;
            reasons.push("回答包含较多确定性词汇");
        } else if match_count == 0 && assistant_msg.len() > 30 {
            score -= 10;
            reasons.push("回答缺乏确定性表述");
        }

        let vague_words = ["可能", "大概", "也许", "或许", "不太确定", "不清楚", "不知道"];
        let vague_count = vague_words.iter().filter(|kw| assistant_msg.contains(*kw)).count();
        if vague_count >= 2 {
            score -= 15;
            reasons.push("回答含糊其辞");
        }

        score = score.max(0).min(100);
        let reason = if reasons.is_empty() {
            format!("综合评价：{}/100", score)
        } else {
            format!("{}。综合评价：{}/100", reasons.join("；"), score)
        };

        DimensionScore { dimension: EvalDimension::Accuracy, score, reason }
    }

    fn eval_empathy(user_msg: &str, assistant_msg: &str) -> DimensionScore {
        let mut score: u32 = 50;
        let mut reasons: Vec<&str> = Vec::new();

        let user_emotional = [
            "难过", "不开心", "压力", "累", "烦", "焦虑", "失眠", "困",
            "不舒服", "生病", "担心", "害怕", "孤独", "想哭", "开心", "高兴",
            "兴奋", "激动", "感动", "期待", "紧张", "失望", "沮丧", "生气",
        ];
        let user_has_emotion = user_emotional.iter().any(|kw| user_msg.contains(kw));

        if user_has_emotion {
            let empathy_phrases = [
                "理解", "感受到", "知道", "明白", "陪伴", "支持", "放松",
                "没关系", "别担心", "辛苦了", "加油", "抱抱", "温暖",
            ];
            let empathy_count = empathy_phrases.iter().filter(|kw| assistant_msg.contains(*kw)).count();
            if empathy_count >= 2 {
                score += 30;
                reasons.push("对用户情绪有充分回应");
            } else if empathy_count >= 1 {
                score += 15;
                reasons.push("对用户情绪有一定回应");
            } else {
                score -= 20;
                reasons.push("用户表达了情绪但未得到共情回应");
            }
        } else {
            score += 10;
            reasons.push("用户无明显情绪表达");
        }

        let assistant_care = ["理解你", "感受到", "明白你", "陪伴", "支持", "辛苦了", "加油", "温暖", "关心", "关怀"];
        if assistant_care.iter().any(|kw| assistant_msg.contains(kw)) {
            score += 10;
            reasons.push("回答中主动表达了关怀");
        }

        score = score.max(0).min(100);
        let reason = if reasons.is_empty() {
            format!("综合评价：{}/100", score)
        } else {
            format!("{}。综合评价：{}/100", reasons.join("；"), score)
        };

        DimensionScore { dimension: EvalDimension::Empathy, score, reason }
    }

    fn eval_completeness(user_msg: &str, assistant_msg: &str) -> DimensionScore {
        let mut score: u32 = 50;
        let mut reasons: Vec<&str> = Vec::new();

        if assistant_msg.len() > 200 {
            score += 20;
            reasons.push("回答详实");
        } else if assistant_msg.len() > 80 {
            score += 10;
            reasons.push("回答篇幅适中");
        } else if assistant_msg.len() < 20 {
            score -= 15;
            reasons.push("回答过于简短");
        }

        let user_sub = user_msg.matches('?').count()
            + user_msg.matches('？').count()
            + user_msg.matches('。').count();
        let has_sub_items = user_sub > 1
            || user_msg.contains("1.")
            || user_msg.contains("2.")
            || user_msg.contains("首先")
            || user_msg.contains("其次")
            || user_msg.contains("还有")
            || user_msg.contains("另外");

        if has_sub_items {
            let coverage = ["首先", "其次", "第一", "第二", "另外", "还有", "同时", "此外"];
            let coverage_count = coverage.iter().filter(|kw| assistant_msg.contains(*kw)).count();
            if coverage_count >= 2 {
                score += 15;
                reasons.push("对多项子问题都有覆盖");
            } else {
                score -= 10;
                reasons.push("用户提出了多个要点但回答覆盖不足");
            }
        }

        let has_actionable = assistant_msg.contains("建议")
            || assistant_msg.contains("可以")
            || assistant_msg.contains("步骤")
            || assistant_msg.contains("方案")
            || assistant_msg.contains("总结");
        if has_actionable {
            score += 10;
            reasons.push("回答包含可执行建议");
        }

        score = score.max(0).min(100);
        let reason = if reasons.is_empty() {
            format!("综合评价：{}/100", score)
        } else {
            format!("{}。综合评价：{}/100", reasons.join("；"), score)
        };

        DimensionScore { dimension: EvalDimension::Completeness, score, reason }
    }

    fn eval_creativity(assistant_msg: &str) -> DimensionScore {
        let mut score: u32 = 50;
        let mut reasons: Vec<&str> = Vec::new();

        let creative_phrases = [
            "想象", "比喻", "换个角度", "有趣", "创意",
            "灵感", "独特", "新颖", "不妨", "试试",
            "例如", "类比", "假设", "如果", "换个思路",
        ];
        let creative_count = creative_phrases.iter().filter(|kw| assistant_msg.contains(*kw)).count();

        if creative_count >= 3 {
            score += 25;
            reasons.push("回答富有创意和启发性");
        } else if creative_count >= 1 {
            score += 10;
            reasons.push("回答有一定的创意元素");
        } else if assistant_msg.len() < 30 {
            score -= 5;
            reasons.push("回答过短难以体现创意");
        }

        if assistant_msg.contains("哈哈") || assistant_msg.contains("😄") || assistant_msg.contains("有趣") || assistant_msg.contains("幽默") {
            score += 10;
            reasons.push("回答带有轻松幽默感");
        }

        if assistant_msg.contains("比如") || assistant_msg.contains("例如") || assistant_msg.contains("举个例子") {
            score += 10;
            reasons.push("回答中使用了举例说明");
        }

        score = score.max(0).min(100);
        let reason = if reasons.is_empty() {
            format!("综合评价：{}/100", score)
        } else {
            format!("{}。综合评价：{}/100", reasons.join("；"), score)
        };

        DimensionScore { dimension: EvalDimension::Creativity, score, reason }
    }

    fn eval_timeliness(response_time_ms: u32) -> DimensionScore {
        let (score, reason) = if response_time_ms == 0 {
            (50, "未提供响应时间，默认评分".to_string())
        } else if response_time_ms < 2000 {
            (85, format!("响应时间为{}ms，非常迅速", response_time_ms))
        } else if response_time_ms < 5000 {
            (70, format!("响应时间为{}ms，可接受", response_time_ms))
        } else if response_time_ms < 10000 {
            (50, format!("响应时间为{}ms，偏慢", response_time_ms))
        } else {
            (30, format!("响应时间为{}ms，过慢影响体验", response_time_ms))
        };

        DimensionScore { dimension: EvalDimension::Timeliness, score, reason }
    }

    fn star_rating(score: u32) -> (u32, String) {
        match score {
            0..=33 => (1, "⭐".to_string()),
            34..=66 => (2, "⭐⭐".to_string()),
            _ => (3, "⭐⭐⭐".to_string()),
        }
    }

    fn build_suggestion(dimensions: &[DimensionScore]) -> String {
        let weak: Vec<&DimensionScore> = dimensions.iter().filter(|d| d.score < 40).collect();
        if weak.is_empty() {
            if dimensions.iter().all(|d| d.score >= 70) {
                return "表现优秀！保持当前水平。".to_string();
            }
            return "整体表现良好，部分维度还有提升空间。".to_string();
        }

        let tips: Vec<String> = weak
            .iter()
            .map(|d| match d.dimension {
                EvalDimension::Accuracy => "可尝试提供更确定、更具体的回答",
                EvalDimension::Empathy => "可加强对用户情绪的关注和回应",
                EvalDimension::Completeness => "可增加回答的深度和覆盖面",
                EvalDimension::Creativity => "可尝试使用比喻、举例等方式增加创意",
                EvalDimension::Timeliness => "可优化响应速度",
            })
            .map(|s| s.to_string())
            .collect();

        tips.join("；")
    }

    fn calc_trend(results: &[EvalResult]) -> (String, f64) {
        if results.len() < 3 {
            return ("stable".to_string(), 0.0);
        }

        let recent: Vec<u32> = results.iter().rev().take(5).map(|r| r.overall_score).collect();
        if recent.len() < 2 {
            return ("stable".to_string(), 0.0);
        }

        let diffs: Vec<i32> = recent.windows(2).map(|w| w[0] as i32 - w[1] as i32).collect();
        let trend_sum: i32 = diffs.iter().sum();
        let trend_score = trend_sum as f64 / diffs.len() as f64;

        if trend_score > 5.0 {
            ("improving".to_string(), trend_score)
        } else if trend_score < -5.0 {
            ("declining".to_string(), trend_score)
        } else {
            ("stable".to_string(), trend_score)
        }
    }

    fn build_overall(stats: &EvalStats) -> String {
        if stats.total_evals == 0 {
            return "尚无评测数据。".to_string();
        }

        let star_text = if stats.average_stars >= 2.5 {
            "表现优秀"
        } else if stats.average_stars >= 1.5 {
            "表现尚可"
        } else {
            "有待提升"
        };

        let trend_text = match stats.trend.as_str() {
            "improving" => "呈上升趋势",
            "declining" => "呈下降趋势",
            _ => "保持稳定",
        };

        let weakest = stats
            .dimension_averages
            .iter()
            .min_by(|a, b| a.average.partial_cmp(&b.average).unwrap_or(std::cmp::Ordering::Equal))
            .map(|d| d.label.as_str())
            .unwrap_or("无");

        let strongest = stats
            .dimension_averages
            .iter()
            .max_by(|a, b| a.average.partial_cmp(&b.average).unwrap_or(std::cmp::Ordering::Equal))
            .map(|d| d.label.as_str())
            .unwrap_or("无");

        format!(
            "共{}次测评，平均得分{:.1}分（{}），平均星级{:.1}，整体{}。最弱维度：{}，最强维度：{}。",
            stats.total_evals, stats.average_score, star_text,
            stats.average_stars, trend_text, weakest, strongest
        )
    }

    fn build_recommendations(stats: &EvalStats) -> Vec<String> {
        let mut recs = Vec::new();

        for dim in &stats.dimension_averages {
            if dim.average < 40.0 && dim.recent_trend == "declining" {
                recs.push(format!("⚠️ {}持续走低且近期下降，需要重点关注", dim.label));
            } else if dim.average < 40.0 {
                recs.push(format!("🔧 {}评分较低，建议针对性改进", dim.label));
            }
        }

        if stats.average_stars < 1.5 && stats.total_evals > 0 {
            recs.push("整体评分偏低，建议审查对话质量并优化回复策略".to_string());
        }

        if stats.trend == "declining" {
            recs.push("近期质量有下降趋势，建议排查可能的原因".to_string());
        }

        if recs.is_empty() && stats.total_evals > 0 {
            recs.push("各项表现稳定，继续保持".to_string());
        }

        recs
    }

    pub fn filter_recent(results: &[EvalResult], count: usize) -> Vec<EvalResult> {
        let mut sorted: Vec<EvalResult> = results.to_vec();
        sorted.sort_by(|a, b| b.evaluated_at.cmp(&a.evaluated_at));
        sorted.truncate(count);
        sorted
    }

    pub fn filter_by_period(results: &[EvalResult], start: DateTime<Utc>, end: DateTime<Utc>) -> Vec<EvalResult> {
        results
            .iter()
            .filter(|r| {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&r.evaluated_at) {
                    let t = t.with_timezone(&Utc);
                    t >= start && t <= end
                } else {
                    false
                }
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_high_quality() {
        let result = XinMichelinService::evaluate_dialogue(
            "如何学习Rust编程语言？",
            "学习Rust需要循序渐进。首先应该掌握所有权系统，这是最核心的概念。\
             其次需要理解借用和生命周期。建议从官方Rust Book开始，配合Rustlings练习题巩固。\
             另外可以参加Rust社区，多写项目实践。记住：编译器是你的朋友！",
            1500,
        );
        assert!(result.overall_score >= 60);
        assert!(result.stars >= 2);
        assert!(!result.strengths.is_empty());
    }

    #[test]
    fn test_evaluate_low_quality() {
        let result = XinMichelinService::evaluate_dialogue(
            "我最近压力很大，失眠严重",
            "哦",
            5000,
        );
        assert!(result.overall_score < 55);
        assert!(result.stars <= 2);
        assert!(!result.weaknesses.is_empty());
    }

    #[test]
    fn test_evaluate_empathy() {
        let result = XinMichelinService::evaluate_dialogue(
            "今天很难过，被批评了",
            "我能感受到你的难过，被批评确实让人沮丧。不过别太在意，\
             每个人都会遇到这样的情况。你已经很努力了，这只是一个成长的过程。需要我陪你聊聊吗？",
            2000,
        );
        let empathy_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == EvalDimension::Empathy)
            .unwrap();
        assert!(empathy_dim.score >= 70);
    }

    #[test]
    fn test_evaluate_accuracy() {
        let result = XinMichelinService::evaluate_dialogue(
            "Python的列表和元组有什么区别？",
            "列表是可变的，可以修改元素；元组是不可变的，创建后不能修改。\
             因为元组不可变，所以可以作为字典的键，而列表不能。列表使用方括号[]，元组使用圆括号()。",
            1800,
        );
        let accuracy_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == EvalDimension::Accuracy)
            .unwrap();
        assert!(accuracy_dim.score >= 70);
    }

    #[test]
    fn test_star_rating() {
        assert_eq!(XinMichelinService::star_rating(20), (1, "⭐".to_string()));
        assert_eq!(XinMichelinService::star_rating(50), (2, "⭐⭐".to_string()));
        assert_eq!(XinMichelinService::star_rating(80), (3, "⭐⭐⭐".to_string()));
    }

    #[test]
    fn test_aggregate_stats_empty() {
        let stats = XinMichelinService::aggregate_stats(&[]);
        assert_eq!(stats.total_evals, 0);
        assert_eq!(stats.average_score, 0.0);
    }

    #[test]
    fn test_aggregate_stats() {
        let r1 = XinMichelinService::evaluate_dialogue("如何学习Rust？", "从所有权开始学", 1000);
        let r2 = XinMichelinService::evaluate_dialogue("今天开心", "太好了！继续保持好心情！", 800);
        let stats = XinMichelinService::aggregate_stats(&[r1, r2]);
        assert_eq!(stats.total_evals, 2);
        assert!(stats.average_score > 0.0);
        assert_eq!(stats.dimension_averages.len(), 5);
    }

    #[test]
    fn test_generate_report() {
        let r1 = XinMichelinService::evaluate_dialogue("如何学Rust？", "从所有权开始", 1500);
        let r2 = XinMichelinService::evaluate_dialogue("好累", "辛苦了，休息一下吧", 1200);
        let report = XinMichelinService::generate_report(&[r1, r2]);
        assert_eq!(report.stats.total_evals, 2);
        assert!(!report.overall_assessment.is_empty());
    }

    #[test]
    fn test_filter_recent() {
        let r1 = XinMichelinService::evaluate_dialogue("a", "b", 1000);
        let r2 = XinMichelinService::evaluate_dialogue("c", "d", 1500);
        let r3 = XinMichelinService::evaluate_dialogue("e", "f", 800);
        let filtered = XinMichelinService::filter_recent(&[r1, r2, r3], 2);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_timeliness_fast() {
        let result = XinMichelinService::evaluate_dialogue("hello", "hi", 500);
        let time_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == EvalDimension::Timeliness)
            .unwrap();
        assert!(time_dim.score >= 80);
    }

    #[test]
    fn test_timeliness_slow() {
        let result = XinMichelinService::evaluate_dialogue("hello", "hi", 12000);
        let time_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == EvalDimension::Timeliness)
            .unwrap();
        assert!(time_dim.score <= 30);
    }

    #[test]
    fn test_creativity() {
        let result = XinMichelinService::evaluate_dialogue(
            "想不出好点子",
            "换个角度想象一下，如果你是用户，你最想要什么？\
             比如有个产品就是从一个简单的比喻出发设计出来的。不妨试试画个思维导图，灵感就在其中！",
            2000,
        );
        let creative_dim = result
            .dimensions
            .iter()
            .find(|d| d.dimension == EvalDimension::Creativity)
            .unwrap();
        assert!(creative_dim.score >= 60);
    }
}