use chrono::{DateTime, Duration, NaiveDateTime, Utc};

use crate::models::*;
use crate::repository::*;
use crate::repository::DbPool;

pub struct AccessControlService;

impl AccessControlService {
    pub async fn handle_swipe_card(
        pool: &DbPool,
        req: SwipeCardRequest,
    ) -> Result<SwipeCardResponse, AppError> {
        if req.card_no.trim().is_empty() {
            return Err(AppError::Validation("卡号不能为空".to_string()));
        }
        if req.door_no.trim().is_empty() {
            return Err(AppError::Validation("门号不能为空".to_string()));
        }

        let direction = Direction::from_str(&req.direction.clone().unwrap_or_else(|| "in".to_string()));
        let swiped_at = match &req.swiped_at {
            Some(s) => s.clone(),
            None => Utc::now().to_rfc3339(),
        };

        let (person_type, person_id, person_name, department, access_result, deny_reason, message) =
            Self::authenticate_card(pool, &req.card_no, &swiped_at).await?;

        let window_start = {
            let dt = parse_datetime(&swiped_at);
            let window_start_dt = dt - Duration::seconds(DEFAULT_DEDUP_WINDOW_SECONDS);
            window_start_dt.to_rfc3339()
        };

        if let Some(existing) = AccessRecordRepository::find_latest_within_window(
            pool,
            &req.card_no,
            direction.as_str(),
            &req.door_no,
            &window_start,
        ).await? {
            tracing::info!(
                "重复刷卡已去重 - 卡号: {}, 门号: {}, 方向: {}, 原记录: {}, 原时间: {}",
                req.card_no,
                req.door_no,
                direction.as_str(),
                existing.record_no,
                existing.swiped_at
            );

            return Ok(SwipeCardResponse {
                success: access_result == AccessResult::Allowed,
                record_no: existing.record_no.clone(),
                access_result: AccessResult::from_str(&existing.access_result),
                person_type: PersonType::from_str(&existing.person_type),
                person_name: existing.person_name.clone(),
                department: existing.department.clone(),
                deny_reason: existing.deny_reason.clone(),
                swiped_at: existing.swiped_at.clone(),
                message: format!("【重复刷卡】{}（首次通行时间：{}）", message, existing.swiped_at),
                is_duplicate: true,
            });
        }

        let record_no = generate_record_no();

        let access_record = AccessRecord {
            id: 0,
            record_no: record_no.clone(),
            card_no: req.card_no.clone(),
            person_type: person_type.as_str().to_string(),
            person_id,
            person_name: person_name.clone(),
            department: department.clone(),
            door_no: req.door_no.clone(),
            direction: direction.as_str().to_string(),
            access_result: access_result.as_str().to_string(),
            deny_reason: deny_reason.clone(),
            swiped_at: swiped_at.clone(),
            created_at: Utc::now().to_rfc3339(),
        };

        AccessRecordRepository::create(pool, &access_record).await?;

        tracing::info!(
            "刷卡处理完成 - 卡号: {}, 人员类型: {}, 结果: {}, 门号: {}, 时间: {}",
            req.card_no,
            person_type.as_str(),
            access_result.as_str(),
            req.door_no,
            swiped_at
        );

        Ok(SwipeCardResponse {
            success: access_result == AccessResult::Allowed,
            record_no,
            access_result,
            person_type,
            person_name,
            department,
            deny_reason,
            swiped_at,
            message,
            is_duplicate: false,
        })
    }

