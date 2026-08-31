use std::time::Duration;

use quick_xml::de::from_str;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::db::repositories::news_repo;
use crate::error::app_error::AppError;
use crate::models::news::{NewsCache, NewsSource};
use crate::services::intelligence_v4_service;
use crate::services::news_source_service;
use crate::services::news_cache_service::{self, NewsCacheItem};

#[derive(Debug, Serialize, Clone)]
pub struct FetchNewsResult {
    pub deleted: u64,
    pub inserted: usize,
    pub net_change: i64,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
struct RssFeed {
    channel: RssChannel,
}

#[derive(Debug, Deserialize)]
struct RssChannel {
    #[serde(default)]
    item: Vec<RssItem>,
}

#[derive(Debug, Deserialize)]
struct RssItem {
    title: String,
    link: Option<String>,
    description: Option<String>,
    #[serde(rename = "pubDate", default)]
    pub_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AtomFeed {
    #[serde(default)]
    entry: Vec<AtomEntry>,
}

#[derive(Debug, Deserialize)]
struct AtomEntry {
    title: String,
    #[serde(default)]
    link: Vec<AtomLink>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    published: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AtomLink {
    #[serde(default)]
    href: Option<String>,
}

fn score_article(title: &str, summary: Option<&str>) -> i32 {
    let text = if let Some(s) = summary {
        format!("{} {}", title.to_lowercase(), s.to_lowercase())
    } else {
        title.to_lowercase()
    };

    let mut score: i32 = 0;

    // ===== 方向1: 技术创新 (权重最高) =====
    let tech_innovation: &[&str] = &[
        // 突破性技术
        "突破", "首次", "重大", "里程碑", "革命性", "颠覆", "全新",
        "新架构", "下一代", "原型", "proof of concept",
        // AI/大模型前沿
        "大模型", "llm", "transformer", "gpt-5", "claude", "deepseek",
        "agi", "通用人工智能", "多模态", "推理能力", "思维链",
        "chain of thought", "rlhf", "fine-tuning", "微调", "预训练",
        "扩散模型", "生成式ai", "generative ai", "文生图", "文生视频",
        // 底层技术
        "rust", "golang", "typescript", "webassembly", "wasm",
        "编译器", "操作系统", "数据库内核", "分布式系统",
        "量子计算", "神经网络芯片", "边缘计算", "存算一体",
        // 隐私与前沿
        "隐私计算", "联邦学习", "同态加密", "零知识证明", "zkp",
        "差分隐私", "安全多方计算", "tee", "可信执行环境",
        // 开发范式
        "devops 2.0", "平台工程", "ai原生", "ai native",
        "云原生2.0", "serverless 2.0", "低代码", "nocode",
    ];
    for kw in tech_innovation {
        if text.contains(kw) {
            score += 5;
        }
    }

    // ===== 方向2: 漏洞发现与解决 (权重高) =====
    let vulnerability: &[&str] = &[
        // 漏洞类型
        "零日漏洞", "cve-", "0day", "nday", "1day",
        "远程代码执行", "rce", "本地提权", "lpe", "权限提升",
        "缓冲区溢出", "sql注入", "xss", "csrf", "ssrf",
        "反序列化", "命令注入", "路径遍历", "信息泄露",
        "内存破坏", "use-after-free", "double free", "race condition",
        // 漏洞发现
        "漏洞发现", "漏洞挖掘", "漏洞复现", "漏洞分析", "漏洞报告",
        "fuzzing", "fuzz", "符号执行", "污点分析", "静态分析",
        "代码审计", "逆向工程", "逆向分析", "二进制分析",
        // 漏洞修复
        "漏洞修复", "补丁", "patch", "安全更新", "版本升级",
        "漏洞缓解", "漏洞利用", "exploit", "poc验证", "exp",
        "cvss", "漏洞评分", "影响范围", "攻击面", "漏洞披露",
        // 特定领域
        "供应链攻击", "供应链安全", "软件供应链", "sbom",
        "容器逃逸", "沙箱逃逸", "内核漏洞", "驱动漏洞",
        "浏览器漏洞", "协议漏洞", "硬件漏洞", "spectre", "meltdown",
    ];
    for kw in vulnerability {
        if text.contains(kw) {
            score += 5;
        }
    }

    // ===== 方向3: 攻防方案 (权重高) =====
    let attack_defense: &[&str] = &[
        // 攻击方案
        "apt攻击", "apt组织", "威胁组织", "黑客组织",
        "勒索软件", "ransomware", "勒索攻击", "数据泄露",
        "网络攻击", "ddos", "钓鱼攻击", "社会工程学",
        "横向移动", "持久化", "权限维持", "c2", "c&c",
        "红蓝对抗", "红队", "蓝队", "渗透测试", "攻防演练",
        "内网渗透", "域渗透", "云渗透", "无线渗透",
        // 防御方案
        "威胁情报", "威胁狩猎", "threat hunting", "威胁检测",
        "零信任", "zero trust", "身份认证", "访问控制", "mfa",
        "edr", "xdr", "ndr", "soc", "siem", "soar",
        "waf", "ids", "ips", "防火墙", "蜜罐", "honeypot",
        "终端安全", "云安全", "容器安全", "k8s安全",
        "数据安全", "dlp", "加密", "密钥管理", "hsm",
        // 安全框架
        "mitre", "att&ck", "kill chain", "diamond model",
        "nist", "iso27001", "等保", "网络安全法", "数据安全法",
        // 安全方案
        "防御方案", "解决方案", "安全架构", "安全策略",
        "纵深防御", "主动防御", "欺骗防御", "自适应安全",
    ];
    for kw in attack_defense {
        if text.contains(kw) {
            score += 5;
        }
    }

    // ===== 方向4: 工具应用 (权重中高) =====
    let tool_application: &[&str] = &[
        // 安全工具
        "metasploit", "burpsuite", "nmap", "wireshark", "cobalt strike",
        "bloodhound", "mimikatz", "hashcat", "john the ripper",
        "sqlmap", "hydra", "aircrack", "bettercap", "responder",
        "ghidra", "ida pro", "x64dbg", "ollydbg", "radare2",
        "yara", "sigma", "suricata", "snort", "zeek",
        // 开发工具
        "开源工具", "工具发布", "新版本", "v2.0", "cli工具",
        "vscode", "neovim", "helix", "zed", "tmux",
        "docker", "kubernetes", "terraform", "ansible", "helm",
        "prometheus", "grafana", "elk", "opentelemetry",
        "github actions", "gitlab ci", "jenkins", "argo",
        // 开发框架
        "react", "vue", "svelte", "solidjs", "next.js", "nuxt",
        "spring", "django", "fastapi", "gin", "actix",
        "tensorflow", "pytorch", "langchain", "llamaindex",
        // 工具链
        "开发工具链", "效率工具", "自动化工具", "devtools",
        "调试工具", "性能分析", "profiler", "benchmark",
        "包管理器", "构建工具", "cargo", "npm", "pip", "go mod",
    ];
    for kw in tool_application {
        if text.contains(kw) {
            score += 4;
        }
    }

    // ===== 通用技术深度 (权重中) =====
    let tech_deep_dive: &[&str] = &[
        "源码分析", "架构设计", "性能优化", "最佳实践",
        "微服务", "serverless", "事件驱动", "cqrs",
        "service mesh", "api gateway", "消息队列",
        "算法", "数据结构", "系统设计", "设计模式",
        "api设计", "graphql", "grpc", "websocket", "restful",
        "数据库", "缓存", "redis", "postgresql", "mysql",
        "分布式", "一致性", "共识算法", "raft", "paxos",
    ];
    for kw in tech_deep_dive {
        if text.contains(kw) {
            score += 3;
        }
    }

    // ===== 热点/趋势 (权重中低) =====
    let hotspot: &[&str] = &[
        "热点", "爆火", "刷屏", "热议", "头条",
        "ai安全", "深度伪造", "deepfake", "prompt注入",
        "模型开源", "权重发布", "基准测试", "benchmark",
        "技术争议", "开源之争", "许可证变更",
        "黑客大会", "blackhat", "defcon", "rsa",
        "安全公告", "应急响应", "紧急更新",
    ];
    for kw in hotspot {
        if text.contains(kw) {
            score += 3;
        }
    }

    // ===== 通用价值 (权重低) =====
    let medium_value: &[&str] = &[
        "ai", "人工智能", "机器学习", "深度学习",
        "编程语言", "框架", "库", "sdk",
        "云计算", "云原生", "devops", "ci/cd",
        "安全", "加密", "协议",
        "自动化", "测试", "监控",
        "开源", "github", "gitlab",
        "工具", "平台", "应用",
    ];
    for kw in medium_value {
        if text.contains(kw) {
            score += 2;
        }
    }

    // ===== 负向过滤 (惩罚) =====
    let negative: &[&str] = &[
        "融资", "上市", "股价", "裁员", "八卦",
        "收购", "财报", "营收", "利润",
        "倒闭", "破产", "跑路",
        "明星", "综艺", "娱乐", "电影", "电视剧",
        "炒币", "币价", "暴涨", "暴跌",
        "广告", "促销", "优惠", "打折",
        "水军", "营销号", "标题党", "震惊",
        "直播", "带货", "网红", "up主",
        "体育", "足球", "篮球", "电竞",
        "天气", "疫情", "地震", "台风",
    ];
    for kw in negative {
        if text.contains(kw) {
            score -= 10;
        }
    }

    score
}

pub async fn get_news(pool: &SqlitePool, user_id: i64) -> Result<Vec<NewsCache>, AppError> {
    news_repo::get_news(pool, user_id).await
}

pub async fn get_news_by_category(
    pool: &SqlitePool,
    user_id: i64,
    category: &str,
) -> Result<Vec<NewsCache>, AppError> {
    news_repo::get_news_by_category(pool, user_id, category).await
}

pub async fn add_news(
    pool: &SqlitePool,
    user_id: i64,
    title: &str,
    url: Option<&str>,
    source: Option<&str>,
    summary: Option<&str>,
) -> Result<NewsCache, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    news_repo::add_news(pool, user_id, title, url, source, summary, now).await
}

pub async fn mark_read(pool: &SqlitePool, user_id: i64, id: i64) -> Result<(), AppError> {
    news_repo::mark_news_read(pool, user_id, id).await
}

pub async fn clear_old(pool: &SqlitePool, days: i64) -> Result<u64, AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    let before = now - days * 24 * 3600 * 1000;
    news_repo::clear_old_news(pool, before).await
}

pub async fn fetch_and_cache_news(pool: &SqlitePool, user_id: i64) -> Result<FetchNewsResult, AppError> {
    let start_time = std::time::Instant::now();
    tracing::info!("🚀 [新闻拉取] 开始手动刷新...");

    let now_ts = chrono::Utc::now().timestamp_millis();
    let cutoff_24h = now_ts - 24 * 3600 * 1000;

    tracing::info!("🗑️ [1/5] 清理过期旧新闻（>24h且未收藏）...");
    tracing::info!(
        now_ts = now_ts,
        cutoff = cutoff_24h,
        "📊 当前时间戳: {}, 24h前截止: {}",
        now_ts,
        cutoff_24h
    );

    let expired_deleted = news_repo::delete_expired_non_favorite(pool, cutoff_24h).await.unwrap_or(0);

    let total_count = news_repo::count_news(pool, user_id).await.unwrap_or(0);
    let favorite_count = news_repo::count_favorite_news(pool, user_id).await.unwrap_or(0);

    tracing::info!(
        deleted = expired_deleted,
        total = total_count,
        favorite = favorite_count,
        "✅ [1/5] 清理完成 - 删除{}条 | 剩余{}条 (其中收藏{}条)",
        expired_deleted,
        total_count,
        favorite_count
    );

    if total_count > 0 && expired_deleted == 0 {
        tracing::info!("ℹ️ [1/5] 无过期新闻需要删除");
    }

    tracing::info!("🧹 [1/5] 清理重复新闻...");
    let dup_removed = news_repo::remove_duplicate_news(pool).await.unwrap_or(0);
    if dup_removed > 0 {
        tracing::info!(
            count = dup_removed,
            "✅ [1/5] 已清理{}条重复新闻",
            dup_removed
        );
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("NexTerm/1.0 News Reader")
        .build()
        .map_err(|e| AppError::Internal(format!("创建HTTP客户端失败: {}", e)))?;

    tracing::info!("📡 [2/5] 正在获取新闻源列表...");
    let sources = news_source_service::get_news_sources_for_fetch(pool, user_id).await;
    tracing::info!(
        source_count = sources.len(),
        "✅ [2/5] 获取到 {} 个新闻源",
        sources.len()
    );

    if sources.is_empty() {
        tracing::warn!("⚠️ [新闻拉取] 未配置任何新闻源，请先添加");
        return Ok(FetchNewsResult { deleted: 0, inserted: 0, net_change: 0, total: 0 });
    }

    let mut all_items: Vec<(i32, ParsedItem, &NewsSource)> = Vec::new();
    let mut success_count = 0usize;
    let mut fail_count = 0usize;

    tracing::info!("🌐 [3/5] 开始拉取各新闻源内容 (预计10-30秒)...");

    for (idx, source) in sources.iter().enumerate() {
        let source_start = std::time::Instant::now();
        tracing::info!(
            current = idx + 1,
            total = sources.len(),
            source = %source.name,
            "⏳ 正在拉取 [{}/{}] {}...",
            idx + 1,
            sources.len(),
            source.name
        );

        match fetch_single_source(&client, source).await {
            Ok(items) => {
                let count = items.len();
                let elapsed = source_start.elapsed().as_secs_f64();
                for item in items {
                    let score = score_article(&item.title, item.description.as_deref());
                    all_items.push((score, item, source));
                }
                success_count += 1;
                tracing::info!(
                    source = %source.name,
                    raw_count = count,
                    total_items = all_items.len(),
                    elapsed_sec = elapsed,
                    "✅ [{}/{}] {} 完成 - 获取{}篇文章 ({:.1}s)",
                    idx + 1,
                    sources.len(),
                    source.name,
                    count,
                    elapsed
                );
            }
            Err(e) => {
                fail_count += 1;
                tracing::warn!(
                    source = %source.name,
                    error = %e,
                    "❌ [{}/{}] {} 失败 - {}",
                    idx + 1,
                    sources.len(),
                    source.name,
                    e
                );
            }
        }
    }

    tracing::info!(
        success = success_count,
        failed = fail_count,
        total_raw = all_items.len(),
        elapsed_sec = start_time.elapsed().as_secs_f64(),
        "📊 [3/5] 拉取完成 - 成功{}/失败{} - 共{}篇原始文章 ({:.1}s)",
        success_count,
        fail_count,
        all_items.len(),
        start_time.elapsed().as_secs_f64()
    );

    if all_items.is_empty() {
        tracing::warn!("⚠️ [新闻拉取] 所有新闻源均无内容或全部失败");
        return Ok(FetchNewsResult { deleted: expired_deleted, inserted: 0, net_change: -(expired_deleted as i64), total: total_count });
    }

    tracing::info!("🔍 [4/5] 正在评分和排序 {} 篇文章...", all_items.len());
    let scoring_start = std::time::Instant::now();
    all_items.sort_by(|a, b| b.0.cmp(&a.0));
    tracing::info!(
        elapsed_ms = scoring_start.elapsed().as_millis(),
        "✅ [4/5] 评分排序完成 ({:.0}ms)",
        scoring_start.elapsed().as_millis()
    );

    let now = chrono::Utc::now().timestamp_millis();
    let cutoff_24h = now - 24 * 3600 * 1000;
    let mut inserted = 0usize;
    let mut filtered_by_score = 0usize;
    let mut filtered_by_time = 0usize;
    let mut filtered_by_duplicate = 0usize;

    tracing::info!("🎯 [5/5] 开始筛选入库 (阈值≥5分 | 24h内 | 去重 | 每分类≤8条 | 总计≤32条 | 宁缺毋滥)...");
    let filter_start = std::time::Instant::now();

    // 每个分类最多 N 条，总计最多 M 条（all_items 已按 score 降序排列，取前N即为质量最优）
    const CATEGORY_MAX: usize = 8;   // 每分类上限
    const TOTAL_MAX: usize = 32;     // 总量上限
    use std::collections::HashMap;
    let mut category_counts: HashMap<&str, usize> = HashMap::new();

    for (score, item, source) in &all_items {
        // 总量上限检查
        if inserted >= TOTAL_MAX {
            tracing::info!(total = inserted, "📦 已达总量上限 {}，停止入库", TOTAL_MAX);
            break;
        }

        // 分类上限检查
        let cat_count = category_counts.get(source.category.as_str()).copied().unwrap_or(0);
        if cat_count >= CATEGORY_MAX {
            continue;
        }

        if *score < 5 {
            filtered_by_score += 1;
            continue;
        }

        if item.title.trim().is_empty() {
            continue;
        }

        let url = item.link.as_deref().unwrap_or("");

        if !url.is_empty() && news_repo::exists_by_url(pool, user_id, url).await.unwrap_or(false) {
            filtered_by_duplicate += 1;
            continue;
        }

        if news_repo::exists_by_title_and_source(pool, user_id, &item.title, &source.name).await.unwrap_or(false) {
            filtered_by_duplicate += 1;
            continue;
        }

        let published_at = parse_date(&item.pub_date);

        if let Some(ref pub_str) = published_at {
            if let Ok(ts) = chrono::NaiveDateTime::parse_from_str(pub_str, "%Y-%m-%d %H:%M:%S")
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(pub_str, "%Y-%m-%dT%H:%M:%S")
                })
            {
                let pub_ts = ts.and_utc().timestamp_millis();
                if pub_ts < cutoff_24h {
                    filtered_by_time += 1;
                    continue;
                }
            }
        }

        let summary = item
            .description
            .as_deref()
            .map(|d| strip_html(d))
            .filter(|s| !s.is_empty());

        match news_repo::add_news_full(
            pool,
            user_id,
            &item.title,
            if url.is_empty() { None } else { Some(url) },
            Some(&source.name),
            summary.as_deref(),
            summary.as_deref(),
            Some(&source.category),
            published_at.as_deref(),
            now,
        )
        .await
        {
            Ok(_) => {
                inserted += 1;
                *category_counts.entry(source.category.as_str()).or_insert(0) += 1;
                let title_short: String = item.title.chars().take(25).collect();
                tracing::debug!(
                    title = %title_short,
                    score = score,
                    source = %source.name,
                    progress = inserted,
                    "✅ 入库 #{} [{}] {}",
                    inserted,
                    score,
                    title_short
                );
            }
            Err(e) => {
                tracing::warn!(
                    title = %item.title.chars().take(30).collect::<String>(),
                    error = %e,
                    "⚠️ 入库失败"
                );
            }
        }
    }

