//! CC-Switch CLI 工具
//!
//! 提供终端命令行控制功能，用于无GUI环境

use cc_switch_lib::{AppError, Database, Provider};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "csc")]
#[command(about = "CC-Switch 命令行工具", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 代理服务器控制 (别名: p)
    #[command(alias = "p")]
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// 列出所有供应商 (别名: ls)
    #[command(alias = "ls")]
    List {
        /// 应用类型 (claude/codex/gemini)
        app_type: Option<String>,
    },
    /// 添加供应商 (别名: a)
    #[command(alias = "a")]
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
    /// 删除供应商 (别名: rm)
    #[command(alias = "rm")]
    Remove {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 启用供应商（设置为当前） (别名: en)
    #[command(alias = "en")]
    Enable {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 取消当前指定的供应商（回到层级轮询） (别名: dis)
    #[command(alias = "dis")]
    Disable {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
    },
    /// 查看当前供应商 (别名: c)
    #[command(alias = "c")]
    Current {
        /// 应用类型 (claude/codex/gemini)
        app_type: Option<String>,
    },
    /// 设置供应商优先级层级 (别名: sp)
    #[command(alias = "sp")]
    SetPriority {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
        /// 优先级层级 (0为最高优先级，数字越大优先级越低)
        priority: usize,
    },
    /// 添加供应商到故障转移队列 (别名: qa)
    #[command(alias = "qa")]
    AddToQueue {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 从故障转移队列移除供应商 (别名: qr)
    #[command(alias = "qr")]
    RemoveFromQueue {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID
        id: String,
    },
    /// 测试供应商URL延迟 (别名: t)
    #[command(alias = "t")]
    TestLatency {
        /// 应用类型 (claude/codex/gemini)
        app_type: String,
        /// 供应商ID（可选，不指定则测试该类型所有供应商）
        id: Option<String>,
        /// 测速模式：pure=不启动claude（默认）；startup=启动claude触发真实启动链路（保底）
        #[arg(long, default_value = "startup")]
        mode: String,
    },
    /// 导出配置到 SQL 文件 (别名: ex)
    #[command(alias = "ex")]
    Export {
        /// 导出文件路径
        file_path: String,
    },
    /// 从 SQL 文件导入配置 (别名: im)
    #[command(alias = "im")]
    Import {
        /// 导入文件路径
        file_path: String,
    },
}

#[derive(Subcommand)]
enum ProxyAction {
    /// 启动代理服务器(前台模式) (别名: s)
    #[command(alias = "s")]
    Start,
    /// 停止代理服务器 (别名: x)
    #[command(alias = "x")]
    Stop,
    /// 重启代理服务器 (别名: r)
    #[command(alias = "r")]
    Restart,
    /// 查看代理服务器状态 (别名: st)
    #[command(alias = "st")]
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
        Commands::Disable { app_type } => handle_disable(&app_type),
        Commands::Current { app_type } => handle_current(app_type),
        Commands::SetPriority {
            app_type,
            id,
            priority,
        } => handle_set_priority(&app_type, &id, priority),
        Commands::AddToQueue { app_type, id } => handle_add_to_queue(&app_type, &id),
        Commands::RemoveFromQueue { app_type, id } => handle_remove_from_queue(&app_type, &id),
        Commands::TestLatency { app_type, id, mode } => handle_test_latency(&app_type, id, &mode).await,
        Commands::Export { file_path } => handle_export(&file_path),
        Commands::Import { file_path } => handle_import(&file_path),
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

    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

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
    println!("  日志级别: INFO");
    println!("  查看实时日志: tail -f ~/.cc-switch/logs/rust_proxy.log\n");

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
        use nix::sys::signal::kill;
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

            // Debug: 输出settingsConfig
            if std::env::var("DEBUG_CONFIG").is_ok() {
                println!("    settingsConfig: {}", serde_json::to_string_pretty(&provider.settings_config).unwrap_or_default());
            }
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

    // 构建 settings_config - 根据app_type使用不同的字段名
    let settings_config = match app_type {
        "codex" => json!({
            "env": {
                "OPENAI_API_KEY": api_key,
            },
            "base_url": base_url,
        }),
        "gemini" => json!({
            "apiKey": api_key,
            "baseUrl": base_url,
        }),
        _ => json!({
            "env": {
                "ANTHROPIC_API_KEY": api_key,
                "ANTHROPIC_BASE_URL": base_url,
            }
        }),
    };

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
    println!("\n提示: 该供应商将被优先使用（优先于故障转移队列）");
    println!("      如需应用更改，请重启代理服务器: csc p r");

    Ok(())
}

fn handle_disable(app_type: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    db.clear_current_provider(&app_type_str)?;
    println!("✓ 已取消 {} 的当前指定供应商", app_type_str);
    println!("\n提示: 系统将自动使用故障转移队列中最优先层级的供应商");
    println!("      如需应用更改，请重启代理服务器: csc p r");

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

async fn handle_test_latency(app_type: &str, id: Option<String>, mode: &str) -> Result<(), AppError> {
    use cc_switch_lib::proxy::provider_router::ProviderRouter;

    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    let mode = mode.trim().to_lowercase();
    if mode != "pure" && mode != "startup" {
        return Err(AppError::Message(format!(
            "无效mode: {}，支持: pure/startup",
            mode
        )));
    }

    if mode == "startup" {
        use serde::Deserialize;
        use std::collections::BTreeMap;
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::Duration;

        #[derive(Debug, Clone)]
        struct Target {
            priority: usize,
            supplier: String,
        }

        #[derive(Debug, Deserialize)]
        struct TestOverrideStartResponse {
            ok: bool,
        }

        #[derive(Debug, Deserialize)]
        struct TestOverrideResultResponse {
            ready: bool,
            result: Option<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult>,
        }

        // 1) 发现代理端口：优先使用默认端口/常用端口探测，其次使用数据库端口记录（历史可能不一致）
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| AppError::Message(format!("创建HTTP客户端失败: {e}")))?;

        async fn find_running_proxy_base(
            db: &Database,
            client: &reqwest::Client,
        ) -> Result<String, AppError> {
            let mut ports: Vec<u16> = Vec::new();

            // 真实运行默认端口（当前安装脚本与运行日志均以此为主）
            ports.push(15721);

            // 兼容历史配置/数据库记录
            if let Ok(cfg) = db.get_proxy_config().await {
                ports.push(cfg.listen_port);
            }

            // 常见端口兜底
            ports.push(5000);
            ports.push(8080);

            // 去重保序
            let mut seen = std::collections::HashMap::<u16, ()>::new();
            ports.retain(|p| seen.insert(*p, ()).is_none());

            for port in ports {
                let base = format!("http://127.0.0.1:{port}");
                if let Ok(resp) = client.get(format!("{base}/health")).send().await {
                    if resp.status().is_success() {
                        return Ok(base);
                    }
                }
            }

            Err(AppError::Message(
                "代理服务未运行或不可达，请先启动 cc-switch 代理（默认端口 15721）".to_string(),
            ))
        }

        let base = find_running_proxy_base(db.as_ref(), &client).await?;
        println!("✓ 已检测到代理服务: {base}");

        // 3) 生成需要测试的 supplier 列表（按层级聚合）
        let mut targets: Vec<Target> = Vec::new();
        if let Some(provider_id) = id.as_deref() {
            let all = db
                .get_all_providers(&app_type_str)
                .map_err(|e| AppError::Message(format!("读取供应商失败: {e}")))?;
            let Some(p) = all.get(provider_id) else {
                return Err(AppError::Message(format!("供应商不存在: {}", provider_id)));
            };
            let supplier = p
                .name
                .split('-')
                .next()
                .unwrap_or(&p.name)
                .to_string();
            let priority = p.sort_index.unwrap_or(999999) as usize;
            targets.push(Target { priority, supplier });
        } else {
            let providers = db.get_failover_providers(&app_type_str)?;
            if providers.is_empty() {
                return Err(AppError::Message("没有可测试的供应商（队列为空）".to_string()));
            }

            let mut grouped: BTreeMap<usize, BTreeMap<String, ()>> = BTreeMap::new();
            for p in providers {
                let priority = p.sort_index.unwrap_or(999999) as usize;
                let supplier = p
                    .name
                    .split('-')
                    .next()
                    .unwrap_or(&p.name)
                    .to_string();
                grouped.entry(priority).or_default().insert(supplier, ());
            }

            for (priority, suppliers) in grouped.into_iter() {
                for supplier in suppliers.keys() {
                    targets.push(Target {
                        priority,
                        supplier: supplier.clone(),
                    });
                }
            }
        }

        if targets.is_empty() {
            return Err(AppError::Message("没有可测试的供应商".to_string()));
        }

        // 4) 启动/退出方式逐个测试（每个 supplier 一次启动 Claude Code）
        let test_model = match app_type_str.as_str() {
            "claude" => "claude-sonnet-4-5-20250929",
            "codex" => "gpt-5.2",
            "gemini" => "gemini-2.0-flash",
            _ => "unknown",
        };

        println!(
            "\n开始URL延迟测试（启动/退出真实链路），共{}个供应商（按层级与supplier聚合），model={}\n",
            targets.len(),
            test_model
        );

        let verbose_child = std::env::var("CC_SWITCH_STARTUP_TEST_VERBOSE")
            .ok()
            .as_deref()
            == Some("1");

        let workdir = "/HUBU-AI008/AroenLan/Projects";

        let timeout_secs: u64 = std::env::var("CC_SWITCH_STARTUP_TEST_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(60)
            .clamp(15, 300);
        let override_ttl_secs: u64 = std::env::var("CC_SWITCH_TEST_OVERRIDE_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(timeout_secs.saturating_add(10))
            .clamp(timeout_secs, 360);

        let mut summary: Vec<(usize, String, String, String, u64)> = Vec::new();

        for (idx, t) in targets.iter().enumerate() {
            let run_id = uuid::Uuid::new_v4().to_string();
            print!(
                "[{}/{}] 测试 层级={} supplier={} ... ",
                idx + 1,
                targets.len(),
                t.priority,
                t.supplier
            );
            std::io::stdout().flush().ok();

            // 设置测试覆盖：让下一次请求强制走该 supplier，并触发该 supplier 的 URL 测速/选用
            let start_resp = client
                .post(format!("{base}/__cc_switch/test_override/start"))
                .json(&serde_json::json!({
                    "app_type": app_type_str.as_str(),
                    "priority": t.priority,
                    "supplier": t.supplier,
                    "run_id": run_id,
                    "ttl_secs": override_ttl_secs
                }))
                .send()
                .await;

            let ok = match start_resp {
                Ok(r) => r.json::<TestOverrideStartResponse>().await.ok().map(|v| v.ok) == Some(true),
                Err(_) => false,
            };

            if !ok {
                println!("FAIL (无法设置测试覆盖)");
                continue;
            }

            // 启动 claude（不提问；依赖其启动阶段自然产生的真实请求触发测速）
            let mut cmd = Command::new("claude");
            cmd.current_dir(workdir);
            cmd.stdin(Stdio::inherit());
            if verbose_child {
                cmd.stdout(Stdio::inherit());
                cmd.stderr(Stdio::inherit());
            } else {
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());
            }

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    println!("FAIL (无法启动 claude: {e})");
                    continue;
                }
            };

            // 轮询等待结果
            let timeout = Duration::from_secs(timeout_secs);
            let poll = Duration::from_millis(350);
            let start = std::time::Instant::now();
            let mut got: Option<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult> = None;

            loop {
                if start.elapsed() > timeout {
                    break;
                }

                if let Ok(Some(_status)) = child.try_wait() {
                    // claude 提前退出：可能是启动失败/无TTY/其它原因
                    break;
                }

                if let Ok(resp) = client
                    .get(format!(
                        "{base}/__cc_switch/test_override/result/{}",
                        run_id
                    ))
                    .send()
                    .await
                {
                    if let Ok(v) = resp.json::<TestOverrideResultResponse>().await {
                        if v.ready {
                            got = v.result;
                            break;
                        }
                    }
                }

                tokio::time::sleep(poll).await;
            }

            // 拿到结果或超时：结束 claude 进程，避免进入交互界面
            let _ = child.kill();
            let _ = child.wait();

            if let Some(r) = got {
                let chosen_url = r.chosen_url.clone().unwrap_or_else(|| "-".to_string());
                let metric = r.metric_ms.unwrap_or(u64::MAX);
                println!("{} {} {}", r.chosen_kind, chosen_url, if metric == u64::MAX { "-".to_string() } else { format!("{}ms", metric) });
                if metric != u64::MAX && chosen_url != "-" {
                    summary.push((r.priority, r.supplier.clone(), chosen_url, r.chosen_kind.clone(), metric));
                }
            } else {
                println!("FAIL (等待测速结果超时/未触发真实请求)");
            }
        }

        if summary.is_empty() {
            println!("\n=== 汇总 ===");
            println!("全部测试失败（没有任何可用URL）");
            return Ok(());
        }

        summary.sort_by_key(|(_, _, _, _, ms)| *ms);
        println!("\n=== 汇总（按最优URL延迟排序）===");
        for (i, (priority, supplier, url, kind, metric_ms)) in summary.iter().enumerate() {
            match kind.as_str() {
                "OK" => println!(
                    "{}. [层级 {}] {} -> {} - OK {}ms",
                    i + 1,
                    priority,
                    supplier,
                    url,
                    metric_ms
                ),
                "OV" => println!(
                    "{}. [层级 {}] {} -> {} - OV ~{}ms",
                    i + 1,
                    priority,
                    supplier,
                    url,
                    metric_ms
                ),
                "FB" => println!(
                    "{}. [层级 {}] {} -> {} - FB ~{}ms",
                    i + 1,
                    priority,
                    supplier,
                    url,
                    metric_ms
                ),
                _ => println!(
                    "{}. [层级 {}] {} -> {} - {}",
                    i + 1,
                    priority,
                    supplier,
                    url,
                    kind
                ),
            }
        }

        return Ok(());
    }

    let test_model = match app_type_str.as_str() {
        "claude" => "claude-sonnet-4-5-20250929",
        "codex" => "gpt-5.2",
        "gemini" => "gemini-2.0-flash",
        _ => "unknown",
    };

    let (only_priority, only_supplier) = if let Some(provider_id) = id.as_deref() {
        let all = db
            .get_all_providers(&app_type_str)
            .map_err(|e| AppError::Message(format!("读取供应商失败: {e}")))?;
        let Some(p) = all.get(provider_id) else {
            return Err(AppError::Message(format!("供应商不存在: {}", provider_id)));
        };
        let supplier = p
            .name
            .split('-')
            .next()
            .unwrap_or(&p.name)
            .to_string();
        (p.sort_index.map(|v| v as usize), Some(supplier))
    } else {
        (None, None)
    };

    let router = ProviderRouter::new(db);

    let results = router
        .benchmark_all_suppliers(
            app_type_str.as_str(),
            test_model,
            only_priority,
            only_supplier.as_deref(),
        )
        .await
        .map_err(|e| AppError::Message(format!("测速失败: {e}")))?;

    if results.is_empty() {
        println!("\n=== 汇总 ===");
        println!("全部测试失败（没有任何可用URL）");
        return Ok(());
    }

    println!(
        "\n开始URL延迟测试（带回退），共{}个供应商，按层级与supplier聚合\n",
        results.len()
    );

    let mut by_priority: std::collections::BTreeMap<usize, Vec<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult>> =
        std::collections::BTreeMap::new();
    for r in results.into_iter() {
        by_priority.entry(r.priority).or_default().push(r);
    }

    let mut summary: Vec<(usize, String, String, String, u64)> = Vec::new();

    for (priority, mut list) in by_priority.into_iter() {
        list.sort_by(|a, b| a.supplier.cmp(&b.supplier));
        println!("=== 层级 {} ===", priority);

        for s in list.into_iter() {
            println!("\n供应商: {}", s.supplier);
            for (i, u) in s.urls.iter().enumerate() {
                match u.kind.as_str() {
                    "OK" => println!(
                        "  {}. {} - OK {}ms",
                        i + 1,
                        u.url,
                        u.latency_ms.unwrap_or(0)
                    ),
                    "OV" => println!(
                        "  {}. {} - OV {}ms ({})",
                        i + 1,
                        u.url,
                        u.latency_ms.unwrap_or(0),
                        u.message.as_deref().unwrap_or("-")
                    ),
                    "FB" => println!(
                        "  {}. {} - FB {}ms (+{}ms)",
                        i + 1,
                        u.url,
                        u.latency_ms.unwrap_or(0),
                        u.penalty_ms.unwrap_or(0)
                    ),
                    "FAIL" => println!(
                        "  {}. {} - FAIL ({})",
                        i + 1,
                        u.url,
                        u.reason.as_deref().unwrap_or("-")
                    ),
                    _ => println!("  {}. {} - {}", i + 1, u.url, u.kind),
                }
            }

            if let (Some(url), Some(metric)) = (s.chosen_url.clone(), s.metric_ms) {
                summary.push((priority, s.supplier.clone(), url, s.chosen_kind.clone(), metric));
            }
        }
    }

    if summary.is_empty() {
        println!("\n=== 汇总 ===");
        println!("全部测试失败（没有任何可用URL）");
        return Ok(());
    }

    summary.sort_by_key(|(_, _, _, _, latency)| *latency);

    println!("\n=== 汇总（按最优URL延迟排序）===");
    for (i, (priority, supplier, url, kind, metric_ms)) in summary.iter().enumerate() {
        match kind.as_str() {
            "OK" => println!(
                "{}. [层级 {}] {} -> {} - OK {}ms",
                i + 1,
                priority,
                supplier,
                url,
                metric_ms
            ),
            "OV" => println!(
                "{}. [层级 {}] {} -> {} - OV ~{}ms",
                i + 1,
                priority,
                supplier,
                url,
                metric_ms
            ),
            "FB" => println!(
                "{}. [层级 {}] {} -> {} - FB ~{}ms",
                i + 1,
                priority,
                supplier,
                url,
                metric_ms
            ),
            _ => println!(
                "{}. [层级 {}] {} -> {} - {}",
                i + 1,
                priority,
                supplier,
                url,
                kind
            ),
        };
    }

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

// ============================================================================
// 配置导出/导入
// ============================================================================

fn handle_export(file_path: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let target_path = PathBuf::from(file_path);

    db.export_sql(&target_path)?;
    println!("✓ 配置已成功导出到: {}", file_path);

    Ok(())
}

fn handle_import(file_path: &str) -> Result<(), AppError> {
    let db = Arc::new(Database::init()?);
    let source_path = PathBuf::from(file_path);

    if !source_path.exists() {
        return Err(AppError::Message(format!("文件不存在: {}", file_path)));
    }

    println!("正在导入配置，导入前会自动备份现有配置...");
    let backup_id = db.import_sql(&source_path)?;

    if !backup_id.is_empty() {
        println!("✓ 现有配置已备份: {}", backup_id);
    }
    println!("✓ 配置已成功从文件导入: {}", file_path);
    println!("\n提示: 如需应用导入的配置，请重启代理服务器");

    Ok(())
}
