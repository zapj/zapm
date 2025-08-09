use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

// 配置文件路径
pub static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    #[cfg(target_os = "windows")]
    {
        let home = dirs::home_dir().expect("Could not find home directory");
        home.join(".zapm")
    }
    #[cfg(target_os = "linux")]
    {
        let etc_path = PathBuf::from("/etc");
        etc_path.join("zapm")
    }
});

// 进程配置文件路径
pub static PROCESS_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let path = CONFIG_PATH.join("processes.yaml");
    if !path.exists() {
        let dir = path.parent().unwrap();
        if !dir.exists() {
            fs::create_dir_all(dir).expect("Failed to create config directory");
        }
        fs::write(&path, "").expect("Failed to create process config file");
    }
    path
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConf {
    pub host : String,
    pub port: u16,
    #[serde(skip_serializing)]
    pub api_base_url: String,
    
}

// 进程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub start_time: Option<std::time::SystemTime>,
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub auto_restart: bool,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq,Copy)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Failed,
    Unknown,
}

pub static SERVER_CONF : Lazy<RwLock<ServerConf>> = Lazy::new(|| {
    let path = CONFIG_PATH.as_path().join("zapm.yaml");
    let mut server_conf = match fs::read_to_string(&path) {
        Ok(content) => {
            serde_yaml::from_str::<ServerConf>(&content).unwrap_or_else(|_| {
                let server_conf = ServerConf { host: "localhost".to_string() ,port: 2400 , api_base_url: "http://localhost:2400".to_string()};
                let zapm_yaml_rs = serde_yaml::to_string(&server_conf);
                if let Ok(yaml_str) = zapm_yaml_rs {
                    let _ = fs::write(path, yaml_str);
                }
                server_conf    
            })
        }
        Err(_) => {
            let server_conf = ServerConf { host: "localhost".to_string() ,port: 2400 , api_base_url: "http://localhost:2400".to_string()};
            let zapm_yaml_rs = serde_yaml::to_string(&server_conf);
            if let Ok(yaml_str) = zapm_yaml_rs {
                let _ = fs::write(path, yaml_str);
            }
            server_conf
        }
    };
    server_conf.api_base_url = format!("http://{}:{}",&server_conf.host,&server_conf.port);
    RwLock::new(server_conf)
});
// 全局进程配置
pub static PROCESSES: Lazy<RwLock<HashMap<String, ProcessConfig>>> = Lazy::new(|| {
    let path = PROCESS_CONFIG_PATH.as_path();
    
    // 带错误处理的初始化
    let processes = match fs::read_to_string(path) {
        Ok(content) => {
            serde_yaml::from_str(&content).unwrap_or_else(|_| {
                eprintln!("Warning: Failed to parse process config, using empty map");
                HashMap::new()
            })
        }
        Err(e) => {
            eprintln!("Warning: Failed to read process config ({}), using empty map", e);
            HashMap::new()
        }
    };
    
    RwLock::new(processes)
});

// 初始化配置
pub fn init() -> anyhow::Result<()> {
    // 确保配置目录存在
    if !CONFIG_PATH.exists() {
        fs::create_dir_all(&*CONFIG_PATH)?;
    }
    
    // 确保进程配置文件存在
    if !PROCESS_CONFIG_PATH.exists() {
        fs::write(&*PROCESS_CONFIG_PATH, "{}")?;
    }
    
    // 加载进程配置（通过访问PROCESSES静态变量触发加载）
    let _unused = PROCESSES.read().unwrap();
    
    Ok(())
}

// 加载进程配置
fn load_processes() -> anyhow::Result<()> {
    let path = PROCESS_CONFIG_PATH.as_path();
    let content = fs::read_to_string(path)?;
    let processes: HashMap<String, ProcessConfig> = serde_json::from_str(&content)?;
    
    let mut global_processes = PROCESSES.write().unwrap();
    *global_processes = processes;
    
    Ok(())
}

// 保存进程配置
pub fn save_processes() -> anyhow::Result<()> {
    let processes = PROCESSES.read().unwrap();
    let content = serde_yaml::to_string(&*processes)?;
    fs::write(PROCESS_CONFIG_PATH.as_path(), content)?;
    Ok(())
}

pub fn update_processes(update_list : Vec<ProcessConfig>) -> anyhow::Result<()> {
    let mut processes = PROCESSES.write().unwrap();
    for config in update_list {
        processes.insert(config.name.clone(), config);
    }
    let content = serde_yaml::to_string(&*processes)?;
    fs::write(PROCESS_CONFIG_PATH.as_path(), content)?;
    Ok(())
}

// 获取进程配置
pub fn get_process(name: &str) -> Option<ProcessConfig> {
    let processes = PROCESSES.read().unwrap();
    processes.get(name).cloned()
}

// 添加新进程配置
pub fn add_process(name: &str, command: &str, dir: Option<&str>, env: Option<Vec<(String, String)>>) -> anyhow::Result<()> {
    let config = ProcessConfig {
        start_time: Some(std::time::SystemTime::now()),
        name: name.to_string(),
        command: command.to_string(),
        working_dir: dir.map(|d| d.to_string()),
        env: env.map(|e| e.into_iter().collect()),
        auto_restart: false,
        status: ProcessStatus::Unknown,
        pid: None,
        created_at: chrono::Local::now().to_rfc3339(),
        updated_at: chrono::Local::now().to_rfc3339()
    };
    update_process(config)
}

// 添加或更新进程配置
pub fn update_process(config: ProcessConfig) -> anyhow::Result<()> {
    let mut processes = PROCESSES.write().unwrap();
    processes.insert(config.name.clone(), config);
    drop(processes);
    save_processes()?;
    Ok(())
}


// 删除进程配置
pub fn remove_process(name: &str) -> anyhow::Result<()> {
    let mut processes = PROCESSES.write().unwrap();
    processes.remove(name);
    drop(processes);
    save_processes()?;
    Ok(())
}