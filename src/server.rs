use crate::config::{get_process, update_process, ProcessConfig, ProcessStatus, PROCESSES};
use crate::process::{restart_process, start_process, stop_process};
use axum::{
    extract::Path,
    http::{header, StatusCode, Request},
    response::{IntoResponse},
    routing::{get, post},
    Json, Router, body::Body,
};
use chrono::Local;
use include_dir::{include_dir, Dir};
use mime_guess::from_path;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;

use tokio::net::TcpListener;

// 嵌入静态文件
static STATIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");

// 服务器状态
#[derive(Clone)]
struct AppState {
    // 可以添加一些服务器状态
}

// 启动服务器
pub async fn start_server(host: &str,port: u16) -> anyhow::Result<()> {
    _ = host;
    // 启动进程监控
    crate::process::start_process_monitor();

    // 创建路由
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/processes", get(list_processes_handler))
        .route("/api/processes/:name", get(get_process_handler))
        .route("/api/processes/:name/start", post(|path, req| async { start_process_handler(path, req).await }))
        .route("/api/processes/:name/stop", post(stop_process_handler))
        .route("/api/processes/:name/restart", post(restart_process_handler))
        .route("/api/processes/:name", post(update_process_handler))
        .route("/api/processes/:name", axum::routing::delete(delete_process_handler))
        .route("/static/*path", get(static_handler));

    // 绑定地址
    // let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let addr = SocketAddr::from_str(&format!("{}:{}", host, port))?;
    let listener = TcpListener::bind(addr).await?;
    println!("Server listening on http://{}", addr);

    // 启动服务器
    axum::Server::from_tcp(listener.into_std()?)?
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// 首页处理器
async fn index_handler() -> impl IntoResponse {
    static_handler(Path("index.html".to_string())).await
}

// 静态文件处理器
async fn static_handler(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    
    // 如果路径为空，默认返回 index.html
    let file_path = if path.is_empty() {
        "index.html"
    } else {
        path
    };
    
    // 从嵌入的静态文件中读取
    if let Some(file) = STATIC_DIR.get_file(file_path) {
        let content = file.contents();
        let mime_type = from_path(file_path).first_or_octet_stream();
        
        let mime_str = mime_type.as_ref().to_string();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime_str)],
            content.to_vec()
        );
    }
    
    // 文件未找到
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "text/plain".to_string())],
        "File not found".as_bytes().to_vec()
    )
}

// 进程列表请求处理器
async fn list_processes_handler() -> impl IntoResponse {
    let processes = PROCESSES.read().unwrap();
    Json(processes.clone())
}

// 获取单个进程处理器
async fn get_process_handler(Path(name): Path<String>) -> impl IntoResponse {
    if let Some(process) = get_process(&name) {
        return (StatusCode::OK, Json(process)).into_response();
    }
    
    (StatusCode::NOT_FOUND, "Process not found").into_response()
}

// 启动进程请求
#[derive(Deserialize)]
struct StartProcessRequest {
    #[serde(default)]
    command: String,
    working_dir: Option<String>,
    env: Option<HashMap<String, String>>,
    auto_restart: Option<bool>,
}

