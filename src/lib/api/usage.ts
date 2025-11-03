import { invoke } from "@tauri-apps/api/core";
import type { UsageResult } from "@/types";
import type { AppId } from "./types";
import i18n from "@/i18n";

export const usageApi = {
  async query(providerId: string, appId: AppId): Promise<UsageResult> {
    try {
      return await invoke("query_provider_usage", {
        provider_id: providerId,
        app: appId,
      });
    } catch (error: unknown) {
      // 提取错误消息：优先使用后端返回的错误信息
      const message =
        typeof error === "string"
          ? error
          : error instanceof Error
            ? error.message
            : "";

      // 如果没有错误消息，使用国际化的默认提示
      return {
        success: false,
        error: message || i18n.t("errors.usage_query_failed"),
      };
    }
  },
};
