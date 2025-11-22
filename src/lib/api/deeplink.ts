import { invoke } from "@tauri-apps/api/core";

export interface DeepLinkImportRequest {
  version: string;
  resource: string;
  app: "claude" | "codex" | "gemini";
  name: string;
  homepage: string;
  endpoint: string;
  apiKey: string;
  model?: string;
  notes?: string;
  // Claude 专用模型字段 (v3.7.1+)
  haikuModel?: string;
  sonnetModel?: string;
  opusModel?: string;
  // 配置文件导入字段 (v3.8+)
  config?: string; // Base64 编码的配置内容
  configFormat?: string; // json/toml
  configUrl?: string; // 远程配置 URL
}

export const deeplinkApi = {
  /**
   * Parse a deep link URL
   * @param url The ccswitch:// URL to parse
   * @returns Parsed deep link request
   */
  parseDeeplink: async (url: string): Promise<DeepLinkImportRequest> => {
    return invoke("parse_deeplink", { url });
  },

  /**
   * Merge configuration from Base64/URL into a deep link request
   * This is used to show the complete configuration in the confirmation dialog
   * @param request The deep link import request
   * @returns Merged deep link request with config fields populated
   */
  mergeDeeplinkConfig: async (
    request: DeepLinkImportRequest,
  ): Promise<DeepLinkImportRequest> => {
    return invoke("merge_deeplink_config", { request });
  },

  /**
   * Import a provider from a deep link request
   * @param request The deep link import request
   * @returns The ID of the imported provider
   */
  importFromDeeplink: async (
    request: DeepLinkImportRequest,
  ): Promise<string> => {
    return invoke("import_from_deeplink", { request });
  },
};
