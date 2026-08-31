use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

pub trait Adapter: Send + Sync {
    fn init(&mut self) -> Result<(), AppError>;
    fn run(&self) -> Result<(), AppError>;
    fn stop(&self) -> Result<(), AppError>;
    fn get_info(&self) -> AdapterInfo;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AdapterInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}
