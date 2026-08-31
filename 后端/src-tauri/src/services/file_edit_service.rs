use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use crate::error::app_error::AppError;
use crate::models::file_edit::{
    FileEditContent, FileEditStatus, TableDataResult, TableSheetData,
    PptSlideData, PptSlidesResult,
};
use crate::utils::file_path::canonicalize_user_path;
use calamine::Reader;
use regex::Regex;
use zip::ZipArchive;
use zip::write::SimpleFileOptions;

const TEXT_EXTS: &[&str] = &[
    "txt", "md", "json", "xml", "yaml", "yml", "toml", "cfg", "conf", "ini",
    "csv", "html", "htm", "css", "js", "ts", "jsx", "tsx", "py", "rs", "go",
    "java", "c", "cpp", "h", "sh", "bash", "sql", "rst", "log", "r", "lua",
    "php", "rb", "swift", "kt", "scala", "dart", "tex", "bat", "ps1",
];

pub fn read_file_for_edit(path: &str, ext: &str) -> Result<FileEditContent, AppError> {
    // 安全审计修复（发现 2，HIGH）：原实现直接 `fs::read_to_string(path)` 无路径校验，
    // 攻击者可读取任意文件（如 `~/.ssh/id_rsa`）。
    // 现统一调用 `canonicalize_user_path`：
    // 1. 拒绝空路径
    // 2. 拒绝含 `..` 段的路径
    // 3. canonicalize 父目录解析符号链接
    let canon_path = canonicalize_user_path(path)?;
    let path_str = canon_path.to_string_lossy();

    if !canon_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path_str)));
    }

    let metadata = fs::metadata(&canon_path).map_err(|e| AppError::FileSystem(e))?;
    let file_size = metadata.len();

    if file_size > 100 * 1024 * 1024 {
        return Err(AppError::Validation("文件过大（超过 100MB），无法编辑".into()));
    }

    let ext_lower = ext.to_lowercase();

    if TEXT_EXTS.contains(&ext_lower.as_str()) {
        let content = fs::read_to_string(&canon_path).map_err(|e| {
            AppError::FileSystem(std::io::Error::new(e.kind(), format!("读取文件失败: {}", e)))
        })?;
        return Ok(FileEditContent {
            content,
            format_type: "text".into(),
            file_size,
            is_binary_reconstructed: false,
        });
    }

    if ext_lower == "docx" {
        return read_docx_text(&path_str, file_size);
    }

    if ext_lower == "pptx" {
        return read_pptx_text(&path_str, file_size);
    }

    Err(AppError::Validation(format!("不支持编辑的文件格式: .{}", ext)))
}

pub fn write_file_after_edit(path: &str, content: &str, ext: &str) -> Result<(), AppError> {
    // 安全审计修复（发现 2，HIGH）：与 read_file_for_edit 一致，写入前规范化路径
    let canon_path = canonicalize_user_path(path)?;
    let path_str = canon_path.to_string_lossy();

    let ext_lower = ext.to_lowercase();

    if TEXT_EXTS.contains(&ext_lower.as_str()) {
        fs::write(&canon_path, content).map_err(|e| AppError::FileSystem(e))?;
        return Ok(());
    }

    if ext_lower == "docx" {
        return write_docx(&path_str, content);
    }

    if ext_lower == "pptx" {
        return write_pptx(&path_str, content);
    }

    Err(AppError::Validation(format!("不支持保存的文件格式: .{}", ext)))
}

pub fn get_edit_status(path: &str) -> Result<FileEditStatus, AppError> {
    let file_path = Path::new(path);
    let exists = file_path.exists();

    if !exists {
        return Ok(FileEditStatus {
            exists: false,
            file_size: 0,
            modified_at: 0,
            is_writable: false,
        });
    }

    let metadata = fs::metadata(path).map_err(|e| AppError::FileSystem(e))?;
    let modified_at = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let is_writable = !metadata.permissions().readonly();

    Ok(FileEditStatus {
        exists: true,
        file_size: metadata.len(),
        modified_at,
        is_writable,
    })
}

