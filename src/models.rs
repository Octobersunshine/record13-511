use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const DEFAULT_DEDUP_WINDOW_SECONDS: i64 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Employee {
    pub id: i64,
    pub employee_no: String,
    pub name: String,
    pub department: String,
    pub card_no: String,
    pub position: Option<String>,
    pub phone: Option<String>,
    pub status: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Visitor {
    pub id: i64,
    pub visitor_no: String,
    pub name: String,
    pub id_card_no: Option<String>,
    pub company: Option<String>,
    pub purpose: String,
    pub visited_employee_id: Option<i64>,
    pub card_no: String,
    pub valid_from: String,
    pub valid_to: String,
    pub status: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PersonType {
    Employee,
    Visitor,
    Unknown,
}

impl PersonType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PersonType::Employee => "employee",
            PersonType::Visitor => "visitor",
            PersonType::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "employee" => PersonType::Employee,
            "visitor" => PersonType::Visitor,
            _ => PersonType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    In,
    Out,
}

impl Direction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Direction::In => "in",
            Direction::Out => "out",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "out" => Direction::Out,
            _ => Direction::In,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AccessResult {
    Allowed,
    Denied,
}

impl AccessResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessResult::Allowed => "allowed",
            AccessResult::Denied => "denied",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "denied" => AccessResult::Denied,
            _ => AccessResult::Allowed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AccessRecord {
    pub id: i64,
    pub record_no: String,
    pub card_no: String,
    pub person_type: String,
    pub person_id: Option<i64>,
    pub person_name: Option<String>,
    pub department: Option<String>,
    pub door_no: String,
    pub direction: String,
    pub access_result: String,
    pub deny_reason: Option<String>,
    pub swiped_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwipeCardRequest {
    pub card_no: String,
    pub door_no: String,
    #[serde(default = "default_direction")]
    pub direction: Option<String>,
    pub swiped_at: Option<String>,
}

fn default_direction() -> Option<String> {
    Some("in".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwipeCardResponse {
    pub success: bool,
    pub record_no: String,
    pub access_result: AccessResult,
    pub person_type: PersonType,
    pub person_name: Option<String>,
    pub department: Option<String>,
    pub deny_reason: Option<String>,
    pub swiped_at: String,
    pub message: String,
    pub is_duplicate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRecordQuery {
    pub card_no: Option<String>,
    pub person_type: Option<String>,
    pub door_no: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub direction: Option<String>,
    pub access_result: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            code: -1,
            message: message.to_string(),
            data: None,
        }
    }
}

pub fn generate_record_no() -> String {
    format!(
        "REC-{}-{}",
        Utc::now().format("%Y%m%d%H%M%S"),
        Uuid::new_v4().simple().to_string().chars().take(8).collect::<String>()
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEmployeeRequest {
    pub employee_no: String,
    pub name: String,
    pub department: String,
    pub card_no: String,
    pub position: Option<String>,
    pub phone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVisitorRequest {
    pub visitor_no: String,
    pub name: String,
    pub id_card_no: Option<String>,
    pub company: Option<String>,
    pub purpose: String,
    pub visited_employee_id: Option<i64>,
    pub card_no: String,
    pub valid_from: String,
    pub valid_to: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    Database(String),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("资源未找到: {0}")]
    NotFound(String),

    #[error("系统内部错误: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}