    async fn authenticate_card(
        pool: &DbPool,
        card_no: &str,
        swiped_at: &str,
    ) -> Result<(PersonType, Option<i64>, Option<String>, Option<String>, AccessResult, Option<String>, String), AppError> {
        if let Some(employee) = EmployeeRepository::find_by_card_no(pool, card_no).await? {
            if employee.status != 1 {
                return Ok((
                    PersonType::Employee,
                    Some(employee.id),
                    Some(employee.name),
                    Some(employee.department),
                    AccessResult::Denied,
                    Some("员工卡已停用".to_string()),
                    format!("员工{}({})已被停用，禁止通行", employee.name, employee.employee_no),
                ));
            }
            return Ok((
                PersonType::Employee,
                Some(employee.id),
                Some(employee.name.clone()),
                Some(employee.department.clone()),
                AccessResult::Allowed,
                None,
                format!("欢迎{}，{}，通行已记录", employee.department, employee.name),
            ));
        }

        if let Some(visitor) = VisitorRepository::find_by_card_no(pool, card_no).await? {
            if visitor.status != 1 {
                return Ok((
                    PersonType::Visitor,
                    Some(visitor.id),
                    Some(visitor.name),
                    Some(format!("访客-{}", visitor.purpose)),
                    AccessResult::Denied,
                    Some("访客卡已停用".to_string()),
                    format!("访客{}({})已被停用，禁止通行", visitor.name, visitor.visitor_no),
                ));
            }

            let now = parse_datetime(swiped_at);
            let valid_from = parse_datetime(&visitor.valid_from);
            let valid_to = parse_datetime(&visitor.valid_to);

            if now < valid_from {
                return Ok((
                    PersonType::Visitor,
                    Some(visitor.id),
                    Some(visitor.name),
                    Some(format!("访客-{}", visitor.purpose)),
                    AccessResult::Denied,
                    Some("访客卡尚未生效".to_string()),
                    format!("访客{}的通行权限尚未生效", visitor.name),
                ));
            }
            if now > valid_to {
                return Ok((
                    PersonType::Visitor,
                    Some(visitor.id),
                    Some(visitor.name),
                    Some(format!("访客-{}", visitor.purpose)),
                    AccessResult::Denied,
                    Some("访客卡已过期".to_string()),
                    format!("访客{}的通行权限已过期", visitor.name),
                ));
            }

            return Ok((
                PersonType::Visitor,
                Some(visitor.id),
                Some(visitor.name.clone()),
                Some(format!("访客-{}", visitor.purpose)),
                AccessResult::Allowed,
                None,
                format!("欢迎访客{}，来访事由：{}", visitor.name, visitor.purpose),
            ));
        }

        Ok((
            PersonType::Unknown,
            None,
            None,
            None,
            AccessResult::Denied,
            Some("未识别的卡片".to_string()),
            "无效的门禁卡，禁止通行".to_string(),
        ))
    }

    pub async fn query_records(
        pool: &DbPool,
        query: AccessRecordQuery,
    ) -> Result<PaginatedResponse<AccessRecord>, AppError> {
        let (items, total) = AccessRecordRepository::query(pool, &query).await?;

        let page = query.page.max(1);
        let page_size = query.page_size.max(1).min(100);
        let total_pages = ((total as f64 / page_size as f64).ceil() as u32);

        Ok(PaginatedResponse {
            items,
            total,
            page,
            page_size,
            total_pages,
        })
    }
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc);
    }
    Utc::now()
}

pub struct EmployeeService;

impl EmployeeService {
    pub async fn list(pool: &DbPool) -> Result<Vec<Employee>, AppError> {
        EmployeeRepository::list_all(pool).await
    }

    pub async fn create(
        pool: &DbPool,
        req: CreateEmployeeRequest,
    ) -> Result<Employee, AppError> {
        if req.employee_no.trim().is_empty() {
            return Err(AppError::Validation("员工编号不能为空".to_string()));
        }
        if req.name.trim().is_empty() {
            return Err(AppError::Validation("姓名不能为空".to_string()));
        }
        if req.card_no.trim().is_empty() {
            return Err(AppError::Validation("卡号不能为空".to_string()));
        }
        EmployeeRepository::create(pool, &req).await
    }

    pub async fn get(pool: &DbPool, id: i64) -> Result<Employee, AppError> {
        EmployeeRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("员工(id={})不存在", id)))
    }
}

pub struct VisitorService;

impl VisitorService {
    pub async fn list(pool: &DbPool) -> Result<Vec<Visitor>, AppError> {
        VisitorRepository::list_all(pool).await
    }

    pub async fn create(
        pool: &DbPool,
        req: CreateVisitorRequest,
    ) -> Result<Visitor, AppError> {
        if req.visitor_no.trim().is_empty() {
            return Err(AppError::Validation("访客编号不能为空".to_string()));
        }
        if req.name.trim().is_empty() {
            return Err(AppError::Validation("姓名不能为空".to_string()));
        }
        if req.card_no.trim().is_empty() {
            return Err(AppError::Validation("卡号不能为空".to_string()));
        }
        if req.purpose.trim().is_empty() {
            return Err(AppError::Validation("来访事由不能为空".to_string()));
        }
        VisitorRepository::create(pool, &req).await
    }

    pub async fn get(pool: &DbPool, id: i64) -> Result<Visitor, AppError> {
        VisitorRepository::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("访客(id={})不存在", id)))
    }
}

pub struct StatsService;

