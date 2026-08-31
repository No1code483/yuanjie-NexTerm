use std::collections::HashMap;
use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::db::repositories::editor_repo;
use crate::error::app_error::AppError;
use crate::models::editor::{DiffLine, VersionCleanupRequest, VersionCleanupResult, VersionDiffResult};
use crate::services::editor_storage_service::EditorStorageService;

pub async fn diff_versions(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    version_a: i64,
    version_b: i64,
) -> Result<VersionDiffResult, AppError> {
    let ver_a = editor_repo::get_version(pool, doc_uuid, version_a)
        .await?
        .ok_or(AppError::NotFound)?;
    let ver_b = editor_repo::get_version(pool, doc_uuid, version_b)
        .await?
        .ok_or(AppError::NotFound)?;

    let path_a = PathBuf::from(&ver_a.file_path);
    let path_b = PathBuf::from(&ver_b.file_path);

    let text_a = storage.read_encrypted(&path_a, mek)?;
    let text_b = storage.read_encrypted(&path_b, mek)?;

    let lines_a: Vec<&str> = text_a.lines().collect();
    let lines_b: Vec<&str> = text_b.lines().collect();

    let lcs = compute_lcs(&lines_a, &lines_b);
    let diff_lines = build_diff(&lines_a, &lines_b, &lcs);

    let lines_added = diff_lines.iter().filter(|d| d.line_type == "added").count() as i64;
    let lines_removed = diff_lines.iter().filter(|d| d.line_type == "removed").count() as i64;
    let lines_unchanged = diff_lines.iter().filter(|d| d.line_type == "unchanged").count() as i64;

    Ok(VersionDiffResult {
        doc_uuid: doc_uuid.to_string(),
        version_a,
        version_b,
        lines_added,
        lines_removed,
        lines_unchanged,
        diff_lines,
    })
}

pub async fn cleanup_versions(
    pool: &SqlitePool,
    storage: &EditorStorageService,
    req: VersionCleanupRequest,
) -> Result<VersionCleanupResult, AppError> {
    let all_versions = editor_repo::get_versions(pool, &req.doc_uuid).await?;
    let mut to_remove: Vec<i64> = Vec::new();

    if let Some(keep_latest) = req.keep_latest {
        if (all_versions.len() as i64) > keep_latest {
            let skip = keep_latest as usize;
            for v in &all_versions[skip..] {
                to_remove.push(v.version_num);
            }
        }
    }

    if let Some(older_than) = req.older_than_ms {
        let cutoff = chrono::Utc::now().timestamp_millis() - older_than;
        for v in &all_versions {
            if v.created_at < cutoff && !to_remove.contains(&v.version_num) {
                to_remove.push(v.version_num);
            }
        }
    }

    let mut removed_count: i64 = 0;
    for ver_num in &to_remove {
        let ver_path = storage.version_path(&req.doc_uuid, *ver_num);
        if ver_path.exists() {
            std::fs::remove_file(&ver_path).ok();
        }

        sqlx::query("DELETE FROM editor_versions WHERE doc_uuid = ? AND version_num = ?")
            .bind(&req.doc_uuid)
            .bind(ver_num)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        removed_count += 1;
    }

    let remaining = all_versions.len() as i64 - removed_count;

    Ok(VersionCleanupResult {
        doc_uuid: req.doc_uuid,
        removed_count,
        remaining_count: remaining,
    })
}

fn compute_lcs(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 0..m {
        for j in 0..n {
            if a[i] == b[j] {
                dp[i + 1][j + 1] = dp[i][j] + 1;
            } else {
                dp[i + 1][j + 1] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut result = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

fn build_diff(
    a: &[&str],
    b: &[&str],
    lcs: &[(usize, usize)],
) -> Vec<DiffLine> {
    let mut result = Vec::new();
    let mut lcs_map: HashMap<usize, usize> = HashMap::new();
    for &(ai, bi) in lcs {
        lcs_map.insert(ai, bi);
    }

    let mut ia = 0;
    let mut ib = 0;

    while ia < a.len() || ib < b.len() {
        if ia < a.len() && lcs_map.contains_key(&ia) {
            let matched_bi = lcs_map[&ia];
            while ib < matched_bi {
                result.push(DiffLine {
                    line_type: "added".into(),
                    old_line_no: None,
                    new_line_no: Some(ib as i64 + 1),
                    content: b[ib].to_string(),
                });
                ib += 1;
            }
            result.push(DiffLine {
                line_type: "unchanged".into(),
                old_line_no: Some(ia as i64 + 1),
                new_line_no: Some(ib as i64 + 1),
                content: a[ia].to_string(),
            });
            ia += 1;
            ib += 1;
        } else if ia < a.len() {
            result.push(DiffLine {
                line_type: "removed".into(),
                old_line_no: Some(ia as i64 + 1),
                new_line_no: None,
                content: a[ia].to_string(),
            });
            ia += 1;
        } else {
            result.push(DiffLine {
                line_type: "added".into(),
                old_line_no: None,
                new_line_no: Some(ib as i64 + 1),
                content: b[ib].to_string(),
            });
            ib += 1;
        }
    }

    result
}