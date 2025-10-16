import { invoke } from "@tauri-apps/api/core";
import type {
  McpConfigResponse,
  McpServer,
  McpServerSpec,
  McpStatus,
} from "@/types";
import type { AppType } from "./types";

export const mcpApi = {
  async getStatus(): Promise<McpStatus> {
    return await invoke("get_claude_mcp_status");
  },

  async readConfig(): Promise<string | null> {
    return await invoke("read_claude_mcp_config");
  },

  async upsertServer(
    id: string,
    spec: McpServerSpec | Record<string, any>
  ): Promise<boolean> {
    return await invoke("upsert_claude_mcp_server", { id, spec });
  },

  async deleteServer(id: string): Promise<boolean> {
    return await invoke("delete_claude_mcp_server", { id });
  },

  async validateCommand(cmd: string): Promise<boolean> {
    return await invoke("validate_mcp_command", { cmd });
  },

  async getConfig(app: AppType = "claude"): Promise<McpConfigResponse> {
    return await invoke("get_mcp_config", { app });
  },

  async upsertServerInConfig(
    app: AppType,
    id: string,
    spec: McpServer,
    options?: { syncOtherSide?: boolean }
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
    app: AppType,
    id: string,
    options?: { syncOtherSide?: boolean }
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
};