impl StatsService {
    pub async fn get_hourly_stats(
        pool: &DbPool,
        query: HourlyStatsQuery,
    ) -> Result<HourlyStatsResponse, AppError> {
        let (start_date, end_date, raw_items) =
            AccessRecordRepository::get_hourly_stats(pool, &query).await?;

        let mut aggregated: std::collections::HashMap<String, HourlyStatsAggregated> =
            std::collections::HashMap::new();

        let mut summary = FlowSummary::default();
        let mut hourly_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();

        for item in &raw_items {
            let entry = aggregated.entry(item.hour.clone()).or_insert(HourlyStatsAggregated {
                hour: item.hour.clone(),
                hour_of_day: item.hour_of_day,
                in_count: 0,
                out_count: 0,
                employee_in: 0,
                employee_out: 0,
                visitor_in: 0,
                visitor_out: 0,
                unknown_in: 0,
                unknown_out: 0,
                net_flow: 0,
            });

            let count = item.count;
            let is_in = item.direction == "in";

            if is_in {
                entry.in_count += count;
                summary.total_in += count;
            } else {
                entry.out_count += count;
                summary.total_out += count;
            }

            match item.person_type.as_str() {
                "employee" => {
                    if is_in {
                        entry.employee_in += count;
                    } else {
                        entry.employee_out += count;
                    }
                    summary.employee_total += count;
                }
                "visitor" => {
                    if is_in {
                        entry.visitor_in += count;
                    } else {
                        entry.visitor_out += count;
                    }
                    summary.visitor_total += count;
                }
                _ => {
                    if is_in {
                        entry.unknown_in += count;
                    } else {
                        entry.unknown_out += count;
                    }
                    summary.unknown_total += count;
                }
            }

            *hourly_totals.entry(item.hour.clone()).or_insert(0) += count;
        }

        for entry in aggregated.values_mut() {
            entry.net_flow = entry.in_count - entry.out_count;
        }

        summary.net_flow = summary.total_in - summary.total_out;

        if let Some((peak_hour, peak_count)) = hourly_totals
            .into_iter()
            .max_by_key(|&(_, count)| count)
        {
            summary.peak_hour = Some(peak_hour);
            summary.peak_hour_count = peak_count;
        }

        let mut items: Vec<HourlyStatsAggregated> = aggregated.into_values().collect();
        items.sort_by(|a, b| a.hour.cmp(&b.hour));

        Ok(HourlyStatsResponse {
            start_date,
            end_date,
            items,
            summary,
        })
    }

    pub async fn get_daily_stats(
        pool: &DbPool,
        query: DailyStatsQuery,
    ) -> Result<DailyStatsResponse, AppError> {
        let (start_date, end_date, raw_items) =
            AccessRecordRepository::get_daily_stats(pool, &query).await?;

        let mut aggregated: std::collections::HashMap<String, DailyStatsAggregated> =
            std::collections::HashMap::new();

        let mut summary = FlowSummary::default();
        let mut daily_totals: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();

        for item in &raw_items {
            let entry = aggregated.entry(item.date.clone()).or_insert(DailyStatsAggregated {
                date: item.date.clone(),
                in_count: 0,
                out_count: 0,
                employee_in: 0,
                employee_out: 0,
                visitor_in: 0,
                visitor_out: 0,
                unknown_in: 0,
                unknown_out: 0,
                net_flow: 0,
            });

            let count = item.count;
            let is_in = item.direction == "in";

            if is_in {
                entry.in_count += count;
                summary.total_in += count;
            } else {
                entry.out_count += count;
                summary.total_out += count;
            }

            match item.person_type.as_str() {
                "employee" => {
                    if is_in {
                        entry.employee_in += count;
                    } else {
                        entry.employee_out += count;
                    }
                    summary.employee_total += count;
                }
                "visitor" => {
                    if is_in {
                        entry.visitor_in += count;
                    } else {
                        entry.visitor_out += count;
                    }
                    summary.visitor_total += count;
                }
                _ => {
                    if is_in {
                        entry.unknown_in += count;
                    } else {
                        entry.unknown_out += count;
                    }
                    summary.unknown_total += count;
                }
            }

            *daily_totals.entry(item.date.clone()).or_insert(0) += count;
        }

        for entry in aggregated.values_mut() {
            entry.net_flow = entry.in_count - entry.out_count;
        }

        summary.net_flow = summary.total_in - summary.total_out;

        if let Some((peak_date, peak_count)) = daily_totals
            .into_iter()
            .max_by_key(|&(_, count)| count)
        {
            summary.peak_date = Some(peak_date);
            summary.peak_date_count = peak_count;
        }

        let mut items: Vec<DailyStatsAggregated> = aggregated.into_values().collect();
        items.sort_by(|a, b| a.date.cmp(&b.date));

        if let Some(max_item) = items.iter().max_by_key(|item| item.in_count + item.out_count) {
            summary.peak_hour_count = max_item.in_count + max_item.out_count;
        }

        Ok(DailyStatsResponse {
            start_date,
            end_date,
            items,
            summary,
        })
    }
}