    let filter_elapsed = filter_start.elapsed().as_secs_f64();
    tracing::info!(
        inserted = inserted,
        filtered_score = filtered_by_score,
        filtered_time = filtered_by_time,
        filtered_dup = filtered_by_duplicate,
        elapsed_sec = filter_elapsed,
        "✅ [5/5] 筛选完成 - 入库{}条 | 低分过滤{}条 | 超时过滤{}条 | 重复过滤{}条 ({:.1}s)",
        inserted,
        filtered_by_score,
        filtered_by_time,
        filtered_by_duplicate,
        filter_elapsed
    );

    let net_change = inserted as i64 - expired_deleted as i64;
    let final_count = news_repo::count_news(pool, user_id).await.unwrap_or(0);

    if inserted == 0 && expired_deleted == 0 {
        tracing::info!("ℹ️ 无变化：无过期新闻且24小时内无高价值新文章");
    } else if inserted == 0 {
        tracing::info!(
            deleted = expired_deleted,
            remaining = final_count,
            "📉 仅清理：删除{}条过期新闻，剩余{}条",
            expired_deleted,
            final_count
        );
    } else if expired_deleted == 0 {
        tracing::info!(
            inserted = inserted,
            total = final_count,
            "📈 仅新增：+{}条高价值资讯，总计{}条",
            inserted,
            final_count
        );
    } else {
        tracing::info!(
            deleted = expired_deleted,
            inserted = inserted,
            net_change = net_change,
            total = final_count,
            "🔄 更新完成：删除{}条 → 新增{}条 → 净变化{:+}条（总计{}条）",
            expired_deleted,
            inserted,
            net_change,
            final_count
        );
    }

