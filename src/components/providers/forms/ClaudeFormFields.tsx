import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import ApiKeyInput from "@/components/ProviderForm/ApiKeyInput";
import EndpointSpeedTest from "@/components/ProviderForm/EndpointSpeedTest";
import KimiModelSelector from "@/components/ProviderForm/KimiModelSelector";
import { Zap } from "lucide-react";
import type { ProviderCategory } from "@/types";
import type { TemplateValueConfig } from "@/config/providerPresets";

interface ClaudeFormFieldsProps {
  // API Key
  shouldShowApiKey: boolean;
  apiKey: string;
  onApiKeyChange: (key: string) => void;
  category?: ProviderCategory;
  shouldShowApiKeyLink: boolean;
  websiteUrl: string;

  // Template Values
  templateValueEntries: Array<[string, TemplateValueConfig]>;
  templateValues: Record<string, TemplateValueConfig>;
  templatePresetName: string;
  onTemplateValueChange: (key: string, value: string) => void;

  // Base URL
  shouldShowSpeedTest: boolean;
  baseUrl: string;
  onBaseUrlChange: (url: string) => void;
  isEndpointModalOpen: boolean;
  onEndpointModalToggle: (open: boolean) => void;
  onCustomEndpointsChange: (endpoints: string[]) => void;

  // Model Selector
  shouldShowKimiSelector: boolean;
  shouldShowModelSelector: boolean;
  claudeModel: string;
  claudeSmallFastModel: string;
  onModelChange: (
    field: "ANTHROPIC_MODEL" | "ANTHROPIC_SMALL_FAST_MODEL",
    value: string
  ) => void;

  // Kimi Model Selector
  kimiAnthropicModel: string;
  kimiAnthropicSmallFastModel: string;
  onKimiModelChange: (
    field: "ANTHROPIC_MODEL" | "ANTHROPIC_SMALL_FAST_MODEL",
    value: string
  ) => void;
}

