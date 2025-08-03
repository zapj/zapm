use crate::config::{self,get_process, update_process, ProcessConfig, ProcessStatus};
use anyhow::{Context, Result};
use chrono::Local;
use std::collections::HashMap;
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use std::sync::{Mutex};
use std::time::SystemTime;

use sysinfo::{ProcessExt, System, SystemExt};
use once_cell::sync::Lazy;

#[cfg(target_os = "windows")]
use winapi::um::winbase::{CREATE_NO_WINDOW};

// 运行中的进程
static RUNNING_PROCESSES: Lazy<Mutex<HashMap<String, Child>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// 启动进程
pub fn start_process(
    name: &str,
    cmd: &str,
    working_dir: Option<&str>,
    env_vars: Option<&Vec<String>>,
) -> Result<()> {
    // 检查进程是否已存在
    if let Some(config) = get_process(name) {
        let mut system = System::new_all();
        system.refresh_all();
        
        // 实时检查进程是否真正运行
        let is_running = config.pid
            .map(|pid| system.process(sysinfo::Pid::from(pid as usize)).is_some())
            .unwrap_or(false);
        
        if is_running {
            // 如果状态不一致则更新
            if config.status != ProcessStatus::Running {
                let mut updated_config = config.clone();
                updated_config.status = ProcessStatus::Running;
                updated_config.updated_at = Local::now().to_rfc3339();
                update_process(updated_config)?;
            }
            println!("Process {} is already running", name);
            return Ok(());
        } else if config.status == ProcessStatus::Running {
            // 进程不在运行但状态显示运行，修正状态
            let mut updated_config = config.clone();
            updated_config.status = ProcessStatus::Stopped;
            updated_config.pid = None;
            updated_config.updated_at = Local::now().to_rfc3339();
            update_process(updated_config)?;
        }
    }

    // 解析命令和参数
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty command"));
    }

    let program = parts[0];
    let args = &parts[1..];

    // 创建命令
    let mut command = Command::new(program);
    command.args(args);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    // 设置工作目录
    if let Some(dir) = working_dir {
        command.current_dir(dir);
    }

    // 设置环境变量
    if let Some(env_list) = env_vars {
        for env_var in env_list {
            if let Some((key, value)) = env_var.split_once('=') {
                command.env(key, value);
            }
        }
    }

    // 启动进程
    let child = command
        .spawn()
        .with_context(|| format!("Failed to start process {}", name))?;

    let pid = child.id();

    // 更新进程状态
    let now = Local::now().to_rfc3339();
    let mut env_map = None;
    if let Some(env_list) = env_vars {
        let mut map = HashMap::new();
        for env_var in env_list {
            if let Some((key, value)) = env_var.split_once('=') {
                map.insert(key.to_string(), value.to_string());
            }
        }
        env_map = Some(map);
    }

    let config = ProcessConfig {
        start_time: Some(SystemTime::now()),

        name: name.to_string(),
        command: cmd.to_string(),
        working_dir: working_dir.map(|s| s.to_string()),
        env: env_map,
        auto_restart: false,
        status: ProcessStatus::Running,
        pid: Some(pid),
        created_at: now.clone(),
        updated_at: now,
    };

    update_process(config)?;

    // 保存运行中的进程
    let mut running = RUNNING_PROCESSES.lock().unwrap();
    running.insert(name.to_string(), child);

    println!("Process {} started with PID {}", name, pid);
    Ok(())
}

// 停止进程
pub fn stop_process(name: &str) -> Result<()> {
    let mut running = RUNNING_PROCESSES.lock().unwrap();
    
    if let Some(mut child) = running.remove(name) {
        // 尝试终止进程
        match child.kill() {
            Ok(_) => {
                println!("Process {} stopped", name);
            }
            Err(e) => {
                println!("Failed to stop process {}: {}", name, e);
            }
        }
    } else {
        // 检查进程是否在配置中
        if let Some(config) = get_process(name) {
            if let Some(pid) = config.pid {
                // 尝试通过系统API终止进程
                let mut system = System::new_all();
                system.refresh_all();
                
                if let Some(process) = system.process(sysinfo::Pid::from(pid as usize)) {
                    process.kill();
                    println!("Process {} with PID {} stopped", name, pid);
                } else {
                    println!("Process {} not found in system", name);
                }
            } else {
                println!("Process {} has no PID", name);
            }
        } else {
            return Err(anyhow::anyhow!("Process {} not found", name));
        }
    }

    // 更新进程状态
    if let Some(config) = get_process(name) {
        let mut updated_config = config.clone();
        updated_config.status = ProcessStatus::Stopped;
        updated_config.pid = None;
        updated_config.updated_at = Local::now().to_rfc3339();
        update_process(updated_config)?;
    }

    Ok(())
}

