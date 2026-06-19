use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::fs;
use std::path::Path;

use crate::models::*;

pub type DbPool = SqlitePool;

pub async fn create_pool(database_url: &str) -> Result<DbPool, AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &DbPool) -> Result<(), AppError> {
    let migrations_dir = Path::new("migrations");
    if migrations_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(migrations_dir)
            .map_err(|e| AppError::Internal(format!("读取 migrations 目录失败: {}", e)))?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            if path.extension().map(|e| e == "sql").unwrap_or(false) {
                let sql = fs::read_to_string(&path)
                    .map_err(|e| AppError::Internal(format!("读取 SQL 文件失败: {}", e)))?;
                sqlx::query(&sql)
                    .execute(pool)
                    .await
                    .map_err(|e| AppError::Database(format!("执行迁移 {:?} 失败: {}", path, e)))?;
                tracing::info!("迁移执行成功: {:?}", path);
            }
        }
    }

    Ok(())
}

pub struct EmployeeRepository;

impl EmployeeRepository {
    pub async fn find_by_card_no(pool: &DbPool, card_no: &str) -> Result<Option<Employee>, AppError> {
        let employee = sqlx::query_as::<_, Employee>(
            "SELECT * FROM employees WHERE card_no = ? AND status = 1 LIMIT 1"
        )
        .bind(card_no)
        .fetch_optional(pool)
        .await?;
        Ok(employee)
    }

    pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<Employee>, AppError> {
        let employee = sqlx::query_as::<_, Employee>(
            "SELECT * FROM employees WHERE id = ? LIMIT 1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(employee)
    }

    pub async fn list_all(pool: &DbPool) -> Result<Vec<Employee>, AppError> {
        let employees = sqlx::query_as::<_, Employee>(
            "SELECT * FROM employees ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;
        Ok(employees)
    }

    pub async fn create(pool: &DbPool, req: &CreateEmployeeRequest) -> Result<Employee, AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO employees (employee_no, name, department, card_no, position, phone, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&req.employee_no)
        .bind(&req.name)
        .bind(&req.department)
        .bind(&req.card_no)
        .bind(&req.position)
        .bind(&req.phone)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let id = result.last_insert_rowid();
        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::Database("创建员工后查询失败".to_string()))
    }
}

pub struct VisitorRepository;

impl VisitorRepository {
    pub async fn find_by_card_no(pool: &DbPool, card_no: &str) -> Result<Option<Visitor>, AppError> {
        let visitor = sqlx::query_as::<_, Visitor>(
            "SELECT * FROM visitors WHERE card_no = ? AND status = 1 LIMIT 1"
        )
        .bind(card_no)
        .fetch_optional(pool)
        .await?;
        Ok(visitor)
    }

    pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<Visitor>, AppError> {
        let visitor = sqlx::query_as::<_, Visitor>(
            "SELECT * FROM visitors WHERE id = ? LIMIT 1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(visitor)
    }

    pub async fn list_all(pool: &DbPool) -> Result<Vec<Visitor>, AppError> {
        let visitors = sqlx::query_as::<_, Visitor>(
            "SELECT * FROM visitors ORDER BY id DESC"
        )
        .fetch_all(pool)
        .await?;
        Ok(visitors)
    }

    pub async fn create(pool: &DbPool, req: &CreateVisitorRequest) -> Result<Visitor, AppError> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO visitors (visitor_no, name, id_card_no, company, purpose, visited_employee_id, card_no, valid_from, valid_to, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&req.visitor_no)
        .bind(&req.name)
        .bind(&req.id_card_no)
        .bind(&req.company)
        .bind(&req.purpose)
        .bind(&req.visited_employee_id)
        .bind(&req.card_no)
        .bind(&req.valid_from)
        .bind(&req.valid_to)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;

        let id = result.last_insert_rowid();
        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::Database("创建访客后查询失败".to_string()))
    }
}

pub struct AccessRecordRepository;

