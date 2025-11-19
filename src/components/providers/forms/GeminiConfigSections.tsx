import React from "react";
import { useTranslation } from "react-i18next";
import { Wand2 } from "lucide-react";
import { toast } from "sonner";
import { formatJSON } from "@/utils/formatters";

interface GeminiEnvSectionProps {
  value: string;
  onChange: (value: string) => void;
  onBlur?: () => void;
  error?: string;
}

/**
 * GeminiEnvSection - .env editor section for Gemini environment variables
 */
export const GeminiEnvSection: React.FC<GeminiEnvSectionProps> = ({
  value,
  onChange,
  onBlur,
  error,
}) => {
  const { t } = useTranslation();

  const handleFormat = () => {
    if (!value.trim()) return;

    try {
      // 重新格式化 .env 内容
      const formatted = value
        .split("\n")
        .filter((line) => line.trim())
        .join("\n");
      onChange(formatted);
      toast.success(t("common.formatSuccess", { defaultValue: "格式化成功" }));
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      toast.error(
        t("common.formatError", {
          defaultValue: "格式化失败：{{error}}",
          error: errorMessage,
        }),
      );
    }
  };

  return (
    <div className="space-y-2">
      <label
        htmlFor="geminiEnv"
        className="block text-sm font-medium text-gray-900 dark:text-gray-100"
      >
        {t("geminiConfig.envFile", { defaultValue: "环境变量 (.env)" })}
      </label>

      <textarea
        id="geminiEnv"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        placeholder={`GOOGLE_GEMINI_BASE_URL=https://your-api-endpoint.com/
GEMINI_API_KEY=sk-your-api-key-here
GEMINI_MODEL=gemini-3-pro-preview`}
        rows={6}
        className="w-full px-3 py-2 border border-border-default dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 transition-colors resize-y min-h-[8rem]"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="none"
        spellCheck={false}
        lang="en"
        inputMode="text"
        data-gramm="false"
        data-gramm_editor="false"
        data-enable-grammarly="false"
      />

      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={handleFormat}
          className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
        >
          <Wand2 className="w-3.5 h-3.5" />
          {t("common.format", { defaultValue: "格式化" })}
        </button>

        {error && (
          <p className="text-xs text-red-500 dark:text-red-400">{error}</p>
        )}
      </div>

      {!error && (
        <p className="text-xs text-gray-500 dark:text-gray-400">
          {t("geminiConfig.envFileHint", {
            defaultValue: "使用 .env 格式配置 Gemini 环境变量",
          })}
        </p>
      )}
    </div>
  );
};

interface GeminiConfigSectionProps {
  value: string;
  onChange: (value: string) => void;
  useCommonConfig: boolean;
  onCommonConfigToggle: (checked: boolean) => void;
  onEditCommonConfig: () => void;
  commonConfigError?: string;
  configError?: string;
}

/**
 * GeminiConfigSection - Config JSON editor section with common config support
 */
export const GeminiConfigSection: React.FC<GeminiConfigSectionProps> = ({
  value,
  onChange,
  useCommonConfig,
  onCommonConfigToggle,
  onEditCommonConfig,
  commonConfigError,
  configError,
}) => {
  const { t } = useTranslation();

  const handleFormat = () => {
    if (!value.trim()) return;

    try {
      const formatted = formatJSON(value);
      onChange(formatted);
      toast.success(t("common.formatSuccess", { defaultValue: "格式化成功" }));
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      toast.error(
        t("common.formatError", {
          defaultValue: "格式化失败：{{error}}",
          error: errorMessage,
        }),
      );
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label
          htmlFor="geminiConfig"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          {t("geminiConfig.configJson", {
            defaultValue: "配置文件 (config.json)",
          })}
        </label>

        <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={useCommonConfig}
            onChange={(e) => onCommonConfigToggle(e.target.checked)}
            className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-border-default rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
          />
          {t("geminiConfig.writeCommonConfig", {
            defaultValue: "写入通用配置",
          })}
        </label>
      </div>

      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={onEditCommonConfig}
          className="text-xs text-blue-500 dark:text-blue-400 hover:underline"
        >
          {t("geminiConfig.editCommonConfig", {
            defaultValue: "编辑通用配置",
          })}
        </button>
      </div>

      {commonConfigError && (
        <p className="text-xs text-red-500 dark:text-red-400 text-right">
          {commonConfigError}
        </p>
      )}

      <textarea
        id="geminiConfig"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={`{
  "timeout": 30000,
  "maxRetries": 3
}`}
        rows={8}
        className="w-full px-3 py-2 border border-border-default dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 transition-colors resize-y min-h-[10rem]"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="none"
        spellCheck={false}
        lang="en"
        inputMode="text"
        data-gramm="false"
        data-gramm_editor="false"
        data-enable-grammarly="false"
      />

      <div className="flex items-center justify-between">
        <button
          type="button"
          onClick={handleFormat}
          className="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400 transition-colors"
        >
          <Wand2 className="w-3.5 h-3.5" />
          {t("common.format", { defaultValue: "格式化" })}
        </button>

        {configError && (
          <p className="text-xs text-red-500 dark:text-red-400">
            {configError}
          </p>
        )}
      </div>

      {!configError && (
        <p className="text-xs text-gray-500 dark:text-gray-400">
          {t("geminiConfig.configJsonHint", {
            defaultValue: "使用 JSON 格式配置 Gemini 扩展参数（可选）",
          })}
        </p>
      )}
    </div>
  );
};
