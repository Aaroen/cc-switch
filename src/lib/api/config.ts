// 配置相关 API
import { invoke } from "@tauri-apps/api/core";

/**
 * 获取 Claude 通用配置片段
 * @returns 通用配置片段（JSON 字符串），如果不存在则返回 null
 */
export async function getClaudeCommonConfigSnippet(): Promise<string | null> {
  return invoke<string | null>("get_claude_common_config_snippet");
}

/**
 * 设置 Claude 通用配置片段
 * @param snippet - 通用配置片段（JSON 字符串）
 * @throws 如果 JSON 格式无效
 */
export async function setClaudeCommonConfigSnippet(
  snippet: string,
): Promise<void> {
  return invoke("set_claude_common_config_snippet", { snippet });
}
