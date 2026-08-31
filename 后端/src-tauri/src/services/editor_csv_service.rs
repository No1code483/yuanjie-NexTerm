use std::io::Cursor;

use csv::ReaderBuilder;

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;
use crate::models::editor::CsvInfo;
use crate::services::editor_storage_service::EditorStorageService;

const DEFAULT_MAX_ROWS: usize = 500;

pub fn read_csv_preview(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    max_rows: Option<usize>,
    delimiter: Option<char>,
) -> Result<CsvInfo, AppError> {
    let content_path = storage.content_path(doc_uuid);
    let raw = std::fs::read(&content_path).map_err(AppError::FileSystem)?;

    if raw.len() < 12 {
        return Err(AppError::Validation("文件过小".into()));
    }

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let nonce: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| AppError::Crypto("nonce 错误".into()))?;

    let plaintext = aes_gcm::decrypt_bytes(ciphertext, mek, &nonce)?;

    let text = String::from_utf8(plaintext)
        .map_err(|e| AppError::Crypto(format!("UTF-8 解码失败: {}", e)))?;

    let delim = delimiter.unwrap_or(detect_delimiter(&text));
    let limit = max_rows.unwrap_or(DEFAULT_MAX_ROWS);

    let mut reader = ReaderBuilder::new()
        .delimiter(delim as u8)
        .has_headers(true)
        .flexible(true)
        .from_reader(Cursor::new(&text));

    let headers = reader
        .headers()
        .map_err(|e| AppError::Validation(format!("CSV 表头解析失败: {}", e)))?
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>();

    let column_count = headers.len();
    let mut rows: Vec<Vec<String>> = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| AppError::Validation(format!("CSV 行解析失败: {}", e)))?;
        let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
        rows.push(row);
        if rows.len() >= limit {
            break;
        }
    }

    let row_count = rows.len();

    Ok(CsvInfo {
        headers,
        row_count,
        column_count,
        rows,
        delimiter: delim,
    })
}

fn detect_delimiter(text: &str) -> char {
    let first_line = text.lines().next().unwrap_or("");
    let counts = [
        (',', first_line.chars().filter(|&c| c == ',').count()),
        ('\t', first_line.chars().filter(|&c| c == '\t').count()),
        (';', first_line.chars().filter(|&c| c == ';').count()),
    ];

    counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(delim, count)| if *count > 0 { *delim } else { ',' })
        .unwrap_or(',')
}