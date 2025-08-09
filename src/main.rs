mod config;
mod daemon;
mod process;
mod server;
mod utils;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Service {
        action : String
    },
    /// 启动Web服务器（默认）
    Server {
        /// 监听端口
        #[arg(long, default_value = "")]
        host : String,
        #[arg(short, long, default_value_t = 0)]
        port: u16,

    },
    /// 添加进程
    Add {
        /// 进程名称
        name: String,
        /// 命令
        #[arg(short, long)]
        cmd: String,
        /// 工作目录
        #[arg(short, long)]
        dir: Option<String>,
        /// 环境变量 (格式: KEY=VALUE)
        #[arg(short, long)]
        env: Vec<String>,
        /// 自动重启
        #[arg(short, long)]
        auto_restart: bool,
    },
    Start {
        name:String
    },
    Stop{
        name: String,
    },
    /// 重启进程
    Restart {
        /// 进程名称
        name: String,
    },
    /// 列出所有进程
    List,
    /// 显示进程状态
    Status {
        /// 进程名称
        name: Option<String>,
    },
    /// 查看进程详情
    Show {
        /// 进程名称
        name: String,
    },
    /// 移除进程
    Remove {
        /// 进程名称
        name: String,
        /// 强制删除，不停止进程
        #[arg(short, long)]
        force: bool,
    },
}



#[tokio::main]
async fn main() -> Result<()> {
    // 初始化配置
    config::init()?;

    // 解析命令行参数
    let cli = Cli::parse();
    // 处理命令
    match &cli.command {
        Commands::Service { action  } => {
            if *action == "start" {
                daemon::start_daemon()?;
            }

            if *action == "stop" {
                daemon::stop_daemon()?;
            }

            if *action == "restart" {
                daemon::stop_daemon()?;
                daemon::start_daemon()?;
            }
            
        }
        Commands::Server { host ,port } => {
            // 通过 Web API 启动服务
            if !host.is_empty() {
                config::SERVER_CONF.write().unwrap().host = String::from(host);
            }
            if *port != 0 {
                config::SERVER_CONF.write().unwrap().port = *port;
            }

            server::start_server( config::SERVER_CONF.read().unwrap().host.as_str(),config::SERVER_CONF.read().unwrap().port).await?;
        }
        Commands::Add {
            name,
            cmd,
            dir,
            env,
            auto_restart,
        } => {
            crate::config::add_process(name, cmd, dir.as_deref(), Some(env.iter().map(|e| (e.clone(), e.clone())).collect()))?;
            
            // 更新自动重启设置
            if let Some(mut config) = config::get_process(name) {
                config.auto_restart = *auto_restart;
                config::update_process(config)?;
            }
            
            println!("Process {} added", name);
        }
        Commands::Start { name } => {
            utils::start_process_via_api(name).await?;            
            // println!("Process {} started", name);
        }
        Commands::Stop { name } => {
            process::stop_process(name)?;
            println!("Process {} stopped", name);
        }
        Commands::Restart { name } => {
            process::restart_process(name)?;
            println!("Process {} restarted", name);
        }
        Commands::List => {
            process::list_processes()?;
        }
        Commands::Status { name } => {
            process::show_status(name.as_deref())?;
        }
        Commands::Show { name } => {
            process::show_details(&name)?;
        }
        Commands::Remove { name, force } => {
            process::remove_process(&name, *force)?;
            println!("Process {} removed{}", name, if *force { " (force)" } else { "" });
        }
    }

    Ok(())
}