// 重启进程
pub fn restart_process(name: &str) -> Result<()> {
    if let Some(config) = get_process(name) {
        let cmd = config.command.clone();
        let working_dir = config.working_dir.clone();
        let env = config.env.clone();
        
        // 将环境变量转换为Vec<String>格式
        let env_vars = env.map(|map| {
            map.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<String>>()
        });
        
        // 停止进程
        let _ = stop_process(name);
        
        // 启动进程
        start_process(
            name,
            &cmd,
            working_dir.as_deref(),
            env_vars.as_ref(),
        )?;
        
        println!("Process {} restarted", name);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Process {} not found", name))
    }
}

// 列出所有进程
pub fn list_processes() -> Result<()> {
    let processes = config::PROCESSES.try_read().unwrap();
    
    if processes.is_empty() {
        println!("No processes found");
        return Ok(());
    }
    
    println!("{:<20} {:<10} {:<10} {:<20} {:<10}", "NAME", "STATUS", "PID", "START TIME", "UPTIME");
    println!("{:-<20} {:-<10} {:-<10} {:-<20} {:-<10}", "", "", "", "", "");
    let mut update_configs:Vec<ProcessConfig> = vec![];
    for (name, config) in processes.iter() {
        let mut system = System::new_all();
        system.refresh_all();
        // println!("{:?}",config);
        let (pid_str, status_str) = match config.pid {
            Some(pid) => {
                let is_running = system.process(sysinfo::Pid::from(pid as usize)).is_some();
                let new_status = if is_running { ProcessStatus::Running } else { ProcessStatus::Stopped };
                
                // 如果状态不一致则更新配置文件
                if config.status != new_status {
                    let mut updated_config = config.clone();
                    updated_config.status = new_status;
                    updated_config.updated_at = Local::now().to_rfc3339();
                    if !is_running { updated_config.pid = None; }
                    update_configs.push(updated_config);

                }
                
                (pid.to_string(), format!("{:?}", new_status))
            },
            None => {
                // 检查无PID但状态显示运行的情况
                if config.status == ProcessStatus::Running {
                    let mut updated_config = config.clone();
                    updated_config.status = ProcessStatus::Stopped;
                    updated_config.updated_at = Local::now().to_rfc3339();
                    update_configs.push(updated_config);
                }
                ("-".to_string(), format!("{:?}", config.status))
            },
        };
        let start_time_str = match config.start_time {
            Some(time) => {
                let time: chrono::DateTime<Local> = time.into();
                time.format("%Y-%m-%d %H:%M:%S").to_string()
            },
            None => "-".to_string(),
        };
        
        let uptime_str = match config.start_time {
            Some(time) => {
                let uptime = SystemTime::now().duration_since(time);
                
                if let Ok(uptime) = uptime {
                    format!("{}s", uptime.as_secs())
                } else {
                    "-".to_string()
                }
            },
            None => "-".to_string(),
        };
        
        println!(
            "{:<20} {:<10} {:<10} {:<20} {:<10}",
            name,
            status_str,
            pid_str,
            start_time_str,
            uptime_str
        );
    }
    // let mut p = PROCESSES.try_write().expect("Failed to write to the global processes lock");
    drop(processes);
    let _ = config::update_processes(update_configs);
    Ok(())
}

