//! CC-Switch CLI 工具
//!
//! 提供终端命令行控制功能，用于无GUI环境

use cc_switch_lib::{AppError, AppType, Database, Provider};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "cc-switch-cli")]
#[command(about = "CC-Switch 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 代理服务器控制
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// 列出所有供应商
    List {
        /// 应用类型 (claude/codex/gemini)
        app_type: Option<String>,
    },
    /// 添加供应商
    Add {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
        /// 供应商名称
        #[arg(long)]
        name: String,
        /// API Key
        #[arg(long)]
        api_key: String,
        /// Base URL
        #[arg(long)]
        base_url: String,
        /// 优先级层级 (默认: 0)
        #[arg(long, default_value = "0")]
        priority: usize,
    },
    /// 删除供应商
    Remove {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 启用供应商（设置为当前）
    Enable {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 查看当前供应商
    Current {
        /// 应用类型 (claude/codex/gemini)
        app_type: Option<String>,
    },
    /// 设置供应商优先级层级
    SetPriority {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
        /// 优先级层级 (0为最高优先级，数字越大优先级越低)
        priority: usize,
    },
    /// 添加供应商到故障转移队列
    AddToQueue {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 从故障转移队列移除供应商
    RemoveFromQueue {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ProxyAction {
    /// 启动代理服务器(前台模式)
    Start,
    /// 停止代理服务器
    Stop,
    /// 重启代理服务器
    Restart,
    /// 查看代理服务器状态
    Status,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Proxy { action } => handle_proxy(action).await,
        Commands::List { app_type } => handle_list(app_type),
        Commands::Add {
            app_type,
            id,
            name,
            api_key,
            base_url,
            priority,
        } => handle_add(&app_type, &id, &name, &api_key, &base_url, priority),
        Commands::Remove { app_type, id } => handle_remove(&app_type, &id),
        Commands::Enable { app_type, id } => handle_enable(&app_type, &id),
        Commands::Current { app_type } => handle_current(app_type),
        Commands::SetPriority {
            app_type,
            id,
            priority,
        } => handle_set_priority(&app_type, &id, priority),
        Commands::AddToQueue { app_type, id } => handle_add_to_queue(&app_type, &id),
        Commands::RemoveFromQueue { app_type, id } => handle_remove_from_queue(&app_type, &id),
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}

// ============================================================================
// 代理服务器控制
// ============================================================================

async fn handle_proxy(action: ProxyAction) -> Result<(), AppError> {
    match action {
        ProxyAction::Start => proxy_start().await,
        ProxyAction::Stop => proxy_stop().await,
        ProxyAction::Restart => {
            let _ = proxy_stop().await; // 忽略停止错误
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            proxy_start().await
        }
        ProxyAction::Status => proxy_status().await,
    }
}

async fn proxy_start() -> Result<(), AppError> {
    use cc_switch_lib::proxy::{ProxyConfig, ProxyServer};

    println!("正在启动代理服务器（前台模式）...");
    println!("按 Ctrl+C 停止\n");

    // 初始化数据库
    let db = Arc::new(Database::init()?);

    // 创建代理配置
    let config = ProxyConfig::default();

    // 创建代理服务器（不传入AppHandle，CLI模式下不需要GUI事件）
    let server = ProxyServer::new(config.clone(), db, None);

    // 启动服务器
    server.start().await
        .map_err(|e| AppError::Message(format!("启动服务器失败: {}", e)))?;

    println!("✓ 代理服务器已启动");
    println!("  地址: {}:{}", config.listen_address, config.listen_port);
    println!("  启动时间: {}\n", chrono::Utc::now().to_rfc3339());

    // 保存PID
    let pid_file = get_config_dir().join("proxy.pid");
    std::fs::write(&pid_file, std::process::id().to_string())
        .map_err(|e| AppError::Message(format!("写入PID文件失败: {}", e)))?;

    // 等待Ctrl+C信号
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            println!("\n正在停止...");
            server.stop().await
                .map_err(|e| AppError::Message(format!("停止服务器失败: {}", e)))?;
            std::fs::remove_file(&pid_file).ok();
            println!("✓ 代理服务器已停止");
            Ok(())
        }
        Err(e) => Err(AppError::Message(format!("信号处理失败: {}", e))),
    }
}

async fn proxy_stop() -> Result<(), AppError> {
    // 读取 PID 文件
    let pid_file = get_config_dir().join("proxy.pid");

    if !pid_file.exists() {
        return Err(AppError::Message("代理服务器未运行（PID文件不存在）".to_string()));
    }

    let pid_str = std::fs::read_to_string(&pid_file)
        .map_err(|e| AppError::Message(format!("读取PID文件失败: {}", e)))?;
    let pid: i32 = pid_str.trim().parse()
        .map_err(|e| AppError::Message(format!("解析PID失败: {}", e)))?;

    // 发送 SIGTERM 信号
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid), Signal::SIGTERM)
            .map_err(|e| AppError::Message(format!("停止进程失败: {}", e)))?;
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(&["/PID", &pid.to_string(), "/F"])
            .output()
            .map_err(|e| AppError::Message(format!("停止进程失败: {}", e)))?;
    }

    // 删除 PID 文件
    std::fs::remove_file(&pid_file).ok();

    println!("✓ 代理服务器已停止");
    Ok(())
}

