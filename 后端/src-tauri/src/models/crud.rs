use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilters {
    pub page: u32,
    pub page_size: u32,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub search: Option<String>,
}

impl Default for QueryFilters {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            sort_by: None,
            sort_order: Some("DESC".to_string()),
            search: None,
        }
    }
}

impl QueryFilters {
    pub fn offset(&self) -> i64 {
        ((self.page.max(1) - 1) * self.page_size.min(100)) as i64
    }

    pub fn limit(&self) -> i64 {
        self.page_size.min(100) as i64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}