export function ClaudeFormFields({
  shouldShowApiKey,
  apiKey,
  onApiKeyChange,
  category,
  shouldShowApiKeyLink,
  websiteUrl,
  templateValueEntries,
  templateValues,
  templatePresetName,
  onTemplateValueChange,
  shouldShowSpeedTest,
  baseUrl,
  onBaseUrlChange,
  isEndpointModalOpen,
  onEndpointModalToggle,
  onCustomEndpointsChange,
  shouldShowKimiSelector,
  shouldShowModelSelector,
  claudeModel,
  claudeSmallFastModel,
  onModelChange,
  kimiAnthropicModel,
  kimiAnthropicSmallFastModel,
  onKimiModelChange,
}: ClaudeFormFieldsProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* API Key 输入框 */}
      {shouldShowApiKey && (
        <div className="space-y-1">
          <ApiKeyInput
            value={apiKey}
            onChange={onApiKeyChange}
            required={category !== "official"}
            placeholder={
              category === "official"
                ? t("providerForm.officialNoApiKey", {
                    defaultValue: "官方供应商无需 API Key",
                  })
                : t("providerForm.apiKeyAutoFill", {
                    defaultValue: "输入 API Key，将自动填充到配置",
                  })
            }
            disabled={category === "official"}
          />
          {/* API Key 获取链接 */}
          {shouldShowApiKeyLink && websiteUrl && (
            <div className="-mt-1 pl-1">
              <a
                href={websiteUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="text-xs text-blue-400 dark:text-blue-500 hover:text-blue-500 dark:hover:text-blue-400 transition-colors"
              >
                {t("providerForm.getApiKey", {
                  defaultValue: "获取 API Key",
                })}
              </a>
            </div>
          )}
        </div>
      )}

      {/* 模板变量输入 */}
      {templateValueEntries.length > 0 && (
        <div className="space-y-3">
          <FormLabel>
            {t("providerForm.parameterConfig", {
              name: templatePresetName,
              defaultValue: `${templatePresetName} 参数配置`,
            })}
          </FormLabel>
          <div className="space-y-4">
            {templateValueEntries.map(([key, config]) => (
              <div key={key} className="space-y-2">
                <FormLabel htmlFor={`template-${key}`}>
                  {config.label}
                </FormLabel>
                <Input
                  id={`template-${key}`}
                  type="text"
                  required
                  value={
                    templateValues[key]?.editorValue ??
                    config.editorValue ??
                    config.defaultValue ??
                    ""
                  }
                  onChange={(e) => onTemplateValueChange(key, e.target.value)}
                  placeholder={config.placeholder || config.label}
                  autoComplete="off"
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Base URL 输入框 */}
      {shouldShowSpeedTest && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <FormLabel htmlFor="baseUrl">
              {t("providerForm.apiEndpoint", { defaultValue: "API 端点" })}
            </FormLabel>
            <button
              type="button"
              onClick={() => onEndpointModalToggle(true)}
              className="flex items-center gap-1 text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
            >
              <Zap className="h-3.5 w-3.5" />
              {t("providerForm.manageAndTest", {
                defaultValue: "管理和测速",
              })}
            </button>
          </div>
          <Input
            id="baseUrl"
            type="url"
            value={baseUrl}
            onChange={(e) => onBaseUrlChange(e.target.value)}
            placeholder={t("providerForm.apiEndpointPlaceholder", {
              defaultValue: "https://api.example.com",
            })}
            autoComplete="off"
          />
          <div className="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg">
            <p className="text-xs text-amber-600 dark:text-amber-400">
              {t("providerForm.apiHint", {
                defaultValue: "API 端点地址用于连接服务器",
              })}
            </p>
          </div>
        </div>
      )}

      {/* 端点测速弹窗 */}
      {shouldShowSpeedTest && isEndpointModalOpen && (
        <EndpointSpeedTest
          appType="claude"
          value={baseUrl}
          onChange={onBaseUrlChange}
          initialEndpoints={[{ url: baseUrl }]}
          visible={isEndpointModalOpen}
          onClose={() => onEndpointModalToggle(false)}
          onCustomEndpointsChange={onCustomEndpointsChange}
        />
      )}

      {/* 模型选择器 */}
      {shouldShowModelSelector && (
        <div className="space-y-3">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* ANTHROPIC_MODEL */}
            <div className="space-y-2">
              <FormLabel htmlFor="claudeModel">
                {t("providerForm.anthropicModel", {
                  defaultValue: "主模型",
                })}
              </FormLabel>
              <Input
                id="claudeModel"
                type="text"
                value={claudeModel}
                onChange={(e) =>
                  onModelChange("ANTHROPIC_MODEL", e.target.value)
                }
                placeholder={t("providerForm.modelPlaceholder", {
                  defaultValue: "claude-3-7-sonnet-20250219",
                })}
                autoComplete="off"
              />
            </div>

            {/* ANTHROPIC_SMALL_FAST_MODEL */}
            <div className="space-y-2">
              <FormLabel htmlFor="claudeSmallFastModel">
                {t("providerForm.anthropicSmallFastModel", {
                  defaultValue: "快速模型",
                })}
              </FormLabel>
              <Input
                id="claudeSmallFastModel"
                type="text"
                value={claudeSmallFastModel}
                onChange={(e) =>
                  onModelChange("ANTHROPIC_SMALL_FAST_MODEL", e.target.value)
                }
                placeholder={t("providerForm.smallModelPlaceholder", {
                  defaultValue: "claude-3-5-haiku-20241022",
                })}
                autoComplete="off"
              />
            </div>
          </div>
          <p className="text-xs text-muted-foreground">
            {t("providerForm.modelHelper", {
              defaultValue:
                "可选：指定默认使用的 Claude 模型，留空则使用系统默认。",
            })}
          </p>
        </div>
      )}

      {/* Kimi 模型选择器 */}
      {shouldShowKimiSelector && (
        <KimiModelSelector
          apiKey={apiKey}
          anthropicModel={kimiAnthropicModel}
          anthropicSmallFastModel={kimiAnthropicSmallFastModel}
          onModelChange={onKimiModelChange}
          disabled={category === "official"}
        />
      )}
    </>
  );
}