// 显示进程详情
pub fn show_details(name: &str) -> Result<()> {
    if let Some(config) = get_process(name) {
        println!("Process: {}", config.name);
        println!("Command: {}", config.command);
        println!("Status: {:?}", config.status);
        
        if let Some(pid) = config.pid {
            println!("PID: {}", pid);
            
            // 检查进程是否真的在运行
            let mut system = System::new_all();
            system.refresh_all();
            
            if let Some(process) = system.process(sysinfo::Pid::from(pid as usize)) {
                println!("Memory usage: {} KB", process.memory() / 1024);
                println!("CPU usage: {:.2}%", process.cpu_usage());
                println!("Running time: {} seconds", process.run_time());
            } else {
                println!("Process not found in system (may have terminated)");
            }
        } else {
            println!("PID: -");
        }
        
        if let Some(dir) = &config.working_dir {
            println!("Working directory: {}", dir);
        }
        
        if let Some(env) = &config.env {
            println!("Environment variables:");
            for (key, value) in env {
                println!("  {}={}", key, value);
            }
        }
        
        println!("Created at: {}", config.created_at);
        println!("Updated at: {}", config.updated_at);
        
        Ok(())
    } else {
        Err(anyhow::anyhow!("Process {} not found", name))
    }
}

// 移除进程
pub fn remove_process(name: &str, force: bool) -> Result<()> {
    if !force {
        // 停止进程
        let _ = stop_process(name);
    }
    
    // 从配置中移除
    config::remove_process(name)?;
    
    // 从运行中的进程列表中移除
    let mut running = RUNNING_PROCESSES.lock().unwrap();
    running.remove(name);
    
    Ok(())
}

// 显示进程状态
pub fn show_status(name: Option<&str>) -> Result<()> {
    if let Some(name) = name {
        if let Some(config) = get_process(name) {
            println!("Process: {}", config.name);
            println!("Command: {}", config.command);
            println!("Status: {:?}", config.status);
            
            if let Some(start_time) = config.start_time {
                let start_time: chrono::DateTime<Local> = start_time.into();
                let uptime = Local::now() - start_time;
                println!("Started at: {}", start_time.format("%Y-%m-%d %H:%M:%S"));
                println!("Uptime: {} seconds", uptime.num_seconds());
            }
            
            if let Some(pid) = config.pid {
                println!("PID: {}", pid);
                
                // 检查进程是否真的在运行
                let mut system = System::new_all();
                system.refresh_all();
                
                if let Some(process) = system.process(sysinfo::Pid::from(pid as usize)) {
                    println!("Memory usage: {} KB", process.memory() / 1024);
                    println!("CPU usage: {:.2}%", process.cpu_usage());
                    println!("Running time: {} seconds", process.run_time());
                } else {
                    println!("Process not found in system (may have terminated)");
                }
            } else {
                println!("PID: -");
            }
            
            if let Some(dir) = &config.working_dir {
                println!("Working directory: {}", dir);
            }
            
            if let Some(env) = &config.env {
                println!("Environment variables:");
                for (key, value) in env {
                    println!("  {}={}", key, value);
                }
            }
            
            println!("Created at: {}", config.created_at);
            println!("Updated at: {}", config.updated_at);
        } else {
            return Err(anyhow::anyhow!("Process {} not found", name));
        }
    } else {
        // 显示所有进程状态
        list_processes()?;
    }
    
    Ok(())
}

// 检查进程状态
pub fn check_processes() -> Result<()> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let mut processes = crate::config::PROCESSES.write().unwrap();
    let mut running = RUNNING_PROCESSES.lock().unwrap();
    
    for (name, config) in processes.iter_mut() {
        if config.status == ProcessStatus::Running {
            if let Some(pid) = config.pid {
                // 检查进程是否在系统中运行
                if system.process(sysinfo::Pid::from(pid as usize)).is_none() {
                    // 进程不在运行
                    config.status = ProcessStatus::Failed;
                    config.pid = None;
                    config.updated_at = Local::now().to_rfc3339();
                    
                    // 从运行中的进程列表中移除
                    running.remove(name);
                    
                    println!("Process {} (PID {}) is not running", name, pid);
                    
                    // 如果配置了自动重启，则标记需要重启
                    let needs_restart = config.auto_restart;
                    let process_name = name.clone();
                    
                    // 如果配置了自动重启，则在释放锁后重启进程
                    if needs_restart {
                        // 在这里不直接调用restart_process，而是克隆名称后在外部处理
                        drop(processes);
                        drop(running);
                        let _ = restart_process(&process_name);
                        return Ok(());
                    }
                }
            }
        }
    }
    
    Ok(())
}

// 定期检查进程状态
pub fn start_process_monitor() {
    std::thread::spawn(|| {
        loop {
            let _ = check_processes();
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });
}