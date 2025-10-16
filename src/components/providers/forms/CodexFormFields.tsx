import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import ApiKeyInput from "@/components/ProviderForm/ApiKeyInput";
import EndpointSpeedTest from "@/components/ProviderForm/EndpointSpeedTest";
import { Zap } from "lucide-react";
import type { ProviderCategory } from "@/types";

interface CodexFormFieldsProps {
  // API Key
  codexApiKey: string;
  onApiKeyChange: (key: string) => void;
  category?: ProviderCategory;
  shouldShowApiKeyLink: boolean;
  websiteUrl: string;

  // Base URL
  shouldShowSpeedTest: boolean;
  codexBaseUrl: string;
  onBaseUrlChange: (url: string) => void;
  isEndpointModalOpen: boolean;
  onEndpointModalToggle: (open: boolean) => void;
  onCustomEndpointsChange: (endpoints: string[]) => void;
}

export function CodexFormFields({
  codexApiKey,
  onApiKeyChange,
  category,
  shouldShowApiKeyLink,
  websiteUrl,
  shouldShowSpeedTest,
  codexBaseUrl,
  onBaseUrlChange,
  isEndpointModalOpen,
  onEndpointModalToggle,
  onCustomEndpointsChange,
}: CodexFormFieldsProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* Codex API Key 输入框 */}
      <div className="space-y-1">
        <ApiKeyInput
          id="codexApiKey"
          label="API Key"
          value={codexApiKey}
          onChange={onApiKeyChange}
          required={category !== "official"}
          placeholder={
            category === "official"
              ? t("providerForm.codexOfficialNoApiKey", {
                  defaultValue: "官方供应商无需 API Key",
                })
              : t("providerForm.codexApiKeyAutoFill", {
                  defaultValue: "输入 API Key，将自动填充到配置",
                })
          }
          disabled={category === "official"}
        />
        {/* Codex API Key 获取链接 */}
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

      {/* Codex Base URL 输入框 */}
      {shouldShowSpeedTest && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <FormLabel htmlFor="codexBaseUrl">
              {t("codexConfig.apiUrlLabel", { defaultValue: "API 端点" })}
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
            id="codexBaseUrl"
            type="url"
            value={codexBaseUrl}
            onChange={(e) => onBaseUrlChange(e.target.value)}
            placeholder={t("providerForm.codexApiEndpointPlaceholder", {
              defaultValue: "https://api.example.com/v1",
            })}
            autoComplete="off"
          />
          <div className="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg">
            <p className="text-xs text-amber-600 dark:text-amber-400">
              {t("providerForm.codexApiHint", {
                defaultValue: "Codex API 端点地址",
              })}
            </p>
          </div>
        </div>
      )}

      {/* 端点测速弹窗 - Codex */}
      {shouldShowSpeedTest && isEndpointModalOpen && (
        <EndpointSpeedTest
          appType="codex"
          value={codexBaseUrl}
          onChange={onBaseUrlChange}
          initialEndpoints={[{ url: codexBaseUrl }]}
          visible={isEndpointModalOpen}
          onClose={() => onEndpointModalToggle(false)}
          onCustomEndpointsChange={onCustomEndpointsChange}
        />
      )}
    </>
  );
}
