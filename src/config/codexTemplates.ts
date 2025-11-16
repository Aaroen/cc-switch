/**
 * Codex 配置模板
 * 用于新建自定义供应商时的默认配置
 */

export interface CodexTemplate {
  auth: Record<string, any>;
  config: string;
}

/**
 * 获取 Codex 自定义模板
 * @param locale 语言环境 ('zh' | 'en')
 * @returns Codex 模板配置
 */
export function getCodexCustomTemplate(
  locale: "zh" | "en" = "zh",
): CodexTemplate {
  const templates = {
    zh: `# ========================================
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

    en: `# ========================================
# Codex Custom Provider Configuration Template
# ========================================
# Quick Start:
# 1. Set API Key in auth.json above
# 2. Replace 'custom' below with provider name (lowercase, no spaces)
# 3. Replace base_url with actual API endpoint
# 4. Adjust model name as needed
#
# Docs: https://docs.anthropic.com
# ========================================

# ========== Model Configuration ==========
model_provider = "custom"        # Unique provider identifier
model = "gpt-5-codex"            # Model name
model_reasoning_effort = "high"  # Reasoning effort: low, medium, high
disable_response_storage = true  # Privacy: do not store responses locally

# ========== Provider Settings ==========
[model_providers.custom]
name = "custom"                                    # Must match model_provider above
base_url = "https://api.example.com/v1"           # 👈 Replace with actual endpoint
wire_api = "responses"                            # API response format
requires_openai_auth = true                       # Use OPENAI_API_KEY from auth.json

# ========== Optional: Custom Headers ==========
# If provider requires custom headers, uncomment:
# [model_providers.custom.headers]
# X-Custom-Header = "value"

# ========== Optional: Model Overrides ==========
# If you need to override specific models, uncomment:
# [model_overrides]
# "gpt-5-codex" = { model_provider = "custom", model = "your-model-name" }`,
  };

  return {
    auth: { OPENAI_API_KEY: "" },
    config: templates[locale] || templates.zh,
  };
}