fn read_docx_text(path: &str, file_size: u64) -> Result<FileEditContent, AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut doc_xml = archive.by_name("word/document.xml")
        .map_err(|_| AppError::Validation("不是有效的 DOCX 文件（缺少 word/document.xml）".into()))?;

    let mut xml_str = String::new();
    doc_xml.read_to_string(&mut xml_str).map_err(|e| AppError::FileSystem(e))?;

    Ok(FileEditContent {
        content: xml_str,
        format_type: "xml".into(),
        file_size,
        is_binary_reconstructed: false,
    })
}

fn write_docx(path: &str, content: &str) -> Result<(), AppError> {
    rebuild_zip_with_replacement(path, "word/document.xml", content.as_bytes())
}

fn read_pptx_text(path: &str, file_size: u64) -> Result<FileEditContent, AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();

    if slide_names.is_empty() {
        return Ok(FileEditContent {
            content: String::new(),
            format_type: "xml".into(),
            file_size,
            is_binary_reconstructed: false,
        });
    }

    let mut result = String::new();
    for name in &slide_names {
        if let Ok(mut f) = archive.by_name(name) {
            let mut xml_str = String::new();
            if f.read_to_string(&mut xml_str).is_ok() {
                if !result.is_empty() {
                    result.push_str("\n<!-- === SLIDE_SEPARATOR === -->\n");
                }
                result.push_str(&format!("<!-- slide: {} -->\n", name));
                result.push_str(&xml_str);
            }
        }
    }

    Ok(FileEditContent {
        content: result,
        format_type: "xml".into(),
        file_size,
        is_binary_reconstructed: false,
    })
}

fn write_pptx(path: &str, content: &str) -> Result<(), AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let parts: Vec<&str> = content.split("\n<!-- === SLIDE_SEPARATOR === -->\n").collect();

    let slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();

    if parts.len() != slide_names.len() {
        return Err(AppError::Validation(format!(
            "幻灯片数量不匹配：原始 {} 张，编辑后 {} 张。请保持幻灯片数量不变。",
            slide_names.len(),
            parts.len()
        )));
    }

    let temp_path = format!("{}.tmp", path);

    {
        let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let out_file = fs::File::create(&temp_path).map_err(|e| AppError::FileSystem(e))?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut replacement_index = 0usize;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();

            let is_slide = name.starts_with("ppt/slides/slide") && name.ends_with(".xml");

            writer.start_file(&name, options)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            if is_slide {
                let slide_header = format!("<!-- slide: {} -->\n", name);
                let replacement = parts.iter().find(|p| p.starts_with(&slide_header));

                if let Some(rep) = replacement {
                    let clean_content = rep.strip_prefix(&slide_header).unwrap_or(rep);
                    writer.write_all(clean_content.as_bytes())
                        .map_err(|e| AppError::FileSystem(e))?;
                } else if replacement_index < parts.len() {
                    writer.write_all(parts[replacement_index].as_bytes())
                        .map_err(|e| AppError::FileSystem(e))?;
                    replacement_index += 1;
                } else {
                    let mut buf = Vec::new();
                    entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
                    writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
                }
            } else {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
                writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
            }
        }

        writer.finish()
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(())
}

pub fn read_table_for_edit(path: &str, ext: &str) -> Result<TableDataResult, AppError> {
    let file_path = Path::new(path);
    if !file_path.exists() {
        return Err(AppError::Validation(format!("文件不存在: {}", path)));
    }

    let ext_lower = ext.to_lowercase();

    if ext_lower == "csv" {
        return read_csv_table(path);
    }

    if ["xlsx", "xls", "ods"].contains(&ext_lower.as_str()) {
        return read_calamine_table(path, &ext_lower);
    }

    Err(AppError::Validation(format!("不支持的表格格式: .{}", ext)))
}

pub fn write_table_after_edit(path: &str, ext: &str, sheets: &[TableSheetData]) -> Result<(), AppError> {
    let ext_lower = ext.to_lowercase();

    if ext_lower == "csv" {
        if let Some(sheet) = sheets.first() {
            return write_csv_table(path, sheet);
        }
        return Err(AppError::Validation("没有数据可写入".into()));
    }

    if ext_lower == "xlsx" || ext_lower == "xls" {
        return write_xlsx_table(path, sheets);
    }

    if ext_lower == "ods" {
        return Err(AppError::Validation("ODS 格式暂不支持写入，请另存为 XLSX".into()));
    }

    Err(AppError::Validation(format!("不支持的表格格式: .{}", ext)))
}

