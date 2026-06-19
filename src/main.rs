mod models;
mod repository;
mod service;
mod handlers;

use std::net::SocketAddr;

use axum::{
    Router,
    routing::{delete, get, post},
};

use dotenvy::dotenv;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::handlers::*;
use crate::repository::{create_pool, run_migrations};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "access_control=debug,tower_http=debug,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:access_control.db?mode=rwc".to_string());
    let server_addr = std::env::var("SERVER_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
        .parse::<SocketAddr>()?;

    info!("正在连接数据库: {}", database_url);
    let pool = create_pool(&database_url).await?;

    info!("正在执行数据库迁移...");
    run_migrations(&pool).await?;
    info!("数据库迁移完成");

    let app_state = AppState { pool: pool.clone() };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/stats", get(get_stats))
        .route("/stats/hourly", get(get_hourly_stats))
        .route("/stats/daily", get(get_daily_stats))
        .route("/access/swipe", post(swipe_card))
        .route("/access/records", get(query_access_records))
        .route("/access/records/:id", get(get_access_record).delete(delete_access_record))
        .route("/employees", get(list_employees).post(create_employee))
        .route("/employees/:id", get(get_employee))
        .route("/visitors", get(list_visitors).post(create_visitor))
        .route("/visitors/:id", get(get_visitor))
        .with_state(app_state);

    info!("门禁控制服务启动中，监听地址: {}", server_addr);
    info!("API 端点:");
    info!("  POST   /access/swipe           - 接收刷卡事件");
    info!("  GET    /access/records         - 查询通行记录");
    info!("  GET    /access/records/:id     - 获取单条记录详情");
    info!("  DELETE /access/records/:id     - 删除通行记录");
    info!("  GET    /employees              - 员工列表");
    info!("  POST   /employees              - 新增员工");
    info!("  GET    /employees/:id          - 员工详情");
    info!("  GET    /visitors               - 访客列表");
    info!("  POST   /visitors               - 新增访客");
    info!("  GET    /visitors/:id           - 访客详情");
    info!("  GET    /stats                  - 统计概览");
    info!("  GET    /stats/hourly           - 分时进出人流量统计");
    info!("  GET    /stats/daily            - 分日进出人流量统计");
    info!("  GET    /health                 - 健康检查");

    let listener = tokio::net::TcpListener::bind(server_addr).await?;
    axum::serve(listener, app)
        .await?;

    Ok(())
}
