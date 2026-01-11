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
    use std::io::Write;

    // 初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        // 统一格式：
        // [2026-01-11 18:02:37.257 INFO] [codex ] 正常 200 - hyb ... ( 2.770s) [上游: gpt-5.2]
        // - 北京时间 (UTC+8)
        // - 隐藏模块名/路径
        .format(|buf, record| {
            let tz = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            let now = chrono::Utc::now().with_timezone(&tz);
            writeln!(
                buf,
                "[{} {}] {}",
                now.format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.args()
            )
        })
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

        #[derive(Debug, Clone, Default)]
        struct CodexAutoHint {
            model: Option<String>,
            endpoint: Option<String>,
            openai_beta: Option<String>,
            openai_version: Option<String>,
            openai_organization: Option<String>,
            openai_project: Option<String>,
            accept: Option<String>,
            content_type: Option<String>,
            user_agent: Option<String>,
            stainless_runtime: Option<String>,
            stainless_runtime_version: Option<String>,
            stainless_package_version: Option<String>,
            stainless_os: Option<String>,
            stainless_arch: Option<String>,
            stainless_lang: Option<String>,
            stream: Option<bool>,
            body_keys: Vec<String>,
            input_shape: Option<String>,
            messages_shape: Option<String>,
            prompt_cache_key: Option<String>,
            include: Vec<String>,
            reasoning_effort: Option<String>,
            tool_choice: Option<String>,
            parallel_tool_calls: Option<bool>,
            store: Option<bool>,
            text_format_type: Option<String>,
            instructions_len: Option<u32>,
        }

        #[derive(Debug, Clone)]
        struct CodexModelEntry {
            id: String,
            supported_endpoint_types: Vec<String>,
        }

        impl CodexModelEntry {
            fn supports_responses(&self) -> bool {
                self.supported_endpoint_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("openai_responses"))
            }

            fn supports_chat(&self) -> bool {
                self.supported_endpoint_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("openai_chat"))
            }
        }

        async fn fetch_codex_auto_hint(
            client: &reqwest::Client,
            base: &str,
        ) -> CodexAutoHint {
            let Ok(resp) = client.get(format!("{base}/status")).send().await else {
                return CodexAutoHint::default();
            };
            let Ok(v) = resp.json::<cc_switch_lib::proxy::ProxyStatus>().await else {
                return CodexAutoHint::default();
            };
            let Some(r) = v.last_requests.get("codex") else {
                return CodexAutoHint::default();
            };
            let model = if r.model.trim().is_empty() || r.model == "unknown" {
                None
            } else {
                Some(r.model.clone())
            };
            let endpoint = if r.endpoint.trim().is_empty() {
                None
            } else {
                Some(r.endpoint.clone())
            };
            CodexAutoHint {
                model,
                endpoint,
                openai_beta: r.openai_beta.clone(),
                openai_version: r.openai_version.clone(),
                openai_organization: r.openai_organization.clone(),
                openai_project: r.openai_project.clone(),
                accept: r.accept.clone(),
                content_type: r.content_type.clone(),
                user_agent: r.user_agent.clone(),
                stainless_runtime: r.stainless_runtime.clone(),
                stainless_runtime_version: r.stainless_runtime_version.clone(),
                stainless_package_version: r.stainless_package_version.clone(),
                stainless_os: r.stainless_os.clone(),
                stainless_arch: r.stainless_arch.clone(),
                stainless_lang: r.stainless_lang.clone(),
                stream: r.stream,
                body_keys: r.body_keys.clone(),
                input_shape: r.input_shape.clone(),
                messages_shape: r.messages_shape.clone(),
                prompt_cache_key: r.prompt_cache_key.clone(),
                include: r.include.clone(),
                reasoning_effort: r.reasoning_effort.clone(),
                tool_choice: r.tool_choice.clone(),
                parallel_tool_calls: r.parallel_tool_calls,
                store: r.store,
                text_format_type: r.text_format_type.clone(),
                instructions_len: r.instructions_len,
            }
        }

        fn score_codex_model(id: &str, prefer: Option<&str>) -> i32 {
            let s = id.to_lowercase();
            let mut score = 0i32;
            // 优先贴近 Codex 实际默认模型：优先 family/major/minor
            if s.starts_with("gpt-5.2") {
                score += 120;
            } else if s.starts_with("gpt-5.") {
                score += 90;
            } else if s.starts_with("gpt-4.") || s.starts_with("gpt-4o") || s.starts_with("gpt-4") {
                score += 60;
            }

            // 次级偏好：codex/mini 等后缀（不应压过主版本优先级）
            if s.contains("codex") {
                score += 15;
            }
            if s.contains("mini") {
                score += 10;
            }
            if s.contains("-202") {
                score += 8;
            }
            if let Some(p) = prefer {
                if s == p.to_lowercase() {
                    score += 60;
                } else if s.contains(&p.to_lowercase()) {
                    score += 20;
                }
            }
            score -= (id.len() as i32).min(60) / 6;
            score
        }

        fn read_success_models_from_local_logs(provider_id: &str, limit: usize) -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            let Some(home) = dirs::home_dir() else {
                return out;
            };
            let db_path = home.join(".cc-switch").join("cc-switch.db");
            let conn = rusqlite::Connection::open_with_flags(
                db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            );
            let Ok(conn) = conn else {
                return out;
            };
            let _ = conn.busy_timeout(std::time::Duration::from_millis(800));
            let sql = "SELECT model FROM proxy_request_logs \
                       WHERE app_type = 'codex' AND provider_id = ?1 \
                         AND status_code BETWEEN 200 AND 299 \
                         AND model IS NOT NULL AND model != '' AND model != 'unknown' \
                       ORDER BY created_at DESC LIMIT ?2";
            let mut stmt = match conn.prepare(sql) {
                Ok(s) => s,
                Err(_) => return out,
            };
            let mut rows = match stmt.query(rusqlite::params![provider_id, limit as i64]) {
                Ok(r) => r,
                Err(_) => return out,
            };
            while let Ok(Some(row)) = rows.next() {
                let r: String = match row.get(0) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let r = sanitize_gpt_model_name_for_display(&r);
                if out.iter().any(|x| x.eq_ignore_ascii_case(&r)) {
                    continue;
                }
                out.push(r);
            }
            out
        }

        fn is_disallowed_codex_probe_model(id: &str) -> bool {
            // 仅用于测速“兜底猜测”时的过滤：避免使用带日期版本的别名（用户要求）
            // 注意：如果 /v1/models 明确返回该模型名，则不会走到这里。
            sanitize_gpt_model_name_for_display(id) != id.trim()
        }

        fn sanitize_gpt_model_name_for_display(id: &str) -> String {
            // 任何情况下：不展示带日期后缀的 GPT 模型名（例如 gpt-5.2-2025-12-11 / gpt-5.2-20251211）
            let trimmed = id.trim();
            if trimmed.is_empty() {
                return trimmed.to_string();
            }
            let lower = trimmed.to_lowercase();
            if !lower.starts_with("gpt-") {
                return trimmed.to_string();
            }

            fn is_all_digits(s: &str) -> bool {
                !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
            }

            fn is_year(s: &str) -> bool {
                if s.len() != 4 || !is_all_digits(s) {
                    return false;
                }
                matches!(s.parse::<u32>(), Ok(y) if (2000..=2099).contains(&y))
            }

            fn is_mm_or_dd(s: &str) -> bool {
                s.len() == 2 && is_all_digits(s)
            }

            fn is_yyyymmdd(s: &str) -> bool {
                s.len() == 8 && is_all_digits(s) && s.starts_with("20")
            }

            fn is_mmdd_code(s: &str) -> bool {
                // gpt-4-0613 / gpt-4-1106 / gpt-4-0314
                if s.len() != 4 || !is_all_digits(s) {
                    return false;
                }
                let mm = s[0..2].parse::<u32>().ok();
                let dd = s[2..4].parse::<u32>().ok();
                matches!((mm, dd), (Some(m), Some(d)) if (1..=12).contains(&m) && (1..=31).contains(&d))
            }

            let parts: Vec<&str> = trimmed.split('-').collect();
            for i in 0..parts.len() {
                if is_yyyymmdd(parts[i]) {
                    return parts[..i].join("-");
                }
                if is_year(parts[i]) {
                    // YYYY-MM-DD
                    if i + 2 < parts.len() && is_mm_or_dd(parts[i + 1]) && is_mm_or_dd(parts[i + 2])
                    {
                        return parts[..i].join("-");
                    }
                    // 只要出现 20xx 也截断（避免奇形怪状日期）
                    return parts[..i].join("-");
                }
                if is_mmdd_code(parts[i]) {
                    return parts[..i].join("-");
                }
            }

            // 宽松兜底：只要包含 -202 就截断
            if let Some(idx) = lower.find("-202") {
                return trimmed[..idx].to_string();
            }

            trimmed.to_string()
        }

        fn codex_model_probe_rank(id: &str) -> (i32, i32, i32, i32, usize) {
            // 越小越优先：尽量贴近 gpt-5.2，同时在同一“贴近度”内尽量选择 low/mini
            let s = id.to_lowercase();

            // tier：low/mini 优先，其次普通，其次 high/max（xhigh 也视为更贵）
            let tier = if s.contains("mini") || s.contains("-low") || s.ends_with("-low") {
                0
            } else if s.contains("high") || s.contains("-high") || s.contains("xhigh") {
                2
            } else if s.contains("max") || s.contains("-max") {
                3
            } else {
                1
            };

            // family/major/minor：优先 gpt-5.2，其次 gpt-5.*，再 gpt-4.*
            let mut family = 9i32; // 0: gpt-5.2, 1: gpt-5.*, 2: gpt-4.*, 9: other
            let mut minor_distance = 99i32; // 对 gpt-5.* 使用 |minor-2|，越小越好
            let mut codex_bias = 1i32; // 含 codex 更优先

            if s.contains("codex") {
                codex_bias = 0;
            }

            if let Some(rest) = s.strip_prefix("gpt-") {
                let head = rest.split(|c| c == '-' || c == '_').next().unwrap_or(rest);
                if head.starts_with("5.2") || head == "5.2" {
                    family = 0;
                    minor_distance = 0;
                } else if head.starts_with('5') {
                    family = 1;
                    if let Some((_, b)) = head.split_once('.') {
                        let b = b
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect::<String>();
                        if let Ok(mi) = b.parse::<i32>() {
                            minor_distance = (mi - 2).abs();
                        }
                    }
                } else if head.starts_with('4') {
                    family = 2;
                }
            }

            let len_penalty = id.len();
            (family, minor_distance, tier, codex_bias, len_penalty)
        }

        async fn fetch_codex_models(
            client: &reqwest::Client,
            base_url: &str,
            api_key: &str,
        ) -> Option<Vec<CodexModelEntry>> {
            fn build_url(base_url: &str, endpoint: &str) -> String {
                let base_trimmed = base_url.trim_end_matches('/');
                let endpoint_trimmed = endpoint.trim_start_matches('/');
                let mut url = format!("{base_trimmed}/{endpoint_trimmed}");
                if url.contains("/v1/v1") {
                    url = url.replace("/v1/v1", "/v1");
                }
                url
            }

            // 部分供应商会把 models 暴露在 /models（无 /v1）或 base_url 已包含 /v1
            let endpoints = ["/v1/models", "/models"];
            for ep in endpoints.iter() {
                let url = build_url(base_url, ep);
                let resp = client
                    .get(url)
                    .header("accept", "application/json")
                    .header("authorization", format!("Bearer {}", api_key))
                    .send()
                    .await
                    .ok()?;
                if !resp.status().is_success() {
                    continue;
                }
                let v = resp.json::<serde_json::Value>().await.ok()?;
                let data = v.get("data")?.as_array()?;
                let mut out: Vec<CodexModelEntry> = Vec::new();
                for item in data.iter() {
                    let Some(id) = item.get("id").and_then(|x| x.as_str()) else {
                        continue;
                    };
                    // 任何情况下都不保留带日期后缀的 gpt-*（部分供应商 /v1/models 会返回“伪模型别名”）
                    let id = sanitize_gpt_model_name_for_display(id);
                    if id.trim().is_empty() {
                        continue;
                    }
                    let supported = item
                        .get("supported_endpoint_types")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_default();
                    if let Some(existing) = out.iter_mut().find(|m| m.id.eq_ignore_ascii_case(&id))
                    {
                        for t in supported {
                            if existing
                                .supported_endpoint_types
                                .iter()
                                .any(|x| x.eq_ignore_ascii_case(&t))
                            {
                                continue;
                            }
                            existing.supported_endpoint_types.push(t);
                        }
                        continue;
                    }
                    out.push(CodexModelEntry {
                        id,
                        supported_endpoint_types: supported,
                    });
                }
                if out.is_empty() {
                    continue;
                }

                // 如果存在 GPT 模型，则优先只保留 GPT-*（避免选到 gemini/embedding 等导致无意义的测速请求）
                if out.iter().any(|m| m.id.to_lowercase().starts_with("gpt-")) {
                    out.retain(|m| m.id.to_lowercase().starts_with("gpt-"));
                }

                if !out.is_empty() {
                    return Some(out);
                }
            }
            None
        }

        async fn diagnose_codex_base_url_hint(
            client: &reqwest::Client,
            base_url: &str,
        ) -> Option<String> {
            let base = base_url.trim_end_matches('/').to_string();
            let resp = client
                .get(&base)
                .header("accept", "text/html,application/json,*/*")
                .send()
                .await
                .ok()?;

            let status = resp.status().as_u16();
            let ct = resp
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_lowercase();

            // 仅做轻量诊断：避免输出过多内容
            let text = resp.text().await.ok().unwrap_or_default();
            let lower = text.to_lowercase();
            if lower.contains("站点已暂停")
                || lower.contains("站点已暂")
                || lower.contains("site suspended")
                || lower.contains("site is suspended")
            {
                return Some("根URL提示站点已暂停/非API入口".to_string());
            }
            if ct.contains("text/html") && lower.contains("openresty") && status == 200 {
                return Some("根URL返回 openresty HTML，疑似非 OpenAI 兼容入口".to_string());
            }
            if status >= 400 {
                return Some(format!("根URL状态码 {status}（可能需要路径前缀或被网关拦截）"));
            }
            None
        }

        fn choose_codex_candidates(
            prefer_model: Option<&str>,
            models: Option<&[CodexModelEntry]>,
            recent_success: &[String],
        ) -> Vec<CodexModelEntry> {
            let mut candidates: Vec<CodexModelEntry> = Vec::new();

            // 规则：
            // 1) 若能拉到 /v1/models：只从真实列表里选（不预设可能不存在的模型名）
            // 2) 选“尽可能低档/低版本”的模型（mini/low、较低 minor 等）
            // 3) 如果有“最近成功模型”，把它作为兜底候选（必须在 models 列表里才使用）

            if let Some(list) = models {
                let mut pool: Vec<CodexModelEntry> = list.to_vec();
                pool.sort_by(|a, b| {
                    codex_model_probe_rank(a.id.as_str()).cmp(&codex_model_probe_rank(b.id.as_str()))
                });

                // 默认最多尝试 2 个模型：1 个最低档 + 1 个“最近成功兜底”（尽量减少测速 token 消耗）
                for m in pool.iter() {
                    if candidates.len() >= 1 {
                        break;
                    }
                    if candidates.iter().any(|x| x.id.eq_ignore_ascii_case(&m.id)) {
                        continue;
                    }
                    candidates.push(m.clone());
                }

                // 再追加一个“最近成功模型”作为兜底（必须真实存在于 models 列表）
                let fallback = prefer_model
                    .filter(|p| !p.trim().is_empty() && *p != "unknown")
                    .map(|p| p.to_string())
                    .or_else(|| recent_success.first().cloned());
                if let Some(m) = fallback {
                    if let Some(found) = pool.iter().find(|x| x.id.eq_ignore_ascii_case(&m)) {
                        if !candidates.iter().any(|x| x.id.eq_ignore_ascii_case(&found.id)) {
                            candidates.push(found.clone());
                        }
                    }
                }

                // 不足 2 个时，用下一个最低档补齐
                for m in pool.into_iter() {
                    if candidates.len() >= 2 {
                        break;
                    }
                    if candidates.iter().any(|x| x.id.eq_ignore_ascii_case(&m.id)) {
                        continue;
                    }
                    candidates.push(m);
                }

                candidates.truncate(2);
                return candidates;
            }

            // /v1/models 不可用：退回到“本机历史成功模型”（仍然不预设新格式），同样最多 2 个
            // 注意：/v1/models 可能被供应商禁用/拦截，但真实请求仍可用。
            // 这里允许使用极小的“兜底候选集”（不含日期后缀），保证 `csc t` 在“先测再用”的场景可工作。
            //
            // 规则：最多尝试 2 个模型，优先 low/mini + codex，其次贴近 gpt-5.2。
            let builtin = [
                "gpt-5.2-codex-low",
                "gpt-5.2-codex",
                "gpt-5.2",
                "gpt-5.1-codex-mini",
                "gpt-5.1-codex",
                "gpt-5.1",
            ];

            let mut pool: Vec<String> = Vec::new();
            if let Some(p) = prefer_model
                .map(|s| s.to_string())
                .filter(|m| !m.trim().is_empty() && *m != "unknown")
                .filter(|m| !is_disallowed_codex_probe_model(m))
            {
                pool.push(p);
            }
            for m in recent_success.iter() {
                if is_disallowed_codex_probe_model(m) {
                    continue;
                }
                pool.push(m.clone());
            }
            for m in builtin.iter() {
                if is_disallowed_codex_probe_model(m) {
                    continue;
                }
                pool.push(m.to_string());
            }

            // 去重（忽略大小写）
            let mut deduped: Vec<String> = Vec::new();
            for m in pool.into_iter() {
                if deduped.iter().any(|x| x.eq_ignore_ascii_case(&m)) {
                    continue;
                }
                deduped.push(m);
            }
            deduped.sort_by(|a, b| codex_model_probe_rank(a).cmp(&codex_model_probe_rank(b)));

            for m in deduped.into_iter() {
                if candidates.len() >= 2 {
                    break;
                }
                candidates.push(CodexModelEntry {
                    id: m,
                    supported_endpoint_types: Vec::new(),
                });
            }
            candidates
        }

        fn choose_codex_api_order_for_model(
            preferred_api: &'static str,
            fallback_api: &'static str,
            model: &CodexModelEntry,
        ) -> Vec<&'static str> {
            // 若 /v1(models) 返回了 supported_endpoint_types，则尽量避免做无谓的 API 端点重试
            if model.supports_responses() && !model.supports_chat() {
                return vec!["responses"];
            }
            if model.supports_chat() && !model.supports_responses() {
                return vec!["chat"];
            }
            vec![preferred_api, fallback_api]
        }

        fn is_codex_full_context_supplier(supplier: &str) -> bool {
            if supplier.eq_ignore_ascii_case("wong") {
                return true;
            }
            // 允许通过环境变量扩展：逗号分隔 supplier 名称
            // 例如：export CC_SWITCH_CODEX_FULL_CONTEXT_SUPPLIERS=wong,foo,bar
            let Ok(v) = std::env::var("CC_SWITCH_CODEX_FULL_CONTEXT_SUPPLIERS") else {
                return false;
            };
            for part in v.split(',') {
                let s = part.trim();
                if s.is_empty() {
                    continue;
                }
                if supplier.eq_ignore_ascii_case(s) {
                    return true;
                }
            }
            false
        }

        /// 对“已知探测不稳定但真实可用”的 supplier：允许使用“历史成功请求”做无 token 的保底判定。
        ///
        /// 注意：这不是“真实请求测速”。默认关闭，只有显式配置才会启用：
        /// `export CC_SWITCH_CODEX_NO_TOKEN_BENCHMARK_SUPPLIERS=wong,foo`
        fn is_codex_no_token_benchmark_supplier(supplier: &str) -> bool {
            let Ok(v) = std::env::var("CC_SWITCH_CODEX_NO_TOKEN_BENCHMARK_SUPPLIERS") else {
                return false;
            };
            for part in v.split(',') {
                let s = part.trim();
                if s.is_empty() {
                    continue;
                }
                if supplier.eq_ignore_ascii_case(s) {
                    return true;
                }
            }
            false
        }

        async fn run_startup_override_once(
            client: &reqwest::Client,
            trigger_client: &reqwest::Client,
            base: &str,
            app_type: &str,
            priority: usize,
            supplier: &str,
            url: &str,
            model: &str,
            first_api: &'static str,
            override_ttl_secs: u64,
            timeout_secs: u64,
            spawn_claude: bool,
            verbose_child: bool,
            workdir: &str,
            codex_hint: &CodexAutoHint,
            codex_openai_beta: Option<&str>,
            codex_payload_variant: u8,
            codex_responses_max_output_tokens: u32,
            codex_chat_max_tokens: u32,
            codex_full_context: bool,
        ) -> (
            Option<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult>,
            Option<&'static str>,
        ) {
            use std::process::{Command, Stdio};
            use std::time::Duration;

            let run_id = uuid::Uuid::new_v4().to_string();
            let start_resp = client
                .post(format!("{base}/__cc_switch/test_override/start"))
                .json(&serde_json::json!({
                    "app_type": app_type,
                    "priority": priority,
                    "supplier": supplier,
                    "base_url": url,
                    "run_id": run_id,
                    "ttl_secs": override_ttl_secs
                }))
                .send()
                .await;

            let ok = match start_resp {
                Ok(r) => r
                    .json::<TestOverrideStartResponse>()
                    .await
                    .ok()
                    .map(|v| v.ok)
                    == Some(true),
                Err(_) => false,
            };
            if !ok {
                return (None, None);
            }

            let mut child: Option<std::process::Child> = None;
            let mut api_used: Option<&'static str> = None;

            if app_type == "claude" {
                if spawn_claude {
                    let trigger = std::env::var("CC_SWITCH_STARTUP_TEST_TRIGGER")
                        .ok()
                        .unwrap_or_else(|| "interactive".to_string())
                        .to_lowercase();

                    let mut cmd = if trigger == "print" {
                        let mut c = Command::new("claude");
                        c.current_dir(workdir);
                        c.arg("-p").arg("ping");
                        c.arg("--output-format").arg("json");
                        c.arg("--model").arg(model);
                        c.stdin(Stdio::null());
                        c
                    } else {
                        let mut c = Command::new("script");
                        c.current_dir(workdir);
                        c.arg("-q");
                        c.arg("-c");
                        c.arg("claude");
                        c.arg("/dev/null");
                        c.stdin(Stdio::null());
                        c
                    };

                    cmd.env("ANTHROPIC_BASE_URL", base);
                    cmd.env("ANTHROPIC_API_KEY", "sk-cc-switch-startup-test");
                    cmd.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1");

                    if verbose_child {
                        cmd.stdout(Stdio::inherit());
                        cmd.stderr(Stdio::inherit());
                    } else {
                        cmd.stdout(Stdio::null());
                        cmd.stderr(Stdio::null());
                    }

                    if let Ok(c) = cmd.spawn() {
                        child = Some(c);
                    } else {
                        return (None, None);
                    }
                } else {
                    let payload = serde_json::json!({
                        "model": model,
                        "max_tokens": 64,
                        "temperature": 0.7,
                        "stream": false,
                        "messages": [{"role":"user","content":"ping"}],
                    });
                    let _ = trigger_client
                        .post(format!("{base}/v1/messages"))
                        .header("content-type", "application/json")
                        .json(&payload)
                        .send()
                        .await;
                }
            } else if app_type == "codex" {
                // Codex：部分供应商（例如 wong）对“探测请求形态”更敏感，导致真实可用但探测失败；
                // 这类供应商默认通过真实 `codex exec` 触发一次请求，以贴近正式使用场景。
                //
                // 配置项：
                // - 强制全局开启：export CC_SWITCH_STARTUP_TEST_SPAWN_CODEX=1
                // - 强制全局关闭：export CC_SWITCH_STARTUP_TEST_SPAWN_CODEX=0
                // - 指定 supplier 列表：export CC_SWITCH_CODEX_SPAWN_CLI_SUPPLIERS=wong,foo,bar
                fn should_spawn_codex_cli_for_supplier(supplier: &str) -> bool {
                    if supplier.eq_ignore_ascii_case("wong") {
                        return true;
                    }
                    let Ok(v) = std::env::var("CC_SWITCH_CODEX_SPAWN_CLI_SUPPLIERS") else {
                        return false;
                    };
                    for part in v.split(',') {
                        let s = part.trim();
                        if s.is_empty() {
                            continue;
                        }
                        if supplier.eq_ignore_ascii_case(s) {
                            return true;
                        }
                    }
                    false
                }

                let spawn_codex = match std::env::var("CC_SWITCH_STARTUP_TEST_SPAWN_CODEX")
                    .ok()
                    .as_deref()
                {
                    Some("0") => false,
                    Some("1") => true,
                    _ => should_spawn_codex_cli_for_supplier(supplier),
                };

                if spawn_codex {
                    let prompt = if codex_full_context {
                        // 几百 token 的“伪真实上下文”，用于避免供应商把极简 ping 当成探活拦截
                        let block = "你是一个资深软件工程师。请阅读以下需求并给出可执行的实现方案：\
1) 需要兼容多供应商模型别名差异；2) 优先 family 再 major/minor 再 thinking；\
3) 需要在运行时自动探测并回写映射，避免重复匹配；4) 保持现有稳定路径不受影响。\
同时给出风险点、回滚策略与验证步骤。";
                        let mut out = String::new();
                        for i in 0..4 {
                            out.push_str(&format!("段落{}：{}\\n", i + 1, block));
                        }
                        out
                    } else {
                        "请只回复：OK（不要执行任何命令，不要输出其它内容）。".to_string()
                    };

                    let proxy_base = format!("{}/v1", base.trim_end_matches('/'));
                    let mut cmd = Command::new("codex");
                    cmd.current_dir(workdir);
                    cmd.arg("exec")
                        .arg("--skip-git-repo-check")
                        .arg("--color")
                        .arg("never")
                        .arg("--sandbox")
                        .arg("read-only")
                        .arg("--ask-for-approval")
                        .arg("never")
                        .arg("-C")
                        .arg(workdir)
                        .arg("-m")
                        .arg(model)
                        .arg("-c")
                        .arg("model_provider=\"cc-switch-proxy\"")
                        .arg("-c")
                        .arg(format!(
                            "model_providers.cc-switch-proxy.base_url=\"{proxy_base}\""
                        ))
                        // dummy key：仅用于让 codex CLI 本地校验通过；本代理不会透传 Authorization 到上游
                        .env("OPENAI_API_KEY", "sk-dummy")
                        .arg(prompt)
                        .stdin(Stdio::null());

                    if verbose_child {
                        cmd.stdout(Stdio::inherit());
                        cmd.stderr(Stdio::inherit());
                    } else {
                        cmd.stdout(Stdio::null());
                        cmd.stderr(Stdio::null());
                    }

                    if let Ok(c) = cmd.spawn() {
                        child = Some(c);
                        api_used = Some(first_api);
                    }
                }

                if child.is_none() {
                let req_id = format!("cc-switch-startup-{}", uuid::Uuid::new_v4());
                let hint_keys: std::collections::HashSet<String> = codex_hint
                    .body_keys
                    .iter()
                    .map(|k| k.to_lowercase())
                    .collect();

                fn synthetic_codex_probe_text() -> String {
                    // 几百 token 的“伪真实上下文”，用于绕过部分供应商对极简 ping 的风控/路由差异
                    // 目标：长度适中（不追求严格 token 计数），但明显不像探活请求。
                    let block = "你是一个资深软件工程师。请阅读以下需求并给出可执行的实现方案：\
1) 需要兼容多供应商模型别名差异；2) 优先 family 再 major/minor 再 thinking；\
3) 需要在运行时自动探测并回写映射，避免重复匹配；4) 保持现有稳定路径不受影响。\
同时给出风险点、回滚策略与验证步骤。";
                    let mut out = String::new();
                    for i in 0..4 {
                        out.push_str(&format!("段落{}：{}\\n", i + 1, block));
                    }
                    out
                }

                let enrich_payload = |mut payload: serde_json::Value| -> serde_json::Value {
                    let Some(obj) = payload.as_object_mut() else {
                        return payload;
                    };

                    let has = |k: &str| hint_keys.contains(k);
                    let force = codex_full_context;

                    // 仅在真实请求中出现过的 key 才补齐，避免瞎猜导致上游拒绝
                    if (has("reasoning") || force) && !obj.contains_key("reasoning") {
                        let effort = codex_hint
                            .reasoning_effort
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("low");
                        obj.insert(
                            "reasoning".to_string(),
                            serde_json::json!({ "effort": effort }),
                        );
                    }
                    if (has("parallel_tool_calls") || force) && !obj.contains_key("parallel_tool_calls") {
                        obj.insert(
                            "parallel_tool_calls".to_string(),
                            serde_json::json!(codex_hint.parallel_tool_calls.unwrap_or(true)),
                        );
                    }
                    if (has("tools") || force) && !obj.contains_key("tools") {
                        obj.insert("tools".to_string(), serde_json::json!([]));
                    }
                    if (has("tool_choice") || force) && !obj.contains_key("tool_choice") {
                        let tc = codex_hint
                            .tool_choice
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("auto");
                        obj.insert("tool_choice".to_string(), serde_json::json!(tc));
                    }
                    if (has("metadata") || force) && !obj.contains_key("metadata") {
                        obj.insert(
                            "metadata".to_string(),
                            serde_json::json!({ "cc_switch_startup_test": true }),
                        );
                    }
                    if (has("store") || force) && !obj.contains_key("store") {
                        obj.insert("store".to_string(), serde_json::json!(codex_hint.store.unwrap_or(false)));
                    }
                    if has("temperature") && !obj.contains_key("temperature") {
                        obj.insert("temperature".to_string(), serde_json::json!(0.0));
                    }
                    if has("top_p") && !obj.contains_key("top_p") {
                        obj.insert("top_p".to_string(), serde_json::json!(1.0));
                    }
                    if has("seed") && !obj.contains_key("seed") {
                        obj.insert("seed".to_string(), serde_json::json!(1));
                    }
                    if force && !obj.contains_key("prompt_cache_key") {
                        let v = codex_hint
                            .prompt_cache_key
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("cc-switch-probe");
                        obj.insert("prompt_cache_key".to_string(), serde_json::json!(v));
                    }
                    if force && !obj.contains_key("include") {
                        let arr = if codex_hint.include.is_empty() {
                            vec!["usage".to_string()]
                        } else {
                            codex_hint.include.clone()
                        };
                        obj.insert("include".to_string(), serde_json::json!(arr));
                    }
                    if force && !obj.contains_key("text") {
                        let tf = codex_hint
                            .text_format_type
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("text");
                        obj.insert(
                            "text".to_string(),
                            serde_json::json!({ "format": { "type": tf } }),
                        );
                    }
                    if force && !obj.contains_key("instructions") {
                        // 不持久化真实内容；测试时用合成内容即可
                        obj.insert("instructions".to_string(), serde_json::json!(synthetic_codex_probe_text()));
                    }
                    if has("stream_options") && !obj.contains_key("stream_options") {
                        if obj.get("stream").and_then(|v| v.as_bool()) == Some(true) {
                            obj.insert(
                                "stream_options".to_string(),
                                serde_json::json!({ "include_usage": true }),
                            );
                        }
                    }

                    payload
                };

                let probe_text = if codex_full_context {
                    synthetic_codex_probe_text()
                } else {
                    "ping".to_string()
                };

                let payload_responses = if codex_payload_variant == 0 {
                    // 尽量贴近官方 Codex CLI：structured input + typed parts
                    serde_json::json!({
                        "model": model,
                        "max_output_tokens": codex_responses_max_output_tokens,
                        "stream": true,
                        "input": [{
                            "role": "user",
                            "content": [{"type":"input_text","text": probe_text}]
                        }],
                    })
                } else {
                    // 兼容部分供应商：string input
                    serde_json::json!({
                        "model": model,
                        "max_output_tokens": codex_responses_max_output_tokens,
                        "stream": true,
                        "input": probe_text,
                    })
                };
                let payload_responses = enrich_payload(payload_responses);

                    let common_headers = |rb: reqwest::RequestBuilder| {
                        let accept = codex_hint
                            .accept
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("text/event-stream");
                        let ua = codex_hint
                            .user_agent
                            .as_deref()
                            .filter(|s| !s.trim().is_empty())
                            .unwrap_or("codex_cli_rs/0.5.2");

                        let mut rb = rb
                            .header("content-type", "application/json")
                            .header("accept", accept)
                            // 部分上游会对 UA 做路由/放行判断，尽量模拟官方 Codex CLI
                            .header("user-agent", ua)
                            .header("x-request-id", req_id.clone());

                        // x-stainless-* 仅在真实请求出现过时才透传。
                        // 原因：部分供应商会对未知/伪造的 x-stainless 头做风控，导致“真实可用但测速 400”。
                        let has_any_stainless = codex_hint.stainless_runtime.as_deref().is_some()
                            || codex_hint.stainless_runtime_version.as_deref().is_some()
                            || codex_hint.stainless_package_version.as_deref().is_some()
                            || codex_hint.stainless_os.as_deref().is_some()
                            || codex_hint.stainless_arch.as_deref().is_some()
                            || codex_hint.stainless_lang.as_deref().is_some();

                        if has_any_stainless {
                            if let Some(v) = codex_hint
                                .stainless_os
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-os", v);
                            }
                            if let Some(v) = codex_hint
                                .stainless_arch
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-arch", v);
                            }
                            if let Some(v) = codex_hint
                                .stainless_lang
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-lang", v);
                            }
                            if let Some(v) = codex_hint
                                .stainless_runtime
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-runtime", v);
                            }
                            if let Some(v) = codex_hint
                                .stainless_runtime_version
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-runtime-version", v);
                            }
                            if let Some(v) = codex_hint
                                .stainless_package_version
                                .as_deref()
                                .filter(|s| !s.trim().is_empty())
                            {
                                rb = rb.header("x-stainless-package-version", v);
                            }
                        }

                    if let Some(beta) = codex_openai_beta {
                        rb = rb.header("openai-beta", beta);
                    }
                    if let Some(v) = codex_hint
                        .openai_version
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                    {
                        rb = rb.header("openai-version", v);
                    }
                    if let Some(v) = codex_hint
                        .openai_project
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                    {
                        rb = rb.header("openai-project", v);
                    }
                    if let Some(v) = codex_hint
                        .openai_organization
                        .as_deref()
                        .filter(|s| !s.trim().is_empty())
                    {
                        rb = rb.header("openai-organization", v);
                    }
                    rb
                };

                let path = if first_api == "chat" {
                    "/v1/chat/completions"
                } else {
                    "/v1/responses"
                };

                let _ = if first_api == "chat" {
                    let payload_chat = if codex_payload_variant == 0 {
                        serde_json::json!({
                            "model": model,
                            "max_tokens": codex_chat_max_tokens,
                            "temperature": 0.0,
                            "stream": true,
                            "messages": [{"role":"user","content": probe_text}],
                        })
                    } else {
                        // typed content（少量供应商仅接受数组形式）
                        serde_json::json!({
                            "model": model,
                            "max_tokens": codex_chat_max_tokens,
                            "temperature": 0.0,
                            "stream": true,
                            "messages": [{
                                "role":"user",
                                "content":[{"type":"text","text": probe_text}]
                            }],
                        })
                    };
                    let payload_chat = enrich_payload(payload_chat);
                    common_headers(trigger_client.post(format!("{base}{path}")))
                        .json(&payload_chat)
                        .send()
                        .await
                } else {
                    common_headers(trigger_client.post(format!("{base}{path}")))
                        .json(&payload_responses)
                        .send()
                        .await
                };

                api_used = Some(first_api);
                }
            } else if app_type == "gemini" {
                let payload = serde_json::json!({
                    "contents": [{
                        "role": "user",
                        "parts": [{"text": "ping"}]
                    }]
                });
                let path = format!("models/{}:generateContent", model);
                let _ = trigger_client
                    .post(format!("{base}/v1beta/{path}"))
                    .header("content-type", "application/json")
                    .json(&payload)
                    .send()
                    .await;
            }

            let timeout = Duration::from_secs(timeout_secs);
            let poll = Duration::from_millis(350);
            let start = std::time::Instant::now();
            let mut got: Option<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult> = None;

            loop {
                if start.elapsed() > timeout {
                    break;
                }

                if let Some(c) = child.as_mut() {
                    let _ = c.try_wait();
                }

                if let Ok(resp) = client
                    .get(format!("{base}/__cc_switch/test_override/result/{}", run_id))
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

            if let Some(mut c) = child {
                let _ = c.kill();
                let _ = c.wait();
            }

            (got, api_used)
        }

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
            "\n开始URL延迟测试（启动/退出真实链路），共{}个供应商（按层级与supplier聚合），model=auto(以真实启动请求为准)\n",
            targets.len()
        );

        // Codex: 读取最近一次真实请求画像（model/endpoint/openai-beta），用于 model=auto 对齐真实环境
        let codex_hint = if app_type_str == "codex" {
            fetch_codex_auto_hint(&client, &base).await
        } else {
            CodexAutoHint::default()
        };

        // 默认不再依赖外部 CLI “启动行为”来触发请求（在部分环境中会导致未触发真实请求而超时）。
        // 如需回退到旧行为（仅 claude），可设置 CC_SWITCH_STARTUP_TEST_SPAWN_CLAUDE=1。
        let spawn_claude = std::env::var("CC_SWITCH_STARTUP_TEST_SPAWN_CLAUDE")
            .ok()
            .as_deref()
            == Some("1");

        let verbose_child = std::env::var("CC_SWITCH_STARTUP_TEST_VERBOSE")
            .ok()
            .as_deref()
            == Some("1");

        let workdir = std::env::var("CC_SWITCH_STARTUP_TEST_WORKDIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
            })
            .unwrap_or_else(|| ".".to_string());

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

        // 触发“真实链路请求”用：需要允许等待上游响应（否则仅 2s 的健康探测 client 容易超时）
        let trigger_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| AppError::Message(format!("创建HTTP客户端失败: {e}")))?;

        // 简单 URL 延迟探测：参考 anyrouter_proxy 的“能返回JSON就算在线”的判定
        async fn simple_url_probe_claude(
            client: &reqwest::Client,
            base_url: &str,
            api_key: &str,
            model: &str,
        ) -> Result<u64, String> {
            let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
            let body = serde_json::json!({
                "model": model,
                "max_tokens": 1,
                "messages": [{"role":"user","content":"ping"}],
            });

            let start = std::time::Instant::now();
            let resp = client
                .post(url)
                .header("content-type", "application/json")
                .header("anthropic-version", "2023-06-01")
                .header("x-api-key", api_key)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("请求失败: {e}"))?;

            let latency = start.elapsed().as_millis() as u64;
            // 简单探测仅关心“是否能拿到响应 + RTT”，不对状态码/内容做判断（避免与真实环境不一致）
            let _ = resp.status();
            Ok(latency)
        }

        // 5) 预加载 providers，避免循环中重复读 DB
        let providers = db.get_failover_providers(&app_type_str)?;

        // 6) 按层级输出，并逐供应商、逐 URL 展示“简单延迟 + 全链路延迟”
        let mut targets_by_priority: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for t in targets.iter() {
            targets_by_priority
                .entry(t.priority)
                .or_default()
                .push(t.supplier.clone());
        }
        for list in targets_by_priority.values_mut() {
            list.sort();
            list.dedup();
        }

        let mut summary: Vec<(usize, String, String, String, u64)> = Vec::new();
        let mut done: usize = 0;
        let total = targets.len();

        for (priority, suppliers) in targets_by_priority.iter() {
            println!("=== 层级 {} ===", priority);

            for supplier in suppliers.iter() {
                done += 1;
                println!("\n[{}/{}] 供应商: {}", done, total, supplier);

                // 收集该 supplier 在该层级的 URL（去重），并为简单探测选一个 key（同 URL 任意 key 即可）
                let mut url_to_key: BTreeMap<String, String> = BTreeMap::new();
                let mut url_to_provider_ids: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for p in providers.iter() {
                    let p_priority = p.sort_index.unwrap_or(999999) as usize;
                    if p_priority != *priority {
                        continue;
                    }
                    let p_supplier = p
                        .name
                        .split('-')
                        .next()
                        .unwrap_or(&p.name)
                        .to_string();
                    if &p_supplier != supplier {
                        continue;
                    }
                    let base_url = match app_type_str.as_str() {
                        "claude" => p
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("ANTHROPIC_BASE_URL"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        "codex" => p
                            .settings_config
                            .get("base_url")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        "gemini" => p
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("GOOGLE_GEMINI_BASE_URL"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        _ => None,
                    };
                    let api_key = match app_type_str.as_str() {
                        "claude" => p
                            .settings_config
                            .get("env")
                            .and_then(|env| {
                                env.get("ANTHROPIC_API_KEY")
                                    .or_else(|| env.get("ANTHROPIC_AUTH_TOKEN"))
                            })
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        "codex" => p
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("OPENAI_API_KEY"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        "gemini" => p
                            .settings_config
                            .get("env")
                            .and_then(|env| env.get("GOOGLE_API_KEY"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        _ => None,
                    };
                    if let (Some(u), Some(k)) = (base_url, api_key) {
                        let u = u.trim().trim_end_matches('/').to_string();
                        url_to_key.entry(u.clone()).or_insert(k);
                        url_to_provider_ids.entry(u).or_default().push(p.id.clone());
                    }
                }

                let mut simple_ms: BTreeMap<String, Option<u64>> = BTreeMap::new();
                if app_type_str == "claude" && !url_to_key.is_empty() {
                    println!("  URL:");
                    for (u, k) in url_to_key.iter() {
                        let probe_client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(12))
                            .build()
                            .map_err(|e| AppError::Message(format!("创建HTTP客户端失败: {e}")))?;

                        match simple_url_probe_claude(&probe_client, u, k, test_model).await {
                            Ok(ms) => {
                                println!("    - {} 简单={}ms", u, ms);
                                simple_ms.insert(u.clone(), Some(ms));
                            }
                            Err(_reason) => {
                                // 简单探测仅用于辅助参考：终端不输出原因（避免“挑战页/网关页”等噪音）
                                println!("    - {} 简单=FAIL", u);
                                simple_ms.insert(u.clone(), None);
                            }
                        }
                    }
                }

                // 逐 URL 做“启动/退出真实链路”全链路测速：
                // 每个 URL 单独设置覆盖并启动一次 claude，让 rust_proxy/forwarder 记录真实请求结果回传 run_id。
                let urls: Vec<String> = url_to_key.keys().cloned().collect();
                if urls.is_empty() {
                    println!("  全链路: FAIL(该supplier未找到任何URL)");
                    continue;
                }

                println!("  全链路:");
                let mut per_url_results: BTreeMap<
                    String,
                    cc_switch_lib::proxy::provider_router::BenchmarkUrlResult,
                > = BTreeMap::new();

                for url in urls.iter() {
                    let simple_part = match simple_ms.get(url).copied().flatten() {
                        Some(ms) => format!("简单={}ms", ms),
                        None => {
                            if app_type_str == "claude" {
                                "简单=FAIL".to_string()
                            } else {
                                "简单=N/A".to_string()
                            }
                        }
                    };

                    // 触发一次（或少量重试）“真实链路请求”进入代理：
                    // - claude：保持现有逻辑
                    // - codex：支持 model=auto，并在必要时基于 /v1/models 选 1-2 个候选模型重试（尽量省 token）
                    // - gemini：保持现有逻辑
                    let api_used: Option<&'static str>;

                    // 仅 codex 用：候选模型列表
                    let mut codex_candidates: Vec<CodexModelEntry> = Vec::new();
                    let mut codex_diag_hint: Option<String> = None;
                    if app_type_str == "codex" {
                        let key = url_to_key.get(url).cloned().unwrap_or_default();
                        let probe_client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(8))
                            .build()
                            .map_err(|e| AppError::Message(format!("创建HTTP客户端失败: {e}")))?;
                        let models = fetch_codex_models(&probe_client, url, &key).await;
                        if models.is_none() {
                            codex_diag_hint = diagnose_codex_base_url_hint(&probe_client, url).await;
                        }
                        let provider_ids = url_to_provider_ids.get(url).cloned().unwrap_or_default();
                        let mut recent_success: Vec<String> = Vec::new();
                        for pid in provider_ids.iter() {
                            let mut ms = read_success_models_from_local_logs(pid, 30);
                            for m in ms.drain(..) {
                                if !recent_success.iter().any(|x| x.eq_ignore_ascii_case(&m)) {
                                    recent_success.push(m);
                                }
                            }
                            if recent_success.len() >= 30 {
                                break;
                            }
                        }
                        // prefer_model：优先使用“真实环境最近请求模型”（/status），避免被本机历史里的日期别名污染
                        let prefer_primary = recent_success
                            .iter()
                            .find(|m| !is_disallowed_codex_probe_model(m))
                            .cloned();
                        let prefer_model = codex_hint
                            .model
                            .as_deref()
                            .or_else(|| prefer_primary.as_deref())
                            .or(Some("gpt-5.2"));
                        codex_candidates =
                            choose_codex_candidates(prefer_model, models.as_deref(), &recent_success);
                    }

                    if app_type_str == "codex" && codex_candidates.is_empty() {
                        let hint = codex_diag_hint
                            .as_deref()
                            .map(|h| format!("；{h}"))
                            .unwrap_or_default();
                        let reason = format!("无法获取该供应商的模型列表(/v1/models)且本机无历史成功模型，跳过测速（避免猜测不存在的模型名）{hint}");
                        let r = cc_switch_lib::proxy::provider_router::BenchmarkUrlResult {
                            url: url.clone(),
                            kind: "FAIL".to_string(),
                            latency_ms: None,
                            penalty_ms: None,
                            message: None,
                            reason: Some(reason),
                        };
                        per_url_results.insert(url.clone(), r.clone());
                        println!(
                            "    - {} 简单=N/A 全链路=FAIL ({})",
                            url,
                            r.reason.as_deref().unwrap_or("-")
                        );
                        continue;
                    }

                    // 执行一次“启动覆盖 + 触发请求 + 等待结果”的封装（返回最终结果与 api_used）
                    let mut got: Option<cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult> = None;
                    let mut got_api_used: Option<&'static str> = None;
                    let codex_openai_beta = codex_hint.openai_beta.as_deref();

                    if app_type_str == "codex" {
                        // 0) codex 保底：对“探测不稳定 supplier”，优先使用“历史成功请求”判断可用性
                        //    - 不触发真实请求：避免浪费 token
                        //    - 不影响正常转发：仅影响 `csc t` 的测速展示/选用
                        let no_token_supplier =
                            is_codex_no_token_benchmark_supplier(supplier.as_str());
                        if no_token_supplier {
                            let provider_ids =
                                url_to_provider_ids.get(url).cloned().unwrap_or_default();
                            // 近 7 天，取最近 20 条成功记录估算中位延迟
                            let stats = db
                                .get_recent_success_stats(&provider_ids, "codex", 20, Some(7 * 86400))
                                .ok()
                                .flatten();

                            if let Some(s) = stats {
                                let endpoint_hint = codex_hint
                                    .endpoint
                                    .as_deref()
                                    .unwrap_or("/v1/responses");
                                let preferred_api: &'static str =
                                    if endpoint_hint.contains("chat/completions") {
                                        "chat"
                                    } else {
                                        "responses"
                                    };
                                got_api_used = Some(preferred_api);

                                let request_model = codex_hint
                                    .model
                                    .clone()
                                    .unwrap_or_else(|| "gpt-5.2".to_string());
                                let effective_model = s
                                    .last_model
                                    .clone()
                                    .filter(|m| !m.trim().is_empty())
                                    .unwrap_or_else(|| request_model.clone());
                                let request_model = sanitize_gpt_model_name_for_display(&request_model);
                                let effective_model =
                                    sanitize_gpt_model_name_for_display(&effective_model);

                                let last_at = chrono::NaiveDateTime::from_timestamp_opt(
                                    s.last_success_at,
                                    0,
                                )
                                .map(|naive| {
                                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                        naive,
                                        chrono::Utc,
                                    )
                                    .to_rfc3339()
                                })
                                .unwrap_or_else(|| s.last_success_at.to_string());

                                let url_result =
                                    cc_switch_lib::proxy::provider_router::BenchmarkUrlResult {
                                        url: url.clone(),
                                        kind: "OK".to_string(),
                                        latency_ms: Some(s.median_latency_ms),
                                        penalty_ms: None,
                                        message: Some(format!(
                                            "历史成功保底 samples={} last_at={}",
                                            s.sample_count, last_at
                                        )),
                                        reason: None,
                                    };

                                got = Some(
                                    cc_switch_lib::proxy::provider_router::BenchmarkSupplierResult {
                                        priority: *priority,
                                        supplier: supplier.clone(),
                                        request_model: Some(request_model),
                                        effective_model: Some(effective_model),
                                        chosen_url: Some(url.clone()),
                                        chosen_kind: "OK".to_string(),
                                        metric_ms: Some(s.median_latency_ms),
                                        urls: vec![url_result],
                                    },
                                );
                            }
                        }

                        if got.is_some() {
                            api_used = got_api_used;
                        } else {
                        let endpoint_hint = codex_hint.endpoint.as_deref().unwrap_or("/v1/responses");
                        let preferred_api: &'static str = if endpoint_hint.contains("chat/completions") {
                            "chat"
                        } else {
                            "responses"
                        };
                        let fallback_api: &'static str = if preferred_api == "chat" { "responses" } else { "chat" };

                        fn needs_min_tokens(reason_text: &str, param: &str) -> Option<u32> {
                            let t = reason_text.to_lowercase();
                            if !t.contains(param) {
                                return None;
                            }
                            // 常见报错：Expected a value >= 16, but got 1
                            if t.contains(">= 16") || t.contains("\\u003e= 16") || t.contains("minimum") {
                                return Some(16);
                            }
                            None
                        }

                        let mut stop_all = false;
                        for m in codex_candidates.iter() {
                            let full_context_supplier = is_codex_full_context_supplier(supplier.as_str());
                            let api_order = if full_context_supplier {
                                // wong 已知对探测更敏感：优先按 endpoint_hint 的 API 测，避免无谓切换 API
                                vec![preferred_api]
                            } else {
                                choose_codex_api_order_for_model(preferred_api, fallback_api, m)
                            };
                            for (api_idx, api_name) in api_order.iter().enumerate() {
                                let mut payload_variant: u8 = if *api_name == "responses" {
                                    match codex_hint.input_shape.as_deref() {
                                        Some("string") => 1,
                                        Some("array") => 0,
                                        _ => 0,
                                    }
                                } else {
                                    match codex_hint.messages_shape.as_deref() {
                                        Some("array") => 1,
                                        _ => 0,
                                    }
                                };
                                // 默认从 1 开始尽量省 token；但对“完整上下文探测”供应商（如 wong）至少用 16，
                                // 避免上游对过小 token 直接拒绝且不回显具体字段限制。
                                let (mut responses_max_output_tokens, mut chat_max_tokens) =
                                    if full_context_supplier { (16, 16) } else { (1, 1) };

                                // 每个 api 最多尝试：
                                // 1) variant=0 + tokens=1
                                // 2) 如果命中“最小 token 限制”，则 tokens=16 重试
                                // 3) 如果仍是 openai_error/格式类错误，则切到 variant=1 再试一次
                                let mut attempts_left: u8 = 3;
                                let mut full_context_used = false;
                                loop {
                                    attempts_left = attempts_left.saturating_sub(1);

                                    let (r, api) = run_startup_override_once(
                                        &client,
                                        &trigger_client,
                                        base.as_str(),
                                        app_type_str.as_str(),
                                        *priority,
                                        supplier.as_str(),
                                        url.as_str(),
                                        m.id.as_str(),
                                        *api_name,
                                        override_ttl_secs,
                                        timeout_secs,
                                        spawn_claude,
                                        verbose_child,
                                        &workdir,
                                        &codex_hint,
                                        codex_openai_beta,
                                        payload_variant,
                                        responses_max_output_tokens,
                                        chat_max_tokens,
                                        full_context_used,
                                    )
                                    .await;
                                    got = r;
                                    got_api_used = api;

                                    let (chosen_kind, reason_text) = {
                                        let kind = got
                                            .as_ref()
                                            .map(|x| x.chosen_kind.as_str())
                                            .unwrap_or("FAIL");
                                        let (reason, message) = got
                                            .as_ref()
                                            .and_then(|x| x.urls.first())
                                            .map(|u| {
                                                (
                                                    u.reason.as_deref().unwrap_or(""),
                                                    u.message.as_deref().unwrap_or(""),
                                                )
                                            })
                                            .unwrap_or(("", ""));
                                        (kind, format!("{reason} {message}").to_lowercase())
                                    };

                                    if chosen_kind == "OK" || chosen_kind == "OV" || chosen_kind == "FB" {
                                        stop_all = true;
                                        break;
                                    }

                                    if *api_name == "responses" {
                                        if let Some(min) = needs_min_tokens(&reason_text, "max_output_tokens") {
                                            if responses_max_output_tokens < min {
                                                responses_max_output_tokens = min;
                                                if attempts_left > 0 {
                                                    continue;
                                                }
                                            }
                                        }
                                    } else if *api_name == "chat" {
                                        if let Some(min) = needs_min_tokens(&reason_text, "max_tokens") {
                                            if chat_max_tokens < min {
                                                chat_max_tokens = min;
                                                if attempts_left > 0 {
                                                    continue;
                                                }
                                            }
                                        }
                                    }

                                    let model_not_found = reason_text.contains("model_not_found")
                                        || reason_text.contains("无可用渠道")
                                        || reason_text.contains("distributor");

                                    let auth_or_quota = reason_text.contains("unauthorized")
                                        || reason_text.contains("401")
                                        || reason_text.contains("403")
                                        || reason_text.contains("resource has been exhausted")
                                        || reason_text.contains("429")
                                        || reason_text.contains("quota")
                                        || reason_text.contains("exhausted");

                                    if auth_or_quota {
                                        stop_all = true;
                                        break;
                                    }

                                    if model_not_found {
                                        // 模型不存在：直接换模型，不必切换 API
                                        break;
                                    }

                                    let api_mismatch_or_payload = reason_text.contains("format mismatch")
                                        || reason_text.contains("openai_error")
                                        || reason_text.contains("bad_response_status_code")
                                        || reason_text.contains("openai_responses")
                                        || reason_text.contains("openai_chat")
                                        || reason_text.contains("only [['openai_chat']]")
                                        || reason_text.contains("only [[\"openai_chat\"]]")
                                        || reason_text.contains("404");

                                    // 先尝试切 payload 形态，再考虑切换 API（避免供应商对 input/messages 形态有差异）
                                    if api_mismatch_or_payload && payload_variant == 0 && attempts_left > 0 {
                                        payload_variant = 1;
                                        continue;
                                    }

                                    // 对 400/openai_error：追加一次“完整上下文探测”（使用编造几百 token 的上下文）
                                    // 该模式仅用于测速，不影响正常转发。
                                    if !full_context_used
                                        && full_context_supplier
                                        && (reason_text.contains("状态码 400") || reason_text.contains("status code 400"))
                                        && reason_text.contains("openai_error")
                                        && attempts_left > 0
                                    {
                                        full_context_used = true;
                                        // 完整上下文探测时，确保 tokens >= 16（供应商可能不回显具体限制）
                                        responses_max_output_tokens =
                                            responses_max_output_tokens.max(16);
                                        chat_max_tokens = chat_max_tokens.max(16);
                                        continue;
                                    }

                                    // 首个 API 失败且看起来是格式/接口不匹配：尝试另一个 API
                                    if api_idx == 0 && api_mismatch_or_payload {
                                        // 结束当前 api，进入 fallback api
                                        break;
                                    }

                                    // 其它失败：换模型
                                    break;
                                }

                                if stop_all {
                                    break;
                                }
                            }

                            if stop_all {
                                break;
                            }
                        }
                        api_used = got_api_used;
                        }
                    } else {
                        let (r, api) = run_startup_override_once(
                            &client,
                            &trigger_client,
                            base.as_str(),
                            app_type_str.as_str(),
                            *priority,
                            supplier.as_str(),
                            url.as_str(),
                            test_model,
                            "responses",
                            override_ttl_secs,
                            timeout_secs,
                            spawn_claude,
                            verbose_child,
                            &workdir,
                            &codex_hint,
                            codex_openai_beta,
                            0,
                            1,
                            1,
                            false,
                        )
                        .await;
                        got = r;
                        api_used = api;
                    }

                    if let Some(r) = got {
                        let request_model = r.request_model.as_deref().unwrap_or("unknown");
                        let effective_model = r.effective_model.as_deref().unwrap_or(request_model);
                        let request_model_disp =
                            sanitize_gpt_model_name_for_display(request_model);
                        let effective_model_disp =
                            sanitize_gpt_model_name_for_display(effective_model);
                        let model_display = if effective_model_disp != request_model_disp
                            && request_model_disp != "unknown"
                            && effective_model_disp != "unknown"
                        {
                            format!("{effective_model_disp} (req={request_model_disp})")
                        } else {
                            effective_model_disp
                        };
                        if let Some(u) = r.urls.first() {
                            per_url_results.insert(u.url.clone(), u.clone());
                            let api_suffix = api_used
                                .map(|s| format!(" api={}", s))
                                .unwrap_or_default();
                            match u.kind.as_str() {
                                "OK" => println!(
                                    "    - {} {} 全链路=OK {}ms (model={}{})",
                                    u.url,
                                    simple_part,
                                    u.latency_ms.unwrap_or(0),
                                    model_display.as_str(),
                                    api_suffix
                                ),
                                "OV" => println!(
                                    "    - {} {} 全链路=OV {}ms ({}) (model={}{})",
                                    u.url,
                                    simple_part,
                                    u.latency_ms.unwrap_or(0),
                                    u.message.as_deref().unwrap_or("-"),
                                    model_display.as_str(),
                                    api_suffix
                                ),
                                "FAIL" => println!(
                                    "    - {} {} 全链路=FAIL ({}) (model={}{})",
                                    u.url,
                                    simple_part,
                                    {
                                        let hint = codex_diag_hint
                                            .as_deref()
                                            .map(|h| format!("；{h}"))
                                            .unwrap_or_default();
                                        format!("{}{}", u.reason.as_deref().unwrap_or("-"), hint)
                                    },
                                    model_display.as_str(),
                                    api_suffix
                                ),
                                other => println!(
                                    "    - {} {} 全链路={} (model={}{})",
                                    u.url,
                                    simple_part,
                                    other,
                                    model_display.as_str(),
                                    api_suffix
                                ),
                            }
                        } else {
                            println!("    - {} {} 全链路=FAIL(结果为空)", url, simple_part);
                        }
                    } else {
                        println!(
                            "    - {} {} 全链路=FAIL(等待测速结果超时/未触发真实请求)",
                            url, simple_part
                        );
                    }
                }

                // 选用：优先 OK 最快；无 OK 则 OV；否则 FAIL
                let mut chosen: Option<(String, String, u64)> = None;
                for (u, r) in per_url_results.iter() {
                    let metric = match r.kind.as_str() {
                        "OK" => r.latency_ms.unwrap_or(u64::MAX),
                        "OV" => r
                            .latency_ms
                            .unwrap_or(u64::MAX)
                            .saturating_add(r.penalty_ms.unwrap_or(0)),
                        _ => u64::MAX,
                    };
                    let rank = match r.kind.as_str() {
                        "OK" => 0u8,
                        "OV" => 1u8,
                        _ => 2u8,
                    };

                    match chosen.as_ref() {
                        None => {
                            if rank < 2 {
                                chosen = Some((u.clone(), r.kind.clone(), metric));
                            }
                        }
                        Some((_cu, ckind, cmetric)) => {
                            let crank = if ckind == "OK" { 0u8 } else { 1u8 };
                            if rank < crank || (rank == crank && metric < *cmetric) {
                                if rank < 2 {
                                    chosen = Some((u.clone(), r.kind.clone(), metric));
                                }
                            }
                        }
                    }
                }

                if let Some((url, kind, metric)) = chosen {
                    println!("  选用: {} {} ({}ms)", kind, url, metric);
                    summary.push((*priority, supplier.clone(), url, kind, metric));
                } else {
                    println!("  选用: FAIL (未选出可用URL)");
                }
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
