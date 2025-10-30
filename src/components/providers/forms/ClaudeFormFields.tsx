import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import EndpointSpeedTest from "./EndpointSpeedTest";
import KimiModelSelector from "./KimiModelSelector";
import { ApiKeySection, EndpointField } from "./shared";
import type { ProviderCategory } from "@/types";
import type { TemplateValueConfig } from "@/config/providerPresets";

interface EndpointCandidate {
  url: string;
}

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
    value: string,
  ) => void;

  // Kimi Model Selector
  kimiAnthropicModel: string;
  kimiAnthropicSmallFastModel: string;
  onKimiModelChange: (
    field: "ANTHROPIC_MODEL" | "ANTHROPIC_SMALL_FAST_MODEL",
    value: string,
  ) => void;

  // Speed Test Endpoints
  speedTestEndpoints: EndpointCandidate[];
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
  speedTestEndpoints,
}: ClaudeFormFieldsProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* API Key 输入框 */}
      {shouldShowApiKey && (
        <ApiKeySection
          value={apiKey}
          onChange={onApiKeyChange}
          category={category}
          shouldShowLink={shouldShowApiKeyLink}
          websiteUrl={websiteUrl}
        />
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
        <EndpointField
          id="baseUrl"
          label={t("providerForm.apiEndpoint")}
          value={baseUrl}
          onChange={onBaseUrlChange}
          placeholder={t("providerForm.apiEndpointPlaceholder")}
          hint={t("providerForm.apiHint")}
          onManageClick={() => onEndpointModalToggle(true)}
        />
      )}

      {/* 端点测速弹窗 */}
      {shouldShowSpeedTest && isEndpointModalOpen && (
        <EndpointSpeedTest
          appId="claude"
          value={baseUrl}
          onChange={onBaseUrlChange}
          initialEndpoints={speedTestEndpoints}
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