async fn proxy_status() -> Result<(), AppError> {
    let pid_file = get_config_dir().join("proxy.pid");

    if !pid_file.exists() {
        println!("代理服务器状态: 未运行");
        return Ok(());
    }

    let pid_str = std::fs::read_to_string(&pid_file)
        .map_err(|e| AppError::Message(format!("读取PID文件失败: {}", e)))?;
    let pid: i32 = pid_str.trim().parse()
        .map_err(|e| AppError::Message(format!("解析PID失败: {}", e)))?;

    // 检查进程是否存在
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        match kill(Pid::from_raw(pid), None) {
            Ok(()) => {
                println!("代理服务器状态: 运行中");
                println!("  PID: {}", pid);
            }
            Err(_) => {
                println!("代理服务器状态: 未运行（PID {} 不存在）", pid);
                std::fs::remove_file(&pid_file).ok();
            }
        }
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .map_err(|e| AppError::Message(format!("检查进程失败: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        if output_str.contains(&pid.to_string()) {
            println!("代理服务器状态: 运行中");
            println!("  PID: {}", pid);
        } else {
            println!("代理服务器状态: 未运行（PID {} 不存在）", pid);
            std::fs::remove_file(&pid_file).ok();
        }
    }

    Ok(())
}

// ============================================================================
// 供应商管理
// ============================================================================

fn handle_list(app_type: Option<String>) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);

    let app_types = match app_type {
        Some(t) => vec![parse_app_type(&t)?],
        None => vec!["claude".to_string(), "codex".to_string(), "gemini".to_string()],
    };

    for app_type_str in app_types {
        println!("\n=== {} 供应商 ===", app_type_str);

        let providers = db.get_all_providers(&app_type_str)?;
        let current_id = db.get_current_provider(&app_type_str)?;

        if providers.is_empty() {
            println!("  (无供应商)");
            continue;
        }

        for (_, provider) in providers {
            let is_current = current_id.as_ref().map(|id| id == &provider.id).unwrap_or(false);
            let marker = if is_current { "  [当前]" } else { "" };
            let in_queue = if provider.in_failover_queue { " [队列]" } else { "" };
            let priority = provider.sort_index.map(|p| format!(" [层级:{}]", p)).unwrap_or_default();

            println!("  {} - {}{}{}{}",
                provider.id,
                provider.name,
                priority,
                in_queue,
                marker
            );
        }
    }

    Ok(())
}

fn handle_add(
    app_type: &str,
    id: &str,
    name: &str,
    api_key: &str,
    base_url: &str,
    priority: usize,
) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    // 构建 settings_config
    let settings_config = json!({
        "env": {
            "ANTHROPIC_BASE_URL": base_url,
            "ANTHROPIC_API_KEY": api_key,
        }
    });

    let provider = Provider {
        id: id.to_string(),
        name: name.to_string(),
        settings_config,
        website_url: None,
        category: None,
        created_at: None,
        sort_index: Some(priority),
        notes: None,
        meta: None,
        icon: None,
        icon_color: None,
        in_failover_queue: false,
    };

    db.save_provider(&app_type_str, &provider)?;
    println!("✓ 已添加供应商: {} ({})", name, id);
    println!("  优先级层级: {}", priority);

    Ok(())
}

fn handle_remove(app_type: &str, id: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    db.delete_provider(&app_type_str, id)?;
    println!("✓ 已删除供应商: {}", id);

    Ok(())
}

fn handle_enable(app_type: &str, id: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    db.set_current_provider(&app_type_str, id)?;
    println!("✓ 已启用供应商: {}", id);

    Ok(())
}

fn handle_current(app_type: Option<String>) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);

    let app_types = match app_type {
        Some(t) => vec![parse_app_type(&t)?],
        None => vec!["claude".to_string(), "codex".to_string(), "gemini".to_string()],
    };

    for app_type_str in app_types {
        println!("\n=== {} ===", app_type_str);

        match db.get_current_provider(&app_type_str)? {
            Some(provider_id) => {
                // 获取完整的 Provider 对象
                match db.get_provider_by_id(&provider_id, &app_type_str)? {
                    Some(provider) => {
                        println!("  当前供应商: {} ({})", provider.name, provider.id);
                        if let Some(priority) = provider.sort_index {
                            println!("  优先级层级: {}", priority);
                        }
                    }
                    None => {
                        println!("  (供应商 {} 不存在)", provider_id);
                    }
                }
            }
            None => {
                println!("  (未设置当前供应商)");
            }
        }
    }

    Ok(())
}

fn handle_set_priority(app_type: &str, id: &str, priority: usize) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    // 获取现有供应商
    let mut provider = db.get_provider_by_id(id, &app_type_str)?
        .ok_or_else(|| AppError::Message(format!("供应商不存在: {}", id)))?;

    // 更新优先级
    provider.sort_index = Some(priority);

    // 保存
    db.save_provider(&app_type_str, &provider)?;
    println!("✓ 已设置供应商 {} 的优先级层级为: {}", id, priority);
    println!("  说明: 层级0优先级最高，只有层级0的所有供应商都失败后才会尝试层级1");

    Ok(())
}

fn handle_add_to_queue(app_type: &str, id: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    db.add_to_failover_queue(&app_type_str, id)?;
    println!("✓ 已将供应商 {} 添加到故障转移队列", id);

    Ok(())
}

fn handle_remove_from_queue(app_type: &str, id: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    db.remove_from_failover_queue(&app_type_str, id)?;
    println!("✓ 已将供应商 {} 从故障转移队列移除", id);

    Ok(())
}

// ============================================================================
// 辅助函数
// ============================================================================

fn parse_app_type(s: &str) -> Result<String, AppError> {
    let normalized = s.to_lowercase();
    match normalized.as_str() {
        "claude" | "codex" | "gemini" => Ok(normalized),
        _ => Err(AppError::Message(format!(
            "无效的应用类型: {}，支持: claude, codex, gemini",
            s
        ))),
    }
}

fn get_config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cc-switch")
}
