import { invoke } from "@tauri-apps/api/core";
import type {
  McpConfigResponse,
  McpServer,
  McpServerSpec,
  McpServersMap,
  McpStatus,
} from "@/types";
import type { AppId } from "./types";

export const mcpApi = {
  async getStatus(): Promise<McpStatus> {
    return await invoke("get_claude_mcp_status");
  },

  async readConfig(): Promise<string | null> {
    return await invoke("read_claude_mcp_config");
  },

  async upsertServer(
    id: string,
    spec: McpServerSpec | Record<string, any>,
  ): Promise<boolean> {
    return await invoke("upsert_claude_mcp_server", { id, spec });
  },

  async deleteServer(id: string): Promise<boolean> {
    return await invoke("delete_claude_mcp_server", { id });
  },

  async validateCommand(cmd: string): Promise<boolean> {
    return await invoke("validate_mcp_command", { cmd });
  },

  async getConfig(app: AppId = "claude"): Promise<McpConfigResponse> {
    return await invoke("get_mcp_config", { app });
  },

  async importFromClaude(): Promise<number> {
    return await invoke("import_mcp_from_claude");
  },

  async importFromCodex(): Promise<number> {
    return await invoke("import_mcp_from_codex");
  },

  async upsertServerInConfig(
    app: AppId,
    id: string,
    spec: McpServer,
    options?: { syncOtherSide?: boolean },
  ): Promise<boolean> {
    const payload = {
      app,
      id,
      spec,
      ...(options?.syncOtherSide !== undefined
        ? { syncOtherSide: options.syncOtherSide }
        : {}),
    };
    return await invoke("upsert_mcp_server_in_config", payload);
  },

  async deleteServerInConfig(
    app: AppId,
    id: string,
    options?: { syncOtherSide?: boolean },
  ): Promise<boolean> {
    const payload = {
      app,
      id,
      ...(options?.syncOtherSide !== undefined
        ? { syncOtherSide: options.syncOtherSide }
        : {}),
    };
    return await invoke("delete_mcp_server_in_config", payload);
  },

  async setEnabled(app: AppId, id: string, enabled: boolean): Promise<boolean> {
    return await invoke("set_mcp_enabled", { app, id, enabled });
  },

  async syncEnabledToClaude(): Promise<boolean> {
    return await invoke("sync_enabled_mcp_to_claude");
  },

  async syncEnabledToCodex(): Promise<boolean> {
    return await invoke("sync_enabled_mcp_to_codex");
  },

  async syncEnabledToGemini(): Promise<boolean> {
    return await invoke("sync_enabled_mcp_to_gemini");
  },

  async importFromGemini(): Promise<number> {
    return await invoke("import_mcp_from_gemini");
  },

  // ========================================================================
  // v3.7.0 新增：统一 MCP 管理 API
  // ========================================================================

  /**
   * 获取所有 MCP 服务器（统一结构）
   */
  async getAllServers(): Promise<McpServersMap> {
    return await invoke("get_mcp_servers");
  },

  /**
   * 添加或更新 MCP 服务器（统一结构）
   */
  async upsertUnifiedServer(server: McpServer): Promise<void> {
    return await invoke("upsert_mcp_server", { server });
  },

  /**
   * 删除 MCP 服务器
   */
  async deleteUnifiedServer(id: string): Promise<boolean> {
    return await invoke("delete_mcp_server", { id });
  },

  /**
   * 切换 MCP 服务器在指定应用的启用状态
   */
  async toggleApp(
    serverId: string,
    app: AppId,
    enabled: boolean,
  ): Promise<void> {
    return await invoke("toggle_mcp_app", { serverId, app, enabled });
  },

  /**
   * 手动同步所有启用的 MCP 服务器到对应的应用
   */
  async syncAllServers(): Promise<void> {
    return await invoke("sync_all_mcp_servers");
  },
};
