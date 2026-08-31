use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditContent {
    pub content: String,
    pub format_type: String,
    pub file_size: u64,
    pub is_binary_reconstructed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEditStatus {
    pub exists: bool,
    pub file_size: u64,
    pub modified_at: i64,
    pub is_writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSheetData {
    pub name: String,
    pub rows: Vec<Vec<String>>,
    pub row_count: usize,
    pub col_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataResult {
    pub sheets: Vec<TableSheetData>,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadTableRequest {
    pub path: String,
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTableRequest {
    pub path: String,
    pub ext: String,
    pub sheets: Vec<TableSheetData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportCsvRequest {
    pub path: String,
    pub ext: String,
    pub rows: Vec<Vec<String>>,
    pub sheet_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptSlideData {
    pub index: usize,
    pub name: String,
    pub text_content: String,
    pub raw_xml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptSlidesResult {
    pub slides: Vec<PptSlideData>,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptUpdateSlideRequest {
    pub path: String,
    pub slide_index: usize,
    pub text_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PptReorderRequest {
    pub path: String,
    pub slide_index: usize,
    pub new_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfSaveRequest {
    pub path: String,
    pub base64_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSaveRequest {
    pub path: String,
    pub base64_data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSaveRequest {
    pub path: String,
    pub base64_data: String,
}