pub fn export_table_csv(path: &str, ext: &str, rows: &[Vec<String>]) -> Result<String, AppError> {
    let ext_lower = ext.to_lowercase();
    let base = path.strip_suffix(&format!(".{}", ext_lower))
        .unwrap_or(path);
    let csv_path = format!("{}_export.csv", base);

    let mut csv_content = String::new();
    for row in rows {
        let escaped: Vec<String> = row.iter().map(|cell| {
            if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                format!("\"{}\"", cell.replace('"', "\"\""))
            } else {
                cell.clone()
            }
        }).collect();
        csv_content.push_str(&escaped.join(","));
        csv_content.push('\n');
    }

    fs::write(&csv_path, &csv_content).map_err(|e| AppError::FileSystem(e))?;

    Ok(csv_path)
}

fn read_csv_table(path: &str) -> Result<TableDataResult, AppError> {
    let content = fs::read_to_string(path)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(e.kind(), format!("读取 CSV 文件失败: {}", e))))?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let cells = parse_csv_line(line);
        if cells.iter().any(|c| !c.trim().is_empty()) {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        rows.push(vec![String::new()]);
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    let row_count = rows.len();
    let col_count = max_cols;

    Ok(TableDataResult {
        sheets: vec![TableSheetData {
            name: "Sheet1".into(),
            rows,
            row_count,
            col_count,
        }],
        extension: "csv".into(),
    })
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if in_quotes {
            if ch == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    current.push('"');
                    i += 1;
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else {
            if ch == '"' {
                in_quotes = true;
            } else if ch == ',' {
                cells.push(current.clone());
                current.clear();
            } else {
                current.push(ch);
            }
        }
        i += 1;
    }
    cells.push(current);
    cells
}

fn read_calamine_table(path: &str, ext: &str) -> Result<TableDataResult, AppError> {
    let metadata = fs::metadata(path).map_err(|e| AppError::FileSystem(e))?;
    if metadata.len() > 50 * 1024 * 1024 {
        return Err(AppError::Validation("表格文件过大（超过 50MB），无法编辑".into()));
    }

    let mut workbook = calamine::open_workbook_auto(path)
        .map_err(|e| AppError::Validation(format!("无法打开表格文件: {}", e)))?;

    let sheet_names = workbook.sheet_names().to_vec();
    if sheet_names.is_empty() {
        return Err(AppError::Validation("表格文件中没有工作表".into()));
    }

    let max_rows = 2000usize;
    let mut sheets: Vec<TableSheetData> = Vec::new();

    for sheet_name in &sheet_names {
        let range = workbook.worksheet_range(sheet_name)
            .map_err(|e| AppError::Validation(format!("无法读取工作表 '{}': {}", sheet_name, e)))?;

        let mut rows: Vec<Vec<String>> = Vec::new();

        for (row_idx, row) in range.rows().enumerate() {
            if row_idx >= max_rows {
                break;
            }
            let row_data: Vec<String> = row.iter()
                .map(|cell| cell.to_string())
                .collect();
            let has_content = row_data.iter().any(|s: &String| !s.trim().is_empty());
            if !row_data.is_empty() && has_content {
                rows.push(row_data);
            }
        }

        if rows.is_empty() {
            rows.push(vec![String::new()]);
        }

        let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        for row in &mut rows {
            while row.len() < max_cols {
                row.push(String::new());
            }
        }

        let row_count = rows.len();
        let col_count = max_cols;

        sheets.push(TableSheetData {
            name: sheet_name.clone(),
            rows,
            row_count,
            col_count,
        });
    }

    Ok(TableDataResult {
        sheets,
        extension: ext.to_string(),
    })
}

fn write_csv_table(path: &str, sheet: &TableSheetData) -> Result<(), AppError> {
    let mut content = String::new();
    for row in &sheet.rows {
        let escaped: Vec<String> = row.iter().map(|cell| {
            if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                format!("\"{}\"", cell.replace('"', "\"\""))
            } else {
                cell.clone()
            }
        }).collect();
        content.push_str(&escaped.join(","));
        content.push('\n');
    }
    fs::write(path, &content).map_err(|e| AppError::FileSystem(e))?;
    Ok(())
}

