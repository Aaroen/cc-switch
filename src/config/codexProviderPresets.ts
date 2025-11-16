/**
 * Codex 预设供应商配置模板
 */
import { ProviderCategory } from "../types";
import type { PresetTheme } from "./claudeProviderPresets";

export interface CodexProviderPreset {
  name: string;
  websiteUrl: string;
  // 第三方供应商可提供单独的获取 API Key 链接
  apiKeyUrl?: string;
  auth: Record<string, any>; // 将写入 ~/.codex/auth.json
  config: string; // 将写入 ~/.codex/config.toml（TOML 字符串）
  isOfficial?: boolean; // 标识是否为官方预设
  isPartner?: boolean; // 标识是否为商业合作伙伴
  partnerPromotionKey?: string; // 合作伙伴促销信息的 i18n key
  category?: ProviderCategory; // 新增：分类
  isCustomTemplate?: boolean; // 标识是否为自定义模板
  // 新增：请求地址候选列表（用于地址管理/测速）
  endpointCandidates?: string[];
  // 新增：视觉主题配置
  theme?: PresetTheme;
}

/**
 * 生成第三方供应商的 auth.json
 */
export function generateThirdPartyAuth(apiKey: string): Record<string, any> {
  return {
    OPENAI_API_KEY: apiKey || "",
  };
}

/**
 * 生成第三方供应商的 config.toml
 */
export function generateThirdPartyConfig(
  providerName: string,
  baseUrl: string,
  modelName = "gpt-5-codex",
): string {
  // 清理供应商名称，确保符合TOML键名规范
  const cleanProviderName =
    providerName
      .toLowerCase()
      .replace(/[^a-z0-9_]/g, "_")
      .replace(/^_+|_+$/g, "") || "custom";

  return `model_provider = "${cleanProviderName}"
model = "${modelName}"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.${cleanProviderName}]
name = "${cleanProviderName}"
base_url = "${baseUrl}"
wire_api = "responses"
requires_openai_auth = true`;
}

export const codexProviderPresets: CodexProviderPreset[] = [
  {
    name: "OpenAI Official",
    websiteUrl: "https://chatgpt.com/codex",
    isOfficial: true,
    category: "official",
    auth: {},
    config: ``,
    theme: {
      icon: "codex",
      backgroundColor: "#1F2937", // gray-800
      textColor: "#FFFFFF",
    },
  },
  {
    name: "Azure OpenAI",
    websiteUrl:
      "https://learn.microsoft.com/azure/ai-services/openai/how-to/overview",
    category: "third_party",
    isOfficial: true,
    auth: generateThirdPartyAuth(""),
    config: `model_provider = "azure"
model = "gpt-5-codex"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.azure]
name = "Azure OpenAI"
base_url = "https://YOUR_RESOURCE_NAME.openai.azure.com/openai"
env_key = "OPENAI_API_KEY"
query_params = { "api-version" = "2025-04-01-preview" }
wire_api = "responses"
requires_openai_auth = true`,
    endpointCandidates: ["https://YOUR_RESOURCE_NAME.openai.azure.com/openai"],
    theme: {
      icon: "codex",
      backgroundColor: "#0078D4",
      textColor: "#FFFFFF",
    },
  },
  {
    name: "Custom (Blank Template)",
    websiteUrl: "https://docs.anthropic.com",
    category: "third_party",
    isCustomTemplate: true,
    auth: generateThirdPartyAuth(""),
    config: `# ========================================
# Codex 自定义供应商配置模板
# ========================================
# 快速上手：
# 1. 在上方 auth.json 中设置 API Key
# 2. 将下方 'custom' 替换为供应商名称（小写、无空格）
# 3. 替换 base_url 为实际的 API 端点
# 4. 根据需要调整模型名称
#
# 文档: https://docs.anthropic.com
# ========================================

# ========== 模型配置 ==========
model_provider = "custom"        # 供应商唯一标识
model = "gpt-5-codex"            # 模型名称
model_reasoning_effort = "high"  # 推理强度：low, medium, high
disable_response_storage = true  # 隐私：不本地存储响应

# ========== 供应商设置 ==========
[model_providers.custom]
name = "custom"                                    # 与上方 model_provider 保持一致
base_url = "https://api.example.com/v1"           # 👈 替换为实际端点
wire_api = "responses"                            # API 响应格式
requires_openai_auth = true                       # 使用 auth.json 中的 OPENAI_API_KEY

# ========== 可选：自定义请求头 ==========
# 如果供应商需要自定义请求头，取消注释：
# [model_providers.custom.headers]
# X-Custom-Header = "value"

# ========== 可选：模型覆盖 ==========
# 如果需要覆盖特定模型，取消注释：
# [model_overrides]
# "gpt-5-codex" = { model_provider = "custom", model = "your-model-name" }`,
    theme: {
      icon: "generic",
      backgroundColor: "#6B7280", // gray-500
      textColor: "#FFFFFF",
    },
  },
  {
    name: "AiHubMix",
    websiteUrl: "https://aihubmix.com",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "aihubmix",
      "https://aihubmix.com/v1",
      "gpt-5-codex",
    ),
    endpointCandidates: [
      "https://aihubmix.com/v1",
      "https://api.aihubmix.com/v1",
    ],
  },
  {
    name: "DMXAPI",
    websiteUrl: "https://www.dmxapi.cn",
    category: "aggregator",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "dmxapi",
      "https://www.dmxapi.cn/v1",
      "gpt-5-codex",
    ),
    endpointCandidates: ["https://www.dmxapi.cn/v1"],
  },
  {
    name: "PackyCode",
    websiteUrl: "https://www.packyapi.com",
    apiKeyUrl: "https://www.packyapi.com/register?aff=cc-switch",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "packycode",
      "https://www.packyapi.com/v1",
      "gpt-5-codex",
    ),
    endpointCandidates: [
      "https://www.packyapi.com/v1",
      "https://api-slb.packyapi.com/v1",
    ],
    isPartner: true, // 合作伙伴
    partnerPromotionKey: "packycode", // 促销信息 i18n key
  },
  {
    name: "AnyRouter",
    websiteUrl: "https://anyrouter.top",
    category: "third_party",
    auth: generateThirdPartyAuth(""),
    config: generateThirdPartyConfig(
      "anyrouter",
      "https://anyrouter.top/v1",
      "gpt-5-codex",
    ),
    endpointCandidates: [
      "https://anyrouter.top/v1",
      "https://q.quuvv.cn/v1",
      "https://pmpjfbhq.cn-nb1.rainapp.top/v1",
    ],
  },
];