    let total_elapsed = start_time.elapsed().as_secs_f64();
    tracing::info!(
        deleted = expired_deleted,
        inserted = inserted,
        net_change = net_change,
        total_elapsed_sec = total_elapsed,
        "🎉 [新闻拉取] 全部完成！删除{} → 新增{} → 净变化{:+} (耗时 {:.1}s)",
        expired_deleted,
        inserted,
        net_change,
        total_elapsed
    );

    let result = FetchNewsResult {
        deleted: expired_deleted,
        inserted,
        net_change,
        total: final_count,
    };

    let _ = intelligence_v4_service::instrument(
        pool, "system", "news", "fetch",
        Some(&format!("inserted: {}, total: {}", inserted, final_count)),
    ).await;

    // A5 Phase 3 Task 3: 在线拉取成功后同步写入离线缓存（news_offline_cache）
    // 失败不阻塞主流程，仅记录日志（离线缓存非关键路径）
    if let Err(e) = sync_to_offline_cache(pool).await {
        tracing::warn!(
            error = %e,
            "[A5-Phase3-Task3] 同步新闻到离线缓存失败（不影响主流程）"
        );
    }

    Ok(result)
}

/// 将 news_cache 主存储当前内容镜像到 news_offline_cache（离线快照）
///
/// - 在 fetch_and_cache_news 成功后调用
/// - 取 news_cache 全量最新条目（按 fetched_at DESC，最多 100 条）
/// - 转换为 NewsCacheItem 并通过 news_cache_service::cache_news_items 批量写入
async fn sync_to_offline_cache(pool: &SqlitePool) -> Result<(), AppError> {
    let latest_news: Vec<NewsCache> = sqlx::query_as::<_, NewsCache>(
        "SELECT id, title, url, source, summary, content, category, published_at,
                fetched_at, is_read, is_favorite, ai_summary
         FROM news_cache
         ORDER BY fetched_at DESC
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    if latest_news.is_empty() {
        tracing::debug!("[A5-Phase3-Task3] news_cache 为空，跳过离线缓存同步");
        return Ok(());
    }

    let cached_at = chrono::Utc::now().to_rfc3339();
    let items: Vec<NewsCacheItem> = latest_news
        .iter()
        .map(|n| {
            let source = n.source.clone().unwrap_or_else(|| "unknown".to_string());
            let id = news_cache_service::build_cache_id(
                &source,
                n.url.as_deref(),
                &n.title,
            );
            NewsCacheItem {
                id,
                source,
                title: n.title.clone(),
                content: n.content.clone().or_else(|| n.summary.clone()),
                url: n.url.clone(),
                published_at: n.published_at.clone(),
                fetched_at: n.fetched_at.to_string(),
                cached_at: cached_at.clone(),
            }
        })
        .collect();

    let written = news_cache_service::cache_news_items(pool, items).await?;
    tracing::info!(
        written = written,
        "[A5-Phase3-Task3] 新闻离线缓存同步完成（{} 条）",
        written
    );
    Ok(())
}

async fn fetch_single_source(
    client: &Client,
    source: &NewsSource,
) -> Result<Vec<ParsedItem>, AppError> {
    let response = client
        .get(&source.url)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("请求失败: {}", e)))?;

    let body = response
        .text()
        .await
        .map_err(|e| AppError::Internal(format!("读取响应失败: {}", e)))?;

    let items = match source.feed_type.as_str() {
        "rss" => parse_rss(&body)?,
        "atom" => parse_atom(&body)?,
        _ => return Ok(Vec::new()),
    };

    Ok(items
        .into_iter()
        .filter(|item| !item.title.trim().is_empty())
        .collect())
}

