use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    response::IntoResponse,
};

use crate::models::*;
use crate::repository::DbPool;
use crate::service::*;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Database(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库错误: {}", msg)),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("系统错误: {}", msg)),
        };
        tracing::error!("API错误: {:?} - {}", status, message);
        (status, Json(ApiResponse::<()>::error(&message))).into_response()
    }
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "access-control-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub async fn swipe_card(
    State(state): State<AppState>,
    Json(req): Json<SwipeCardRequest>,
) -> Result<impl IntoResponse, AppError> {
    let resp = AccessControlService::handle_swipe_card(&state.pool, req).await?;
    Ok(Json(ApiResponse::success(resp)))
}

pub async fn query_access_records(
    State(state): State<AppState>,
    Query(query): Query<AccessRecordQuery>,
) -> Result<impl IntoResponse, AppError> {
    let resp = AccessControlService::query_records(&state.pool, query).await?;
    Ok(Json(ApiResponse::success(resp)))
}

pub async fn get_access_record(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    use crate::repository::AccessRecordRepository;
    let record = AccessRecordRepository::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("通行记录(id={})不存在", id)))?;
    Ok(Json(ApiResponse::success(record)))
}

pub async fn delete_access_record(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    use crate::repository::AccessRecordRepository;
    let deleted = AccessRecordRepository::delete_by_id(&state.pool, id).await?;
    if !deleted {
        return Err(AppError::NotFound(format!("通行记录(id={})不存在", id)));
    }
    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "deleted": true
    }))))
}

pub async fn list_employees(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let employees = EmployeeService::list(&state.pool).await?;
    Ok(Json(ApiResponse::success(employees)))
}

pub async fn create_employee(
    State(state): State<AppState>,
    Json(req): Json<CreateEmployeeRequest>,
) -> Result<impl IntoResponse, AppError> {
    let employee = EmployeeService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(employee))))
}

pub async fn get_employee(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let employee = EmployeeService::get(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(employee)))
}

pub async fn list_visitors(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let visitors = VisitorService::list(&state.pool).await?;
    Ok(Json(ApiResponse::success(visitors)))
}

pub async fn create_visitor(
    State(state): State<AppState>,
    Json(req): Json<CreateVisitorRequest>,
) -> Result<impl IntoResponse, AppError> {
    let visitor = VisitorService::create(&state.pool, req).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(visitor))))
}

pub async fn get_visitor(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let visitor = VisitorService::get(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(visitor)))
}

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    use sqlx::query_scalar;

    let total_employees: i64 = query_scalar("SELECT COUNT(*) FROM employees WHERE status = 1")
        .fetch_one(&state.pool)
        .await?;

    let total_visitors: i64 = query_scalar("SELECT COUNT(*) FROM visitors WHERE status = 1")
        .fetch_one(&state.pool)
        .await?;

    let total_records: i64 = query_scalar("SELECT COUNT(*) FROM access_records")
        .fetch_one(&state.pool)
        .await?;

    let today_records: i64 = query_scalar(
        "SELECT COUNT(*) FROM access_records WHERE date(swiped_at) = date('now')"
    )
    .fetch_one(&state.pool)
    .await?;

    let today_allowed: i64 = query_scalar(
        "SELECT COUNT(*) FROM access_records WHERE date(swiped_at) = date('now') AND access_result = 'allowed'"
    )
    .fetch_one(&state.pool)
    .await?;

    let today_denied: i64 = query_scalar(
        "SELECT COUNT(*) FROM access_records WHERE date(swiped_at) = date('now') AND access_result = 'denied'"
    )
    .fetch_one(&state.pool)
    .await?;

    let employee_records: i64 = query_scalar(
        "SELECT COUNT(*) FROM access_records WHERE date(swiped_at) = date('now') AND person_type = 'employee'"
    )
    .fetch_one(&state.pool)
    .await?;

    let visitor_records: i64 = query_scalar(
        "SELECT COUNT(*) FROM access_records WHERE date(swiped_at) = date('now') AND person_type = 'visitor'"
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "total_employees": total_employees,
        "total_active_visitors": total_visitors,
        "total_records": total_records,
        "today": {
            "total": today_records,
            "allowed": today_allowed,
            "denied": today_denied,
            "employee_count": employee_records,
            "visitor_count": visitor_records,
        }
    }))))
}
