use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;

use chrono::{Datelike, Timelike};

use crate::error::app_error::AppError;
use crate::services::xin_context_service::{ChatMessage, ChatRole, ContextWindow};
use crate::services::xin_michelin_service::EvalResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    pub period_type: String,
    pub date: Option<String>,
    pub persona_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationReview {
    pub period_type: String,
    pub period_label: String,
    pub date_from: String,
    pub date_to: String,
    pub conversation_count: usize,
    pub total_messages: usize,
    pub total_tokens: i64,
    pub dominant_topics: Vec<TopicCount>,
    pub top_conversations: Vec<ReviewConversation>,
    pub sentiment_summary: SentimentSummary,
    pub generated_summary: String,
    pub most_active_hours: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCount {
    pub topic: String,
    pub frequency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConversation {
    pub id: String,
    pub title: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentSummary {
    pub positive: usize,
    pub neutral: usize,
    pub negative: usize,
    pub average_empathy_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicTrend {
    pub topic: String,
    pub data_points: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date_label: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicTrendResponse {
    pub period_type: String,
    pub date_from: String,
    pub date_to: String,
    pub trends: Vec<TopicTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthTrajectory {
    pub date_from: String,
    pub date_to: String,
    pub emotion_trend: Vec<TrendPoint>,
    pub empathy_trend: Vec<TrendPoint>,
    pub creativity_trend: Vec<TrendPoint>,
    pub conversation_frequency: Vec<TrendPoint>,
    pub token_usage_trend: Vec<TrendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapData {
    pub cells: Vec<HeatmapCell>,
    pub max_intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub day_of_week: u32,
    pub hour: u32,
    pub count: usize,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct ConversationRow {
    pub id: String,
    pub title: String,
    pub context_json: String,
    pub message_count: i64,
    pub total_tokens: i64,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct XinConversationReviewService;

impl XinConversationReviewService {
    pub async fn generate_review(
        pool: &SqlitePool,
        user_id: i64,
        request: &ReviewRequest,
    ) -> Result<ConversationReview, AppError> {
        let now = chrono::Utc::now();
        let (date_from, date_to, period_label) =
            Self::compute_period_range(&request.period_type, request.date.as_deref(), &now);

        let mut where_clauses = vec![
            "user_id = ?".to_string(),
            "updated_at >= ? AND updated_at <= ?".to_string(),
        ];
        let mut bind_values: Vec<String> = vec![date_from.clone(), date_to.clone()];

        if let Some(ref pid) = request.persona_id {
            where_clauses.push("persona_id = ?".to_string());
            bind_values.push(pid.clone());
        }

        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "SELECT id, title, context_json, message_count, total_tokens, summary, created_at, updated_at
             FROM xin_conversations WHERE {} ORDER BY updated_at DESC",
            where_sql
        );

        let rows: Vec<ConversationRow> = {
            let mut q = sqlx::query_as(&sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q.fetch_all(pool).await.map_err(AppError::Database)?
        };

        let conversation_count = rows.len();
        let total_messages: usize = rows.iter().map(|r| r.message_count as usize).sum();
        let total_tokens: i64 = rows.iter().map(|r| r.total_tokens).sum();

        let mut topic_counts: HashMap<String, usize> = HashMap::new();
        let mut positive = 0usize;
        let mut neutral = 0usize;
        let mut negative = 0usize;
        let mut empathy_scores: Vec<f64> = Vec::new();
        let mut hour_activity: HashMap<usize, usize> = HashMap::new();

        for row in &rows {
            let contexts: Vec<ContextWindow> =
                serde_json::from_str(&row.context_json).unwrap_or_default();
            if let Some(context) = contexts.first() {
                let (user_msgs, assistant_msgs): (Vec<&ChatMessage>, Vec<&ChatMessage>) = context
                    .messages
                    .iter()
                    .filter(|m| m.role != ChatRole::System)
                    .partition(|m| m.role == ChatRole::User);

                for msg in &user_msgs {
                    Self::classify_sentiment(msg.content.as_str(), &mut positive, &mut neutral, &mut negative);
                    for topic in Self::extract_simple_topics(&msg.content) {
                        *topic_counts.entry(topic).or_insert(0) += 1;
                    }
                }
                for msg in &assistant_msgs {
                    Self::classify_sentiment(msg.content.as_str(), &mut positive, &mut neutral, &mut negative);
                    empathy_scores.push(Self::estimate_empathy(&msg.content));
                }
            }

            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&row.created_at) {
                *hour_activity.entry(dt.hour() as usize).or_insert(0) += 1;
            }
        }

        let average_empathy_score = if empathy_scores.is_empty() {
            0.0
        } else {
            empathy_scores.iter().sum::<f64>() / empathy_scores.len() as f64
        };

        let mut dominant_topics: Vec<TopicCount> = topic_counts
            .into_iter()
            .map(|(topic, frequency)| TopicCount { topic, frequency })
            .collect();
        dominant_topics.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        dominant_topics.truncate(10);

        let mut sorted_hours: Vec<usize> = hour_activity.into_iter()
            .map(|(h, _)| h)
            .collect();
        sorted_hours.sort();

        let top_conversations: Vec<ReviewConversation> = rows
            .iter()
            .take(5)
            .map(|r| ReviewConversation {
                id: r.id.clone(),
                title: r.title.clone(),
                message_count: r.message_count,
                total_tokens: r.total_tokens,
                summary: r.summary.clone(),
                created_at: r.created_at.clone(),
            })
            .collect();

        let sentiment_summary = SentimentSummary {
            positive,
            neutral,
            negative,
            average_empathy_score,
        };

        let generated_summary = Self::build_summary_text(
            &request.period_type,
            &period_label,
            conversation_count,
            total_messages,
            total_tokens,
            &dominant_topics,
            &sentiment_summary,
        );

        Ok(ConversationReview {
            period_type: request.period_type.clone(),
            period_label,
            date_from,
            date_to,
            conversation_count,
            total_messages,
            total_tokens,
            dominant_topics,
            top_conversations,
            sentiment_summary,
            generated_summary,
            most_active_hours: sorted_hours,
        })
    }

    pub async fn get_topic_trends(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: Option<String>,
        period_type: &str,
        buckets: usize,
    ) -> Result<TopicTrendResponse, AppError> {
        let now = chrono::Utc::now();
        let (date_from, date_to, _) =
            Self::compute_period_range(period_type, None, &now);

        let mut where_clauses = vec![
            "user_id = ?".to_string(),
            "updated_at >= ? AND updated_at <= ?".to_string(),
        ];
        let mut bind_values: Vec<String> = vec![date_from.clone(), date_to.clone()];

        if let Some(ref pid) = persona_id {
            where_clauses.push("persona_id = ?".to_string());
            bind_values.push(pid.clone());
        }

        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "SELECT context_json, updated_at FROM xin_conversations WHERE {} ORDER BY updated_at ASC",
            where_sql
        );

        #[derive(Debug, sqlx::FromRow)]
        struct RawRow {
            context_json: String,
            updated_at: String,
        }

        let rows: Vec<RawRow> = {
            let mut q = sqlx::query_as(&sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q.fetch_all(pool).await.map_err(AppError::Database)?
        };

        let from_dt = chrono::DateTime::parse_from_rfc3339(&date_from)
            .map(|d| d.to_utc())
            .unwrap_or(now);
        let to_dt = chrono::DateTime::parse_from_rfc3339(&date_to)
            .map(|d| d.to_utc())
            .unwrap_or(now);
        let total_secs = (to_dt - from_dt).num_seconds().max(1) as f64;
        let bucket_secs = total_secs / buckets as f64;

        let mut topic_buckets: HashMap<String, Vec<TrendPoint>> = HashMap::new();
        for i in 0..buckets {
            let bucket_from = from_dt + chrono::Duration::seconds((bucket_secs * i as f64) as i64);
            let bucket_to = from_dt + chrono::Duration::seconds((bucket_secs * (i + 1) as f64) as i64);
            let label = bucket_from.format("%m-%d").to_string();
            let _bucket_end_label = bucket_to.format("%m-%d").to_string();
            let bucket_key = label;

            let bucket_rows: Vec<&RawRow> = rows
                .iter()
                .filter(|r| {
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&r.updated_at) {
                        ts >= bucket_from && ts < bucket_to
                    } else {
                        false
                    }
                })
                .collect();

            for row in &bucket_rows {
                let contexts: Vec<ContextWindow> =
                    serde_json::from_str(&row.context_json).unwrap_or_default();
                if let Some(context) = contexts.first() {
                    for msg in &context.messages {
                        for topic in Self::extract_simple_topics(&msg.content) {
                            let entry = topic_buckets
                                .entry(topic.clone())
                                .or_insert_with(|| {
                                    (0..buckets)
                                        .map(|_| TrendPoint {
                                            date_label: String::new(),
                                            value: 0.0,
                                        })
                                        .collect()
                                });
                            entry[i].date_label = bucket_key.clone();
                            entry[i].value += 1.0;
                        }
                    }
                }
            }
        }

        let trends: Vec<TopicTrend> = topic_buckets
            .into_iter()
            .filter(|(_, points)| points.iter().any(|p| p.value > 0.0))
            .map(|(topic, points)| TopicTrend {
                topic,
                data_points: points,
            })
            .collect();

        Ok(TopicTrendResponse {
            period_type: period_type.to_string(),
            date_from,
            date_to,
            trends,
        })
    }

    pub async fn get_growth_trajectory(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: Option<String>,
        period_type: &str,
        buckets: usize,
        eval_results: Option<Vec<EvalResult>>,
    ) -> Result<GrowthTrajectory, AppError> {
        let now = chrono::Utc::now();
        let (date_from, date_to, _) =
            Self::compute_period_range(period_type, None, &now);

        let mut where_clauses = vec![
            "user_id = ?".to_string(),
            "updated_at >= ? AND updated_at <= ?".to_string(),
        ];
        let mut bind_values: Vec<String> = vec![date_from.clone(), date_to.clone()];

        if let Some(ref pid) = persona_id {
            where_clauses.push("persona_id = ?".to_string());
            bind_values.push(pid.clone());
        }

        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "SELECT context_json, updated_at, total_tokens FROM xin_conversations WHERE {} ORDER BY updated_at ASC",
            where_sql
        );

        #[derive(Debug, sqlx::FromRow)]
        struct TrajRow {
            context_json: String,
            updated_at: String,
            total_tokens: i64,
        }

        let rows: Vec<TrajRow> = {
            let mut q = sqlx::query_as(&sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q.fetch_all(pool).await.map_err(AppError::Database)?
        };

        let from_dt = chrono::DateTime::parse_from_rfc3339(&date_from)
            .map(|d| d.to_utc())
            .unwrap_or(now);
        let to_dt = chrono::DateTime::parse_from_rfc3339(&date_to)
            .map(|d| d.to_utc())
            .unwrap_or(now);
        let total_secs = (to_dt - from_dt).num_seconds().max(1) as f64;
        let bucket_secs = total_secs / buckets as f64;

        let make_empty_points = |buckets: usize| -> Vec<TrendPoint> {
            (0..buckets).map(|_| TrendPoint { date_label: String::new(), value: 0.0 }).collect()
        };

        let mut emotion_points = make_empty_points(buckets);
        let mut empathy_points = make_empty_points(buckets);
        let mut creativity_points = make_empty_points(buckets);
        let mut freq_points = make_empty_points(buckets);
        let mut token_points = make_empty_points(buckets);

        for i in 0..buckets {
            let bucket_from = from_dt + chrono::Duration::seconds((bucket_secs * i as f64) as i64);
            let bucket_to = from_dt + chrono::Duration::seconds((bucket_secs * (i + 1) as f64) as i64);
            let label = bucket_from.format("%m-%d").to_string();

            emotion_points[i].date_label = label.clone();
            empathy_points[i].date_label = label.clone();
            creativity_points[i].date_label = label.clone();
            freq_points[i].date_label = label.clone();
            token_points[i].date_label = label.clone();

            let bucket_rows: Vec<&TrajRow> = rows.iter().filter(|r| {
                chrono::DateTime::parse_from_rfc3339(&r.updated_at)
                    .map(|ts| ts >= bucket_from && ts < bucket_to)
                    .unwrap_or(false)
            }).collect();

            freq_points[i].value = bucket_rows.len() as f64;

            let mut total_tokens: i64 = 0;
            let mut bucket_emotion: f64 = 0.0;
            let mut bucket_emotion_count: usize = 0;

            for row in &bucket_rows {
                total_tokens += row.total_tokens;
                let contexts: Vec<ContextWindow> =
                    serde_json::from_str(&row.context_json).unwrap_or_default();
                if let Some(context) = contexts.first() {
                    for msg in &context.messages {
                        if msg.role == ChatRole::Assistant {
                            bucket_emotion += Self::estimate_empathy(&msg.content);
                            bucket_emotion_count += 1;
                        }
                    }
                }
            }

            token_points[i].value = total_tokens as f64;
            if bucket_emotion_count > 0 {
                emotion_points[i].value = bucket_emotion / bucket_emotion_count as f64;
            }
        }

        if let Some(ref evals) = eval_results {
            for eval in evals {
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&eval.evaluated_at).map(|d| d.to_utc()) {
                    for i in 0..buckets {
                        let bucket_from = from_dt + chrono::Duration::seconds((bucket_secs * i as f64) as i64);
                        let bucket_to = from_dt + chrono::Duration::seconds((bucket_secs * (i + 1) as f64) as i64);
                        if ts >= bucket_from && ts < bucket_to {
                            for dim in &eval.dimensions {
                                match dim.dimension {
                                    crate::services::xin_michelin_service::EvalDimension::Empathy => {
                                        empathy_points[i].value += dim.score as f64;
                                    }
                                    crate::services::xin_michelin_service::EvalDimension::Creativity => {
                                        creativity_points[i].value += dim.score as f64;
                                    }
                                    _ => {}
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        Ok(GrowthTrajectory {
            date_from,
            date_to,
            emotion_trend: emotion_points,
            empathy_trend: empathy_points,
            creativity_trend: creativity_points,
            conversation_frequency: freq_points,
            token_usage_trend: token_points,
        })
    }

    pub async fn get_heatmap(
        pool: &SqlitePool,
        user_id: i64,
        persona_id: Option<String>,
        period_type: &str,
    ) -> Result<HeatmapData, AppError> {
        let now = chrono::Utc::now();
        let (date_from, date_to, _) =
            Self::compute_period_range(period_type, None, &now);

        let mut where_clauses = vec![
            "user_id = ?".to_string(),
            "created_at >= ? AND created_at <= ?".to_string(),
        ];
        let mut bind_values: Vec<String> = vec![date_from.clone(), date_to.clone()];

        if let Some(ref pid) = persona_id {
            where_clauses.push("persona_id = ?".to_string());
            bind_values.push(pid.clone());
        }

        let where_sql = where_clauses.join(" AND ");
        let sql = format!(
            "SELECT created_at FROM xin_conversations WHERE {}",
            where_sql
        );

        #[derive(Debug, sqlx::FromRow)]
        struct TimeRow {
            created_at: String,
        }

        let rows: Vec<TimeRow> = {
            let mut q = sqlx::query_as(&sql);
            q = q.bind(user_id);
            for v in &bind_values {
                q = q.bind(v);
            }
            q.fetch_all(pool).await.map_err(AppError::Database)?
        };

        let mut grid: HashMap<(u32, u32), usize> = HashMap::new();
        for row in &rows {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&row.created_at) {
                let dow = dt.weekday().num_days_from_monday();
                let hour = dt.hour();
                *grid.entry((dow, hour)).or_insert(0) += 1;
            }
        }

        let max_count = grid.values().copied().max().unwrap_or(1) as f64;

        let mut cells: Vec<HeatmapCell> = grid
            .into_iter()
            .map(|((day_of_week, hour), count)| HeatmapCell {
                day_of_week,
                hour,
                count,
                intensity: if max_count > 0.0 {
                    count as f64 / max_count
                } else {
                    0.0
                },
            })
            .collect();
        cells.sort_by(|a, b| {
            a.day_of_week
                .cmp(&b.day_of_week)
                .then(a.hour.cmp(&b.hour))
        });

        Ok(HeatmapData {
            cells,
            max_intensity: max_count,
        })
    }

    fn compute_period_range(
        period_type: &str,
        date: Option<&str>,
        now: &chrono::DateTime<chrono::Utc>,
    ) -> (String, String, String) {
        let today = now.format("%Y-%m-%d").to_string();

        match period_type {
            "weekly" => {
                let days_from_monday = now.weekday().num_days_from_monday() as i64;
                let monday = *now - chrono::Duration::days(days_from_monday);
                let sunday = monday + chrono::Duration::days(6);
                let from = format!("{}T00:00:00Z", monday.format("%Y-%m-%d"));
                let to = format!("{}T23:59:59Z", sunday.format("%Y-%m-%d"));
                let label = format!("{}-{}", monday.format("%Y-%m-%d"), sunday.format("%Y-%m-%d"));
                (from, to, label)
            }
            "monthly" => {
                let month_start = now.format("%Y-%m-01").to_string();
                let from = format!("{}T00:00:00Z", month_start);
                let to = format!("{}T23:59:59Z", today);
                let label = now.format("%Y年%m月").to_string();
                (from, to, label)
            }
            _ => {
                let date_str = date.unwrap_or(&today);
                let from = format!("{}T00:00:00Z", date_str);
                let to = format!("{}T23:59:59Z", date_str);
                let label = date_str.to_string();
                (from, to, label)
            }
        }
    }

    fn classify_sentiment(text: &str, positive: &mut usize, neutral: &mut usize, negative: &mut usize) {
        let lower = text.to_lowercase();
        let pos_words = ["好", "棒", "赞", "开心", "喜欢", "谢谢", "太棒了", "很好", "不错", "厉害",
            "优秀", "满意", "愉快", "完美", "精彩", "哈哈", "😊", "👍", "感谢", "幸福"];
        let neg_words = ["不好", "糟糕", "难过", "生气", "讨厌", "烦", "累", "困", "难受", "失望",
            "焦虑", "担心", "害怕", "痛苦", "不行", "错了", "失败", "哭", "😢", "😡"];

        let pos_count = pos_words.iter().filter(|w| lower.contains(*w)).count();
        let neg_count = neg_words.iter().filter(|w| lower.contains(*w)).count();

        if pos_count > neg_count + 1 {
            *positive += 1;
        } else if neg_count > pos_count + 1 {
            *negative += 1;
        } else {
            *neutral += 1;
        }
    }

    fn estimate_empathy(text: &str) -> f64 {
        let lower = text.to_lowercase();
        let empathy_words = ["理解", "明白", "感受", "心情", "情绪", "陪伴", "倾听", "关心",
            "在乎", "温暖", "支持", "拥抱", "别担心", "有我在", "辛苦了", "没关系",
            "我懂", "你很重要", "你不是一个人", "慢慢来"];
        let cold_words = ["你自己", "随你", "不知道", "没办法", "与我无关", "爱莫能助"];

        let emp_count = empathy_words.iter().filter(|w| lower.contains(*w)).count();
        let cold_count = cold_words.iter().filter(|w| lower.contains(*w)).count();

        let base = 0.5;
        let bonus = (emp_count as f64 * 0.15).min(0.45);
        let penalty = (cold_count as f64 * 0.2).min(0.4);
        (base + bonus - penalty).clamp(0.1, 0.95)
    }

    fn extract_simple_topics(text: &str) -> Vec<String> {
        let topic_keywords: [(&str, &str); 20] = [
            ("编程", "编程开发"), ("代码", "编程开发"), ("rust", "编程开发"),
            ("python", "编程开发"), ("ai", "人工智能"), ("模型", "人工智能"),
            ("学习", "学习成长"), ("学", "学习成长"), ("考试", "学习成长"),
            ("工作", "工作职场"), ("项目", "工作职场"), ("bug", "编程开发"),
            ("音乐", "文化艺术"), ("电影", "文化艺术"), ("读书", "学习成长"),
            ("旅行", "生活方式"), ("美食", "生活方式"), ("健身", "健康运动"),
            ("心情", "情绪心理"), ("游戏", "休闲娱乐"),
        ];

        let lower = text.to_lowercase();
        let mut topics = Vec::new();
        for (keyword, topic) in &topic_keywords {
            if lower.contains(keyword) && !topics.contains(&topic.to_string()) {
                topics.push(topic.to_string());
            }
        }
        if topics.is_empty() {
            topics.push("日常闲聊".to_string());
        }
        topics
    }

    fn build_summary_text(
        period_type: &str,
        period_label: &str,
        conv_count: usize,
        total_msgs: usize,
        total_tokens: i64,
        topics: &[TopicCount],
        sentiment: &SentimentSummary,
    ) -> String {
        let period_name = match period_type {
            "weekly" => "本周",
            "monthly" => "本月",
            _ => "今日",
        };

        let top_topics: Vec<String> = topics.iter().take(3)
            .map(|t| t.topic.clone())
            .collect();
        let topic_text = if top_topics.is_empty() {
            "多领域话题".to_string()
        } else {
            top_topics.join("、")
        };

        let mood = if sentiment.positive > sentiment.negative {
            "积极向上"
        } else if sentiment.negative > sentiment.positive {
            "略显低沉"
        } else {
            "平稳中性"
        };

        format!(
            "{}（{}），共进行了 {} 次对话，累计 {} 条消息（约 {} tokens）。主要话题集中在{}。整体情绪{}{}。",
            period_name,
            period_label,
            conv_count,
            total_msgs,
            total_tokens,
            topic_text,
            mood,
            if sentiment.average_empathy_score > 0.6 {
                format!("，共情度较高（{:.0}%）", sentiment.average_empathy_score * 100.0)
            } else {
                String::new()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_sentiment_positive() {
        let mut pos = 0;
        let mut neu = 0;
        let mut neg = 0;
        XinConversationReviewService::classify_sentiment("今天太棒了！真的很开心，谢谢！", &mut pos, &mut neu, &mut neg);
        assert!(pos > 0);
    }

    #[test]
    fn test_classify_sentiment_negative() {
        let mut pos = 0;
        let mut neu = 0;
        let mut neg = 0;
        XinConversationReviewService::classify_sentiment("今天好难过，感觉很糟糕，太累了", &mut pos, &mut neu, &mut neg);
        assert_eq!(neg, 1);
    }

    #[test]
    fn test_classify_sentiment_neutral() {
        let mut pos = 0;
        let mut neu = 0;
        let mut neg = 0;
        XinConversationReviewService::classify_sentiment("今天天气不错", &mut pos, &mut neu, &mut neg);
        assert_eq!(neu, 1);
    }

    #[test]
    fn test_estimate_empathy_high() {
        let score = XinConversationReviewService::estimate_empathy("我理解你的感受，有我在呢，别担心，慢慢来");
        assert!(score > 0.7);
    }

    #[test]
    fn test_estimate_empathy_low() {
        let score = XinConversationReviewService::estimate_empathy("你自己看着办吧，我不知道，没办法");
        assert!(score < 0.5);
    }

    #[test]
    fn test_extract_simple_topics() {
        let topics = XinConversationReviewService::extract_simple_topics("今天学了一下Rust编程，感觉AI很有意思");
        assert!(topics.contains(&"编程开发".to_string()) || topics.contains(&"人工智能".to_string()));
    }

    #[test]
    fn test_compute_period_range_daily() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-27T15:30:00Z")
            .unwrap()
            .to_utc();
        let (from, to, label) = XinConversationReviewService::compute_period_range("daily", None, &now);
        assert_eq!(label, "2026-05-27");
        assert!(from.contains("T00:00:00"));
        assert!(to.contains("T23:59:59"));
    }

    #[test]
    fn test_compute_period_range_weekly() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-27T15:30:00Z")
            .unwrap()
            .to_utc();
        let (from, to, label) = XinConversationReviewService::compute_period_range("weekly", None, &now);
        assert!(from.contains("T00:00:00"));
        assert!(to.contains("T23:59:59"));
        assert!(!label.is_empty());
    }

    #[test]
    fn test_compute_period_range_monthly() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-05-27T15:30:00Z")
            .unwrap()
            .to_utc();
        let (_from, _to, label) = XinConversationReviewService::compute_period_range("monthly", None, &now);
        assert!(label.contains("2026"));
        assert!(label.contains("月"));
    }

    #[test]
    fn test_build_summary_text() {
        let topics = vec![
            TopicCount { topic: "编程开发".into(), frequency: 5 },
            TopicCount { topic: "人工智能".into(), frequency: 3 },
        ];
        let sentiment = SentimentSummary {
            positive: 8,
            neutral: 2,
            negative: 1,
            average_empathy_score: 0.75,
        };
        let text = XinConversationReviewService::build_summary_text(
            "daily", "2026-05-27", 3, 15, 5000, &topics, &sentiment,
        );
        assert!(text.contains("编程开发"));
        assert!(text.contains("人工智能"));
        assert!(text.contains("2026-05-27"));
        assert!(text.contains("积极向上"));
    }
}