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
        Commands::TestLatency { app_type, id } => handle_test_latency(&app_type, id).await,
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

async fn handle_test_latency(app_type: &str, id: Option<String>) -> Result<(), AppError> {
    use cc_switch_lib::proxy::provider_router::ProviderRouter;
    use std::collections::{BTreeMap, HashMap};

    let db = Arc::new(Database::init()?);
    let app_type_str = parse_app_type(app_type)?;

    // 获取要测试的供应商列表
    let providers = if let Some(provider_id) = id {
        // 测试单个供应商
        let all_providers = db.get_all_providers(&app_type_str)?;
        let provider = all_providers.get(&provider_id)
            .ok_or_else(|| AppError::Message(format!("供应商不存在: {}", provider_id)))?
            .clone();
        vec![provider]
    } else {
        // 测试所有在故障转移队列中的供应商
        db.get_failover_providers(&app_type_str)?
    };

    if providers.is_empty() {
        println!("没有可测试的供应商");
        return Ok(());
    }

    fn supplier_name(provider: &Provider) -> String {
        provider
            .name
            .split('-')
            .next()
            .unwrap_or(&provider.name)
            .to_string()
    }

    fn extract_base_url(provider: &Provider, app_type: &str) -> Option<String> {
        match app_type {
            "claude" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "gemini" => provider
                .settings_config
                .get("env")
                .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            "codex" => provider
                .settings_config
                .get("base_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        }
    }

    let test_model = match app_type_str.as_str() {
        "claude" => "claude-sonnet-4-5-20250929",
        "codex" => "gpt-5.2",
        "gemini" => "gemini-2.0-flash",
        _ => "unknown",
    };

    let total_provider_count = providers.len();

    // 按层级(sort_index)分组，再按 supplier -> url 分组，以复用当前运行中的“支持回退”的测速/缓存策略
    let mut priority_groups: BTreeMap<usize, Vec<Provider>> = BTreeMap::new();
    for provider in providers.into_iter() {
        let priority = provider.sort_index.unwrap_or(999999);
        priority_groups.entry(priority).or_default().push(provider);
    }

    let router = ProviderRouter::new(db);

    println!(
        "\n开始URL延迟测试（带回退），共{}个供应商，按层级与supplier聚合\n",
        total_provider_count
    );

    const CONNECTIVITY_PENALTY_MS: u64 = 30_000;
    const OVERLOADED_PENALTY_MS: u64 = 30_000;

    fn parse_url_priority_from_provider(provider: &Provider) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();

        let from_root = provider
            .settings_config
            .get("baseUrlPriority")
            .or_else(|| provider.settings_config.get("base_url_priority"));

        if let Some(v) = from_root {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let s = s.trim();
                        if !s.is_empty() {
                            out.push(s.to_string());
                        }
                    }
                }
            } else if let Some(s) = v.as_str() {
                for part in s.split(',') {
                    let p = part.trim();
                    if !p.is_empty() {
                        out.push(p.to_string());
                    }
                }
            }
        }

        if let Some(s) = provider
            .settings_config
            .get("env")
            .and_then(|env| env.get("BASE_URL_PRIORITY"))
            .and_then(|v| v.as_str())
        {
            for part in s.split(',') {
                let p = part.trim();
                if !p.is_empty() {
                    out.push(p.to_string());
                }
            }
        }

        // 去重保序
        let mut seen = std::collections::HashMap::<String, ()>::new();
        out.retain(|u| seen.insert(u.to_string(), ()).is_none());
        out
    }

    fn default_url_priority_for_supplier(supplier: &str) -> Vec<&'static str> {
        match supplier.to_lowercase().as_str() {
            "anyrouter" => vec!["https://anyrouter.top"],
            _ => Vec::new(),
        }
    }

    // 汇总：每个 supplier 选中的最优 URL（用于整体排序输出）
    let mut supplier_best: Vec<(usize, String, String, &'static str, u64)> = Vec::new(); // (priority, supplier, url, kind, metric_ms)

    for (priority, providers_in_level) in priority_groups.into_iter() {
        let mut supplier_urls: HashMap<String, HashMap<String, Vec<Provider>>> = HashMap::new();

        for provider in providers_in_level {
            let supplier = supplier_name(&provider);
            let Some(base_url) = extract_base_url(&provider, app_type_str.as_str()) else {
                continue;
            };
            supplier_urls
                .entry(supplier)
                .or_default()
                .entry(base_url)
                .or_default()
                .push(provider);
        }

        if supplier_urls.is_empty() {
            continue;
        }

        println!("=== 层级 {} ===", priority);

        for (supplier, url_groups) in supplier_urls.into_iter() {
            println!("\n供应商: {}", supplier);

            let results = router
                .benchmark_urls_detailed(
                    app_type_str.as_str(),
                    priority,
                    test_model,
                    &supplier,
                    &url_groups,
                )
                .await;

            // 展示该 supplier 下各 URL 的结果
            for (i, r) in results.iter().enumerate() {
                use cc_switch_lib::proxy::provider_router::UrlProbeKind;
                match &r.kind {
                    UrlProbeKind::FullOk { latency_ms } => {
                        println!("  {}. {} - OK {}ms", i + 1, r.url, latency_ms);
                    }
                    UrlProbeKind::Overloaded { latency_ms, message } => {
                        println!(
                            "  {}. {} - OV {}ms ({})",
                            i + 1,
                            r.url,
                            latency_ms,
                            message
                        );
                    }
                    UrlProbeKind::FallbackOk {
                        connect_ms,
                        penalty_ms,
                        ..
                    } => {
                        println!(
                            "  {}. {} - FB {}ms (+{}ms)",
                            i + 1,
                            r.url,
                            connect_ms,
                            penalty_ms
                        );
                    }
                    UrlProbeKind::Failed { reason } => {
                        println!("  {}. {} - FAIL ({})", i + 1, r.url, reason);
                    }
                }
            }

            // 选择“该 supplier 最终会选用的 URL”（匹配路由思路：优先 URL（若 OK）> OK 最快 > OV > FB）
            use cc_switch_lib::proxy::provider_router::UrlProbeKind;
            let mut preferred: Vec<String> = default_url_priority_for_supplier(&supplier)
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            if let Some(p) = url_groups.values().flat_map(|v| v.first()).next() {
                preferred.extend(parse_url_priority_from_provider(p));
            }
            {
                let mut seen = std::collections::HashMap::<String, ()>::new();
                preferred.retain(|u| seen.insert(u.to_string(), ()).is_none());
            }

            let preferred_ok = preferred.iter().find_map(|u| {
                results.iter().find(|r| {
                    r.url == *u && matches!(r.kind, UrlProbeKind::FullOk { .. })
                })
            });

            let pick = if let Some(r) = preferred_ok {
                Some(r)
            } else {
                results.iter().find(|r| matches!(r.kind, UrlProbeKind::FullOk { .. }))
                    .or_else(|| results.iter().find(|r| matches!(r.kind, UrlProbeKind::Overloaded { .. })))
                    .or_else(|| results.iter().find(|r| matches!(r.kind, UrlProbeKind::FallbackOk { .. })))
            };

            if let Some(r) = pick {
                let (kind, metric_ms) = match &r.kind {
                    UrlProbeKind::FullOk { latency_ms } => ("OK", *latency_ms),
                    UrlProbeKind::Overloaded { latency_ms, .. } => {
                        ("OV", latency_ms.saturating_add(OVERLOADED_PENALTY_MS))
                    }
                    UrlProbeKind::FallbackOk {
                        connect_ms,
                        penalty_ms,
                        ..
                    } => ("FB", connect_ms.saturating_add(*penalty_ms)),
                    UrlProbeKind::Failed { .. } => ("FAIL", u64::MAX),
                };

                supplier_best.push((priority, supplier.clone(), r.url.clone(), kind, metric_ms));
            }
        }
    }

    if supplier_best.is_empty() {
        println!("\n=== 汇总 ===");
        println!("全部测试失败（没有任何可用URL）");
        return Ok(());
    }

    supplier_best.sort_by_key(|(_, _, _, _, latency)| *latency);

    println!("\n=== 汇总（按最优URL延迟排序）===");
    for (i, (priority, supplier, url, kind, metric_ms)) in supplier_best.iter().enumerate() {
        match *kind {
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
            "FB" => {
                let connect_ms = metric_ms.saturating_sub(CONNECTIVITY_PENALTY_MS);
                println!(
                    "{}. [层级 {}] {} -> {} - FB {}ms (+{}ms)",
                    i + 1,
                    priority,
                    supplier,
                    url,
                    connect_ms,
                    CONNECTIVITY_PENALTY_MS
                );
            }
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