impl AccessRecordRepository {
    pub async fn create(pool: &DbPool, record: &AccessRecord) -> Result<AccessRecord, AppError> {
        sqlx::query(
            "INSERT INTO access_records (record_no, card_no, person_type, person_id, person_name, department, door_no, direction, access_result, deny_reason, swiped_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&record.record_no)
        .bind(&record.card_no)
        .bind(&record.person_type)
        .bind(&record.person_id)
        .bind(&record.person_name)
        .bind(&record.department)
        .bind(&record.door_no)
        .bind(&record.direction)
        .bind(&record.access_result)
        .bind(&record.deny_reason)
        .bind(&record.swiped_at)
        .bind(&record.created_at)
        .execute(pool)
        .await?;

        Self::find_by_record_no(pool, &record.record_no)
            .await?
            .ok_or_else(|| AppError::Database("创建记录后查询失败".to_string()))
    }

    pub async fn find_by_record_no(pool: &DbPool, record_no: &str) -> Result<Option<AccessRecord>, AppError> {
        let record = sqlx::query_as::<_, AccessRecord>(
            "SELECT * FROM access_records WHERE record_no = ? LIMIT 1"
        )
        .bind(record_no)
        .fetch_optional(pool)
        .await?;
        Ok(record)
    }

    pub async fn find_latest_within_window(
        pool: &DbPool,
        card_no: &str,
        direction: &str,
        door_no: &str,
        from_time: &str,
    ) -> Result<Option<AccessRecord>, AppError> {
        let record = sqlx::query_as::<_, AccessRecord>(
            "SELECT * FROM access_records
             WHERE card_no = ? AND direction = ? AND door_no = ? AND swiped_at >= ?
             ORDER BY swiped_at DESC
             LIMIT 1"
        )
        .bind(card_no)
        .bind(direction)
        .bind(door_no)
        .bind(from_time)
        .fetch_optional(pool)
        .await?;
        Ok(record)
    }

    pub async fn query(
        pool: &DbPool,
        query: &AccessRecordQuery,
    ) -> Result<(Vec<AccessRecord>, i64), AppError> {
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<&str> = Vec::new();

        if let Some(card_no) = &query.card_no {
            conditions.push("card_no = ?".to_string());
            params.push(card_no);
        }
        if let Some(person_type) = &query.person_type {
            conditions.push("person_type = ?".to_string());
            params.push(person_type);
        }
        if let Some(door_no) = &query.door_no {
            conditions.push("door_no = ?".to_string());
            params.push(door_no);
        }
        if let Some(start_time) = &query.start_time {
            conditions.push("swiped_at >= ?".to_string());
            params.push(start_time);
        }
        if let Some(end_time) = &query.end_time {
            conditions.push("swiped_at <= ?".to_string());
            params.push(end_time);
        }
        if let Some(direction) = &query.direction {
            conditions.push("direction = ?".to_string());
            params.push(direction);
        }
        if let Some(access_result) = &query.access_result {
            conditions.push("access_result = ?".to_string());
            params.push(access_result);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM access_records {}", where_clause);
        let data_sql = format!(
            "SELECT * FROM access_records {} ORDER BY swiped_at DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        for p in &params {
            count_query = count_query.bind(p);
        }
        let total: i64 = count_query.fetch_one(pool).await?;

        let offset = (query.page.saturating_sub(1)) * query.page_size;
        let mut data_query = sqlx::query_as::<_, AccessRecord>(&data_sql);
        for p in &params {
            data_query = data_query.bind(p);
        }
        data_query = data_query.bind(query.page_size as i32).bind(offset as i32);
        let items = data_query.fetch_all(pool).await?;

        Ok((items, total))
    }

    pub async fn find_by_id(pool: &DbPool, id: i64) -> Result<Option<AccessRecord>, AppError> {
        let record = sqlx::query_as::<_, AccessRecord>(
            "SELECT * FROM access_records WHERE id = ? LIMIT 1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;
        Ok(record)
    }

    pub async fn delete_by_id(pool: &DbPool, id: i64) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM access_records WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_hourly_stats(
        pool: &DbPool,
        query: &HourlyStatsQuery,
    ) -> Result<(String, String, Vec<HourlyStatsItem>), AppError> {
        use chrono::{Duration, Utc};

        let end_date = query
            .end_date
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
        let start_date = query
            .start_date
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                (Utc::now() - Duration::days(6))
                    .format("%Y-%m-%d")
                    .to_string()
            });

        let mut conditions: Vec<String> = vec!["access_result = 'allowed'".to_string()];
        conditions.push("date(swiped_at) >= date(?)".to_string());
        conditions.push("date(swiped_at) <= date(?)".to_string());

        let mut params: Vec<String> = vec![start_date.clone(), end_date.clone()];

        if let Some(person_type) = &query.person_type {
            conditions.push("person_type = ?".to_string());
            params.push(person_type.clone());
        }
        if let Some(door_no) = &query.door_no {
            conditions.push("door_no = ?".to_string());
            params.push(door_no.clone());
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let sql = format!(
            "SELECT
                strftime('%Y-%m-%d %H:00', swiped_at) as hour,
                cast(strftime('%H', swiped_at) as integer) as hour_of_day,
                direction,
                person_type,
                COUNT(*) as count
             FROM access_records
             {}
             GROUP BY strftime('%Y-%m-%d %H:00', swiped_at), hour_of_day, direction, person_type
             ORDER BY hour ASC, direction ASC, person_type ASC",
            where_clause
        );

        let mut data_query = sqlx::query_as::<_, HourlyStatsItem>(&sql);
        for p in &params {
            data_query = data_query.bind(p);
        }
        let items = data_query.fetch_all(pool).await?;

        Ok((start_date, end_date, items))
    }

    pub async fn get_daily_stats(
        pool: &DbPool,
        query: &DailyStatsQuery,
    ) -> Result<(String, String, Vec<DailyStatsItem>), AppError> {
        use chrono::{Duration, Utc};

        let end_date = query
            .end_date
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
        let start_date = query
            .start_date
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                (Utc::now() - Duration::days(29))
                    .format("%Y-%m-%d")
                    .to_string()
            });

        let mut conditions: Vec<String> = vec!["access_result = 'allowed'".to_string()];
        conditions.push("date(swiped_at) >= date(?)".to_string());
        conditions.push("date(swiped_at) <= date(?)".to_string());

        let mut params: Vec<String> = vec![start_date.clone(), end_date.clone()];

        if let Some(person_type) = &query.person_type {
            conditions.push("person_type = ?".to_string());
            params.push(person_type.clone());
        }
        if let Some(door_no) = &query.door_no {
            conditions.push("door_no = ?".to_string());
            params.push(door_no.clone());
        }

        let where_clause = format!("WHERE {}", conditions.join(" AND "));

        let sql = format!(
            "SELECT
                date(swiped_at) as date,
                direction,
                person_type,
                COUNT(*) as count
             FROM access_records
             {}
             GROUP BY date(swiped_at), direction, person_type
             ORDER BY date ASC, direction ASC, person_type ASC",
            where_clause
        );

        let mut data_query = sqlx::query_as::<_, DailyStatsItem>(&sql);
        for p in &params {
            data_query = data_query.bind(p);
        }
        let items = data_query.fetch_all(pool).await?;

        Ok((start_date, end_date, items))
    }
}
