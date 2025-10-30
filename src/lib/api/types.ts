export type AppType = "claude" | "codex";
// 为避免与后端 Rust `AppType` 枚举语义混淆，可使用更贴近“标识符”的别名
export type AppId = AppType;
