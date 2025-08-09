use std::path::Path;
use std::{env, fs};
use anyhow::Result;
use reqwest;

use crate::config;


// 确保目录存在
pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

// 格式化时间戳
pub fn format_timestamp(timestamp: u64) -> String {
    let dt = chrono::DateTime::from_timestamp(timestamp as i64, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

// 格式化内存大小
pub fn format_memory_size(size_in_kb: u64) -> String {
    if size_in_kb < 1024 {
        return format!("{} KB", size_in_kb);
    } else if size_in_kb < 1024 * 1024 {
        return format!("{:.2} MB", size_in_kb as f64 / 1024.0);
    } else {
        return format!("{:.2} GB", size_in_kb as f64 / (1024.0 * 1024.0));
    }
}

// 格式化运行时间
pub fn format_uptime(seconds: u64) -> String {
    let days = seconds / (24 * 3600);
    let hours = (seconds % (24 * 3600)) / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

// 通过 Web API 启动服务
pub async fn start_process_via_api(name : &str) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes/{}/start",api_base_url, name);
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({}))
        .send()
        .await?;   
    if response.status() == 200 {
        println!("Process {} started , {}", name,response.text().await?);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }

}

pub async fn stop_process_via_api(name: &str) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes/{}/stop",api_base_url, name);
    let response = reqwest::Client::new()
        .post(&url)
        .send()
        .await?;
    if response.status() == 200 {
        println!("Process {} stopped", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }
}


pub async fn restart_process_via_api(name: &str) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes/{}/restart",api_base_url, name);
    let response = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({}))
        .send()
        .await?;
    if response.status() == 200 {
        println!("Process {} restarted", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }
}

pub async fn delete_process_via_api(name: &str) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes/{}", api_base_url,name);
    let response = reqwest::Client::new()
        .delete(&url)
        .json(&serde_json::json!({}))
        .send()
        .await?;
    if response.status() == 200 {
        println!("Process {} removed", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }
}   

pub async fn update_process_via_api(name: &str, command: &str, dir: &str, env: &[(String, String)]) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes/{}", api_base_url, name);
    let response = reqwest::Client::new()
        .put(&url)
        .json(&serde_json::json!({
            "command": command,
            "dir": dir,
            "env": env
        }))
        .send()
        .await?;
    if response.status() == 200 {
        println!("Process {} updated", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }
}   


pub async fn add_process_via_api(name: &str, command: &str, dir: &str, env: &[(String, String)]) -> Result<()> {
    let api_base_url = config::SERVER_CONF.read().unwrap().api_base_url.to_string();
    let url = format!("{}/api/processes", api_base_url);
    let response = reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({
            "name": name,
            "command": command,
            "dir": dir,
            "env": env
        }))
        .send()
        .await?;
    
    if response.status() == 200 {
        Ok(())
    } else {
        Err(anyhow::anyhow!(response.text().await?))
    }
}      

pub fn install_service() -> Result<()> {
    use std::process::Command;
    
    if cfg!(target_os = "windows") {
        // let current_dir = std::env::current_dir()?;
        let execute_path = format!("{}",env::current_exe().unwrap().to_str().unwrap()); 
        Command::new("sc.exe")
            .arg("create")
            .arg("zapm")
            .arg("binPath=")
            .arg(execute_path)
            .arg("service")
            .arg("type= own")
            .arg("start= auto")
            .arg("displayname= zapm-service")
            .output()
            .map_err(|_| anyhow::anyhow!("无法创建服务，运行 sc 创建服务试试"))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("无法创建服务，返回错误代码 {:?}", output.status))
                }
            })?;
        Command::new("sc.exe")
            .arg("start")
            .arg("zapm")
            .output()
            .map_err(|_| anyhow::anyhow!("无法启动服务，运行 sc start zapm 来手动启动"))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("无法启动服务，返回错误代码 {:?}", output.status))
                }
            })?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("无法在您的操作系统中创建服务，请使用 systemd 或其他方法手动安装"))
    }
} 


pub fn uninstall_service() -> Result<()> {
    use std::process::Command;
    
    if cfg!(target_os = "windows") {
        Command::new("sc.exe")
            .arg("stop")
            .arg("zapm")
            .output()
            .map_err(|_| anyhow::anyhow!("无法停止服务，使用 sc stop zapm 手动停止"))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("无法停止服务，返回错误代码 {:?}", output.status))
                }
            })?;
        Command::new("sc.exe")
            .arg("delete")
            .arg("zapm")
            .output()
            .map_err(|_| anyhow::anyhow!("无法删除服务，运行 sc delete zapm 手动删除"))
            .and_then(|output| {
                if output.status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("无法删除服务，返回错误代码 {:?}", output.status))
                }
            })?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("无法在您的操作系统中删除服务，请使用 systemd 或其他方法手动卸载"))
    }
}   