use sqlx::SqlitePool;

use crate::db::repositories::news_source_repo;
use crate::error::app_error::AppError;
use crate::models::news::{NewsSource, NewsSourceRow};

pub async fn get_news_sources(pool: &SqlitePool, user_id: i64) -> Result<Vec<NewsSourceRow>, AppError> {
    news_source_repo::get_all_news_sources(pool, user_id).await
}

pub async fn add_news_source(
    pool: &SqlitePool,
    user_id: i64,
    name: &str,
    url: &str,
    category: &str,
    feed_type: &str,
) -> Result<NewsSourceRow, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("名称不能为空".into()));
    }
    if url.trim().is_empty() {
        return Err(AppError::Validation("URL不能为空".into()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::Validation("URL必须以http://或https://开头".into()));
    }
    let valid_categories = [
        "security", "ai", "programming",
        "vulnerability", "attack_defense", "tool_application",
        "tech_innovation", "cloud_native", "open_source",
        "github",
    ];
    if !valid_categories.contains(&category) {
        return Err(AppError::Validation(format!(
            "无效的分类，有效分类: {}",
            valid_categories.join(", ")
        )));
    }
    let valid_types = ["rss", "atom"];
    if !valid_types.contains(&feed_type) {
        return Err(AppError::Validation("无效的Feed类型".into()));
    }

    news_source_repo::add_news_source(pool, user_id, name.trim(), url.trim(), category, feed_type).await
}

pub async fn delete_news_source(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool, AppError> {
    news_source_repo::delete_news_source(pool, user_id, id).await
}

pub async fn get_news_sources_for_fetch(pool: &SqlitePool, user_id: i64) -> Vec<NewsSource> {
    match news_source_repo::get_all_news_sources(pool, user_id).await {
        Ok(rows) => {
            if rows.is_empty() {
                get_default_sources()
            } else {
                rows.into_iter()
                    .map(|r| NewsSource {
                        name: r.name,
                        url: r.url,
                        category: r.category,
                        feed_type: r.feed_type,
                    })
                    .collect()
            }
        }
        Err(_) => get_default_sources(),
    }
}

fn get_default_sources() -> Vec<NewsSource> {
    vec![
        // ===== 网安 (security) =====
        NewsSource { name: "FreeBuf".into(), url: "https://www.freebuf.com/feed".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "安全客".into(), url: "https://www.anquanke.com/feed".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "安全内参".into(), url: "https://www.secrss.com/feed".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "The Hacker News".into(), url: "https://feeds.feedburner.com/TheHackersNews".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "BleepingComputer".into(), url: "https://www.bleepingcomputer.com/feed/".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "Security Affairs".into(), url: "https://securityaffairs.com/feed".into(), category: "security".into(), feed_type: "rss".into() },

        // ===== 漏洞发现与解决 (vulnerability → 网安) =====
        NewsSource { name: "NVD News".into(), url: "https://nvd.nist.gov/feeds/xml/cve/misc/nvd-rss.xml".into(), category: "vulnerability".into(), feed_type: "rss".into() },
        NewsSource { name: "Seebug".into(), url: "https://www.seebug.org/rss/new".into(), category: "vulnerability".into(), feed_type: "rss".into() },
        NewsSource { name: "Exploit DB".into(), url: "https://feeds.exploit-db.com/exploitdb".into(), category: "vulnerability".into(), feed_type: "rss".into() },
        NewsSource { name: "Google 安全博客".into(), url: "https://googleonlinesecurity.blogspot.com/feeds/posts/default".into(), category: "vulnerability".into(), feed_type: "atom".into() },
        NewsSource { name: "CISA 已知利用漏洞".into(), url: "https://www.cisa.gov/known-exploited-vulnerabilities-catalog.xml".into(), category: "vulnerability".into(), feed_type: "rss".into() },

        // ===== 攻防方案 (attack_defense → 网安) =====
        NewsSource { name: "奇安信威胁情报".into(), url: "https://ti.qianxin.com/feed".into(), category: "attack_defense".into(), feed_type: "rss".into() },
        NewsSource { name: "火线Zone".into(), url: "https://www.huoxian.cn/feed".into(), category: "attack_defense".into(), feed_type: "rss".into() },
        NewsSource { name: "Dark Reading".into(), url: "https://www.darkreading.com/rss.xml".into(), category: "attack_defense".into(), feed_type: "rss".into() },

        // ===== 工具应用 (tool_application → 编程) =====
        NewsSource { name: "GitHub Trending (全语言)".into(), url: "https://mshibanami.github.io/GitHubTrendingRSS/daily/all.xml".into(), category: "github".into(), feed_type: "rss".into() },
        NewsSource { name: "GitHub Trending (Rust)".into(), url: "https://mshibanami.github.io/GitHubTrendingRSS/daily/rust.xml".into(), category: "github".into(), feed_type: "rss".into() },
        NewsSource { name: "GitHub Trending (Python)".into(), url: "https://mshibanami.github.io/GitHubTrendingRSS/daily/python.xml".into(), category: "github".into(), feed_type: "rss".into() },
        NewsSource { name: "GitHub Trending (Go)".into(), url: "https://mshibanami.github.io/GitHubTrendingRSS/daily/go.xml".into(), category: "github".into(), feed_type: "rss".into() },
        NewsSource { name: "Dev.to".into(), url: "https://dev.to/feed".into(), category: "tool_application".into(), feed_type: "rss".into() },

        // ===== 技术创新 (tech_innovation → AI) =====
        NewsSource { name: "机器之心".into(), url: "https://www.jiqizhixin.com/rss".into(), category: "tech_innovation".into(), feed_type: "rss".into() },
        NewsSource { name: "量子位".into(), url: "https://www.qbitai.com/feed".into(), category: "tech_innovation".into(), feed_type: "rss".into() },
        NewsSource { name: "Hacker News (首页)".into(), url: "https://hnrss.org/frontpage".into(), category: "tech_innovation".into(), feed_type: "rss".into() },

        // ===== 云原生 (cloud_native → 编程) =====
        NewsSource { name: "CNCF Blog".into(), url: "https://www.cncf.io/blog/feed/".into(), category: "cloud_native".into(), feed_type: "rss".into() },

        // ===== 开源 (open_source → 编程) =====
        NewsSource { name: "阮一峰".into(), url: "https://www.ruanyifeng.com/blog/atom.xml".into(), category: "open_source".into(), feed_type: "atom".into() },
        NewsSource { name: "Solidot".into(), url: "https://www.solidot.org/index.rss".into(), category: "open_source".into(), feed_type: "rss".into() },

        // ===== HN 频道聚合 =====
        NewsSource { name: "Hacker News (安全)".into(), url: "https://hnrss.org/security".into(), category: "security".into(), feed_type: "rss".into() },
        NewsSource { name: "Hacker News (编程)".into(), url: "https://hnrss.org/programming".into(), category: "programming".into(), feed_type: "rss".into() },
    ]
}