// 启动进程处理器
async fn start_process_handler(
    Path(name): Path<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    // 检查Content-Type是否为application/json
    let is_json = request.headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("application/json"))
        .unwrap_or(false);
        
    if !is_json {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            [(header::CONTENT_TYPE, "text/plain".to_string())],
            "Unsupported Media Type: Content-Type must be application/json".as_bytes().to_vec(),
        );
    }

    // 解析JSON请求体
    let payload = match hyper::body::to_bytes(request.into_body()).await {
        Ok(bytes) => {
            if bytes.is_empty() {
                // 如果请求体为空，使用默认值
                StartProcessRequest {
                    command: String::new(),
                    working_dir: None,
                    env: None,
                    auto_restart: None,
                }
            } else {
                match serde_json::from_slice::<StartProcessRequest>(&bytes) {
                    Ok(payload) => payload,
                    Err(_) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            [(header::CONTENT_TYPE, "text/plain".to_string())],
                            "Invalid JSON payload".as_bytes().to_vec(),
                        );
                    }
                }
            }
        }
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                [(header::CONTENT_TYPE, "text/plain".to_string())],
                "Failed to read request body".as_bytes().to_vec(),
            );
        }
    };
    // 将环境变量转换为Vec<String>格式
    let mut env_vars = payload.env.map(|map| {
        map.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
    });
    
    // 根据服务名称读取配置
    let config = get_process(&name);
    
    // 如果配置存在，使用配置中的命令和工作目录
    let (command, working_dir, env_to_use) = if let Some(cfg) = &config {
        // 如果进程已经在运行，直接返回成功
        if cfg.status == ProcessStatus::Running {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain".to_string())],
                format!("Process {} is already running", name).as_bytes().to_vec(),
            );
        }
        
        // 优先使用请求中的参数，如果没有则使用配置中的参数
        let cmd = if payload.command.is_empty() { &cfg.command } else { &payload.command };
        let dir = payload.working_dir.as_deref().or(cfg.working_dir.as_deref());
        
        // 确定使用哪个环境变量集
        let env = if env_vars.is_some() {
            env_vars.as_ref()
        } else if let Some(cfg_env) = &cfg.env {
            // 将配置中的环境变量转换为Vec<String>
            let env_vec: Vec<String> = cfg_env.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            
            env_vars = Some(env_vec);
            env_vars.as_ref()
        } else {
            None
        };
        
        (cmd, dir, env)
    } else {
        // 如果配置不存在，使用请求中的参数
        (&payload.command, payload.working_dir.as_deref(), env_vars.as_ref())
    };
    
    match start_process(
        &name,
        command,
        working_dir,
        env_to_use,
    ) {
        Ok(_) => {
            // 更新自动重启设置
            if let Some(auto_restart) = payload.auto_restart {
                if let Some(mut config) = get_process(&name) {
                    config.auto_restart = auto_restart;
                    let _ = update_process(config);
                }
            }
            
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain".to_string())],
                format!("Process {} started successfully", name).as_bytes().to_vec(),
            )
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "text/plain".to_string())],
                e.to_string().as_bytes().to_vec(),
            )
        }
    }
}

// 停止进程处理器
async fn stop_process_handler(Path(name): Path<String>) -> impl IntoResponse {
    match stop_process(&name) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// 重启进程处理器
async fn restart_process_handler(Path(name): Path<String>) -> impl IntoResponse {
    match restart_process(&name) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// 更新进程请求
#[derive(Deserialize)]
struct UpdateProcessRequest {
    command: String,
    working_dir: Option<String>,
    env: Option<HashMap<String, String>>,
    auto_restart: bool,
}

// 更新进程处理器
async fn update_process_handler(
    Path(name): Path<String>,
    Json(payload): Json<UpdateProcessRequest>,
) -> impl IntoResponse {
    let now = Local::now().to_rfc3339();
    
    // 检查进程是否存在
    let mut config = if let Some(existing) = get_process(&name) {
        existing
    } else {
        // 创建新的进程配置
        ProcessConfig {
            start_time: Some(std::time::SystemTime::now()),
            name: name.clone(),
            command: payload.command.clone(),
            working_dir: payload.working_dir.clone(),
            env: payload.env.clone(),
            auto_restart: payload.auto_restart,
            status: ProcessStatus::Stopped,
            pid: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        }
    };
    
    // 更新配置
    config.command = payload.command;
    config.working_dir = payload.working_dir;
    config.env = payload.env;
    config.auto_restart = payload.auto_restart;
    config.updated_at = now;
    
    match update_process(config) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// 删除进程处理器
async fn delete_process_handler(Path(name): Path<String>) -> impl IntoResponse {
    // 先停止进程
    let _ = stop_process(&name);
    
    // 删除进程配置
    match crate::config::remove_process(&name) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}