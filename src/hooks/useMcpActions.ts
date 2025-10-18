import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { mcpApi, type AppType } from "@/lib/api";
import type { McpServer } from "@/types";
import {
  extractErrorMessage,
  translateMcpBackendError,
} from "@/utils/errorUtils";

export interface UseMcpActionsResult {
  servers: Record<string, McpServer>;
  loading: boolean;
  reload: () => Promise<void>;
  toggleEnabled: (id: string, enabled: boolean) => Promise<void>;
  saveServer: (
    id: string,
    server: McpServer,
    options?: { syncOtherSide?: boolean },
  ) => Promise<void>;
  deleteServer: (id: string) => Promise<void>;
}

/**
 * useMcpActions - MCP management business logic
 * Responsibilities:
 * - Load MCP servers
 * - Toggle enable/disable status
 * - Save server configuration
 * - Delete server
 * - Error handling and toast notifications
 */
export function useMcpActions(appType: AppType): UseMcpActionsResult {
  const { t } = useTranslation();
  const [servers, setServers] = useState<Record<string, McpServer>>({});
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const cfg = await mcpApi.getConfig(appType);
      setServers(cfg.servers || {});
    } catch (error) {
      console.error("[useMcpActions] Failed to load MCP config", error);
      const detail = extractErrorMessage(error);
      const mapped = translateMcpBackendError(detail, t);
      toast.error(mapped || detail || t("mcp.error.loadFailed"), {
        duration: mapped || detail ? 6000 : 5000,
      });
    } finally {
      setLoading(false);
    }
  }, [appType, t]);

  const toggleEnabled = useCallback(
    async (id: string, enabled: boolean) => {
      // Optimistic update
      const previousServers = servers;
      setServers((prev) => ({
        ...prev,
        [id]: {
          ...prev[id],
          enabled,
        },
      }));

      try {
        await mcpApi.setEnabled(appType, id, enabled);
        toast.success(enabled ? t("mcp.msg.enabled") : t("mcp.msg.disabled"), {
          duration: 1500,
        });
      } catch (error) {
        // Rollback on failure
        setServers(previousServers);
        const detail = extractErrorMessage(error);
        const mapped = translateMcpBackendError(detail, t);
        toast.error(mapped || detail || t("mcp.error.saveFailed"), {
          duration: mapped || detail ? 6000 : 5000,
        });
      }
    },
    [appType, servers, t],
  );

  const saveServer = useCallback(
    async (
      id: string,
      server: McpServer,
      options?: { syncOtherSide?: boolean },
    ) => {
      try {
        const payload: McpServer = { ...server, id };
        await mcpApi.upsertServerInConfig(appType, id, payload, {
          syncOtherSide: options?.syncOtherSide,
        });
        await reload();
        toast.success(t("mcp.msg.saved"), { duration: 1500 });
      } catch (error) {
        const detail = extractErrorMessage(error);
        const mapped = translateMcpBackendError(detail, t);
        const msg = mapped || detail || t("mcp.error.saveFailed");
        toast.error(msg, { duration: mapped || detail ? 6000 : 5000 });
        // Re-throw to allow form-level error handling
        throw error;
      }
    },
    [appType, reload, t],
  );

  const deleteServer = useCallback(
    async (id: string) => {
      try {
        await mcpApi.deleteServerInConfig(appType, id);
        await reload();
        toast.success(t("mcp.msg.deleted"), { duration: 1500 });
      } catch (error) {
        const detail = extractErrorMessage(error);
        const mapped = translateMcpBackendError(detail, t);
        toast.error(mapped || detail || t("mcp.error.deleteFailed"), {
          duration: mapped || detail ? 6000 : 5000,
        });
        throw error;
      }
    },
    [appType, reload, t],
  );

  return {
    servers,
    loading,
    reload,
    toggleEnabled,
    saveServer,
    deleteServer,
  };
}