fn write_xlsx_table(path: &str, sheets: &[TableSheetData]) -> Result<(), AppError> {
    let temp_path = format!("{}.tmp", path);

    {
        let mut workbook = rust_xlsxwriter::Workbook::new();

        for sheet in sheets {
            if sheet.name.is_empty() {
                continue;
            }
            let worksheet = workbook.add_worksheet().set_name(&sheet.name)
                .map_err(|e| AppError::Validation(format!("创建工作表失败: {}", e)))?;

            for (row_idx, row) in sheet.rows.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    if !cell.is_empty() {
                        worksheet.write_string(
                            row_idx as u32,
                            col_idx as u16,
                            cell,
                        ).map_err(|e| AppError::Validation(format!("写入单元格失败: {}", e)))?;
                    }
                }
            }
        }

        workbook.save(&temp_path)
            .map_err(|e| AppError::Validation(format!("保存 XLSX 失败: {}", e)))?;
    }

    fs::remove_file(path).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(e)
        }
    }).map_err(|e| AppError::FileSystem(e))?;

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(())
}

fn extract_text_from_pptx_slide_xml(xml: &str) -> String {
    let re = Regex::new(r"<a:t[^>]*>([^<]*)</a:t>").expect("PPtX slide XML regex should compile");
    let texts: Vec<&str> = re.captures_iter(xml)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str())
        .filter(|s| !s.trim().is_empty())
        .collect();
    texts.join(" ")
}

fn replace_text_in_slide_xml(xml: &str, new_text: &str) -> String {
    let re = Regex::new(r"<a:t[^>]*>([^<]*)</a:t>").expect("PPtX slide XML regex should compile");
    let new_lines: Vec<&str> = new_text.lines().filter(|l| !l.trim().is_empty()).collect();

    if new_lines.is_empty() {
        return xml.to_string();
    }

    let mut line_idx = 0usize;

    re.replace_all(xml, |caps: &regex::Captures| {
        let full_match = caps.get(0).expect("capture group 0 should exist").as_str();
        let inner_text = caps.get(1).expect("capture group 1 should exist").as_str();

        if inner_text.trim().is_empty() {
            return full_match.to_string();
        }

        if line_idx < new_lines.len() {
            let replacement = format!("<a:t>{}</a:t>", new_lines[line_idx]);
            line_idx += 1;
            replacement
        } else {
            full_match.to_string()
        }
    }).to_string()
}

pub fn get_pptx_slides(path: &str) -> Result<PptSlidesResult, AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    slide_names.sort();

    if slide_names.is_empty() {
        return Err(AppError::Validation("PPTX 文件中没有幻灯片".into()));
    }

    let mut slides: Vec<PptSlideData> = Vec::new();

    for (idx, name) in slide_names.iter().enumerate() {
        let mut xml_str = String::new();
        if let Ok(mut f) = archive.by_name(name) {
            f.read_to_string(&mut xml_str)
                .map_err(|e| AppError::FileSystem(e))?;
        }

        let text_content = extract_text_from_pptx_slide_xml(&xml_str);

        slides.push(PptSlideData {
            index: idx,
            name: name.clone(),
            text_content,
            raw_xml: xml_str,
        });
    }

    Ok(PptSlidesResult {
        slides,
        extension: "pptx".into(),
    })
}

pub fn update_pptx_slide_text(path: &str, slide_index: usize, text_content: &str) -> Result<(), AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    slide_names.sort();

    if slide_index >= slide_names.len() {
        return Err(AppError::Validation(format!(
            "幻灯片索引超出范围：{} >= {}",
            slide_index,
            slide_names.len()
        )));
    }

    let target_slide = &slide_names[slide_index];

    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    if let Ok(mut f) = archive.by_name(target_slide) {
        let mut xml_str = String::new();
        f.read_to_string(&mut xml_str)
            .map_err(|e| AppError::FileSystem(e))?;
        let new_xml = replace_text_in_slide_xml(&xml_str, text_content);
        rebuild_zip_with_replacement(path, target_slide, new_xml.as_bytes())?;
    }

    Ok(())
}