fn parse_rss(xml: &str) -> Result<Vec<ParsedItem>, AppError> {
    let feed: RssFeed = from_str(xml)
        .map_err(|e| AppError::Internal(format!("RSS解析失败: {}", e)))?;

    Ok(feed
        .channel
        .item
        .into_iter()
        .map(|i| ParsedItem {
            title: i.title,
            link: i.link,
            description: i.description,
            pub_date: i.pub_date,
        })
        .collect())
}

fn parse_atom(xml: &str) -> Result<Vec<ParsedItem>, AppError> {
    let feed: AtomFeed = from_str(xml)
        .map_err(|e| AppError::Internal(format!("Atom解析失败: {}", e)))?;

    Ok(feed
        .entry
        .into_iter()
        .map(|e| {
            let link = e
                .link
                .iter()
                .find_map(|l| l.href.clone())
                .or_else(|| {
                    e.link
                        .first()
                        .and_then(|l| l.href.clone())
                });

            ParsedItem {
                title: e.title,
                link,
                description: e.summary,
                pub_date: e.published,
            }
        })
        .collect())
}

struct ParsedItem {
    title: String,
    link: Option<String>,
    description: Option<String>,
    pub_date: Option<String>,
}

fn parse_date(raw: &Option<String>) -> Option<String> {
    let raw = raw.as_ref()?;
    let raw = raw.trim();

    let formats = [
        "%a, %d %b %Y %H:%M:%S %z",
        "%a, %d %b %Y %H:%M:%S %Z",
        "%Y-%m-%dT%H:%M:%S%:z",
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M:%SZ",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d",
    ];

    for fmt in &formats {
        if let Ok(dt) = chrono::DateTime::parse_from_str(raw, fmt) {
            return Some(dt.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(raw, fmt) {
            return Some(dt.format("%Y-%m-%d %H:%M:%S").to_string());
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(raw, fmt) {
            return Some(d.format("%Y-%m-%d 00:00:00").to_string());
        }
    }

    None
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace('\n', " ")
        .replace('\r', " ");

    let result = result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if result.chars().count() > 500 {
        let truncated: String = result.chars().take(500).collect();
        format!("{}...", truncated)
    } else {
        result
    }
}

pub async fn toggle_news_favorite(pool: &SqlitePool, user_id: i64, id: i64, is_favorite: bool) -> Result<(), AppError> {
    news_repo::toggle_favorite(pool, user_id, id, is_favorite).await
}

pub async fn get_pending_delete_news(pool: &SqlitePool) -> Result<Vec<crate::models::news::NewsPendingDelete>, AppError> {
    news_repo::get_pending_delete_news(pool, 50).await
}

pub async fn move_old_news_to_pending(pool: &SqlitePool, ids_to_move: Vec<i64>) -> Result<u64, AppError> {
    if ids_to_move.is_empty() {
        return Ok(0);
    }

    tracing::info!(
        count = ids_to_move.len(),
        "📦 移动{}条旧新闻到待删除列表（保留收藏新闻）",
        ids_to_move.len()
    );

    news_repo::clear_pending_delete(pool).await?;
    let moved = news_repo::move_to_pending_delete(pool, ids_to_move).await?;

    Ok(moved)
}

// ========== AI 智能新闻短报 ==========

/// 调用底层智能 LLM 对新闻内容进行阅读分析，生成结构化短报
/// 受限于本地模型速度，此过程可能需要较长时间（10-60秒），前端需展示加载状态
pub async fn generate_news_ai_summary(
    pool: &SqlitePool,
    user_id: i64,
    id: i64,
    provider: Option<String>,
    endpoint: Option<String>,
    model: Option<String>,
    is_github: Option<bool>,
) -> Result<NewsCache, AppError> {
    let news = news_repo::get_news_by_id(pool, user_id, id).await?;

    let content = if let Some(ref c) = news.content {
        if !c.trim().is_empty() { c.clone() }
        else { news.summary.clone().unwrap_or_default() }
    } else {
        news.summary.clone().unwrap_or_default()
    };

    if content.is_empty() {
        return Err(AppError::Validation("新闻内容为空，无法生成摘要".into()));
    }

    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let endpoint = endpoint.unwrap_or_else(|| "http://localhost:11434".to_string());
    let model = model.unwrap_or_else(|| "qwen3:8b".to_string());

    let system_prompt = if is_github.unwrap_or(false) {
        // GitHub 项目专用 Prompt：先理解项目内容，再生成介绍
        "你是一个专业的开源项目分析师。\
        请阅读以下 GitHub 项目信息，输出一份简洁的项目介绍。\n\n\
        要求：\n\
        1. 第一行：一句话概括项目核心功能（不超过40字）\n\
        2. 接下来用「📌 功能」格式列出2-3个核心功能点\n\
        3. 用「🔧 技术栈」格式列出主要技术栈/语言\n\
        4. 用「⭐ 亮点」格式给出1-2个该项目值得关注的理由\n\
        5. 最后用「🏷 标签」格式给出3-5个分类标签\n\n\
        格式示例：\n\
        一个高性能的终端复用工具，支持会话持久化和多路复用。\n\
        📌 功能：终端会话后台保持、断网重连、多窗口共享同一会话\n\
        📌 功能：支持SSH/Tmux/Screen自动检测\n\
        🔧 技术栈：Rust | TUI | async-std\n\
        ⭐ 亮点：零配置开箱即用，内存占用<5MB\n\
        🏷 标签：终端工具 | 会话管理 | Rust | CLI".to_string()
    } else {
        // 普通新闻 Prompt
        "你是一个专业的网络安全与技术资讯分析助手。\
        请阅读以下新闻内容，输出一份简洁的短报。\n\n\
        要求：\n\
        1. 第一行：一句话核心摘要（不超过50字），直接概括核心信息\n\
        2. 接下来用「📌 要点」格式列出2-3个关键要点\n\
        3. 最后用「🏷 标签」格式给出3-5个分类标签\n\n\
        格式示例：\n\
        某组织发布新型勒索软件变种，利用CVE-2024漏洞进行横向移动攻击。\n\
        📌 要点：攻击者利用CVE-2024漏洞获取初始访问权限\n\
        📌 要点：勒索软件使用AES+RSA双重加密，暂无解密工具\n\
        📌 要点：建议立即更新补丁并加强网络分段\n\
        🏷 标签：勒索软件 | CVE-2024 | 漏洞利用 | 应急响应".to_string()
    };

    let user_prompt = format!(
        "【新闻标题】{}\n\n【新闻来源】{}\n\n【新闻正文】\n{}",
        news.title,
        news.source.as_deref().unwrap_or("未知"),
        if content.len() > 3000 { &content[..3000] } else { &content }
    );

    tracing::info!(
        news_id = id,
        provider = %provider,
        model = %model,
        "🤖 [AI新闻短报] 开始调用LLM生成摘要..."
    );

    let start = std::time::Instant::now();
    let result = crate::services::intelligence_v4_service::call_llm_chat(
        &provider,
        &endpoint,
        None,
        Some(&model),
        &system_prompt,
        &user_prompt,
    ).await?;

    let elapsed = start.elapsed().as_secs_f64();
    tracing::info!(
        news_id = id,
        elapsed_secs = elapsed,
        result_len = result.len(),
        "✅ [AI新闻短报] LLM生成完成 ({:.1}s)", elapsed
    );

    news_repo::update_ai_summary(pool, user_id, id, &result).await?;

    let updated = news_repo::get_news_by_id(pool, user_id, id).await?;
    Ok(updated)
}