pub fn add_pptx_slide(path: &str) -> Result<usize, AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    slide_names.sort();

    let new_index = slide_names.len() + 1;
    let new_slide_name = format!("ppt/slides/slide{}.xml", new_index);

    let blank_slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p><a:r><a:rPr lang="zh-CN"/><a:t>新幻灯片</a:t></a:r><a:endParaRPr/></a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let temp_path = format!("{}.tmp", path);

    {
        let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let out_file = fs::File::create(&temp_path).map_err(|e| AppError::FileSystem(e))?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();
            writer.start_file(&name, options)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
            writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
        }

        writer.start_file(&new_slide_name, options)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        writer.write_all(blank_slide_xml.as_bytes()).map_err(|e| AppError::FileSystem(e))?;

        writer.finish()
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    fs::remove_file(path).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }
    }).map_err(|e| AppError::FileSystem(e))?;

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(new_index - 1)
}

pub fn delete_pptx_slide(path: &str, slide_index: usize) -> Result<(), AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    slide_names.sort();

    if slide_names.len() <= 1 {
        return Err(AppError::Validation("至少保留一张幻灯片".into()));
    }

    if slide_index >= slide_names.len() {
        return Err(AppError::Validation(format!(
            "幻灯片索引超出范围：{} >= {}",
            slide_index,
            slide_names.len()
        )));
    }

    let target_slide = &slide_names[slide_index];
    let temp_path = format!("{}.tmp", path);

    {
        let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let out_file = fs::File::create(&temp_path).map_err(|e| AppError::FileSystem(e))?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();
            if name == *target_slide {
                continue;
            }
            writer.start_file(&name, options)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
            writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
        }

        writer.finish()
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    fs::remove_file(path).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }
    }).map_err(|e| AppError::FileSystem(e))?;

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(())
}

pub fn reorder_pptx_slides(path: &str, slide_index: usize, new_index: usize) -> Result<(), AppError> {
    let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    slide_names.sort();

    if slide_index >= slide_names.len() || new_index >= slide_names.len() {
        return Err(AppError::Validation("幻灯片索引超出范围".into()));
    }

    if slide_index == new_index {
        return Ok(());
    }

    let moved = slide_names.remove(slide_index);
    slide_names.insert(new_index, moved);

    let temp_path = format!("{}.tmp", path);

    {
        let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let out_file = fs::File::create(&temp_path).map_err(|e| AppError::FileSystem(e))?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();

            if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                continue;
            }

            writer.start_file(&name, options)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
            writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
        }

        for (new_pos, old_name) in slide_names.iter().enumerate() {
            let new_name = format!("ppt/slides/slide{}.xml", new_pos + 1);
            if let Ok(mut entry) = archive.by_name(old_name) {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
                writer.start_file(&new_name, options)
                    .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
                writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
            }
        }

        writer.finish()
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    fs::remove_file(path).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }
    }).map_err(|e| AppError::FileSystem(e))?;

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(())
}

pub fn pdf_save(path: &str, base64_data: &str) -> Result<(), AppError> {
    media_save(path, base64_data)
}

pub fn image_save(path: &str, base64_data: &str) -> Result<(), AppError> {
    media_save(path, base64_data)
}

pub fn audio_save(path: &str, base64_data: &str) -> Result<(), AppError> {
    media_save(path, base64_data)
}

fn media_save(path: &str, base64_data: &str) -> Result<(), AppError> {
    // 安全审计修复（发现 2，HIGH）：写入前规范化路径，拒绝含 `..` 的路径
    let canon_path = canonicalize_user_path(path)?;
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine.decode(base64_data)
        .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))?;
    fs::write(&canon_path, &bytes).map_err(|e| AppError::FileSystem(e))
}

fn rebuild_zip_with_replacement(path: &str, target_entry: &str, new_content: &[u8]) -> Result<(), AppError> {
    let temp_path = format!("{}.tmp", path);

    {
        let file = fs::File::open(path).map_err(|e| AppError::FileSystem(e))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let out_file = fs::File::create(&temp_path).map_err(|e| AppError::FileSystem(e))?;
        let mut writer = zip::ZipWriter::new(out_file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            let name = entry.name().to_string();

            writer.start_file(&name, options)
                .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

            if name == target_entry {
                writer.write_all(new_content).map_err(|e| AppError::FileSystem(e))?;
            } else {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).map_err(|e| AppError::FileSystem(e))?;
                writer.write_all(&buf).map_err(|e| AppError::FileSystem(e))?;
            }
        }

        writer.finish()
            .map_err(|e| AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    }

    fs::rename(&temp_path, path).map_err(|e| AppError::FileSystem(e))?;

    Ok(())
}