import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Wand2 } from "lucide-react";
import { toast } from "sonner";

interface GeminiConfigEditorProps {
  value: string;
  onChange: (value: string) => void;
}

export function GeminiConfigEditor({
  value,
  onChange,
}: GeminiConfigEditorProps) {
  const { t } = useTranslation();

  // 将 JSON 格式转换为 .env 格式显示
  const jsonToEnv = (jsonString: string): string => {
    try {
      const config = JSON.parse(jsonString);
      const env = config?.env || {};

      const lines: string[] = [];
      if (env.GOOGLE_GEMINI_BASE_URL) {
        lines.push(`GOOGLE_GEMINI_BASE_URL=${env.GOOGLE_GEMINI_BASE_URL}`);
      }
      if (env.GEMINI_API_KEY) {
        lines.push(`GEMINI_API_KEY=${env.GEMINI_API_KEY}`);
      }
      if (env.GEMINI_MODEL) {
        lines.push(`GEMINI_MODEL=${env.GEMINI_MODEL}`);
      }

      return lines.join("\n");
    } catch {
      return "";
    }
  };

  // 将 .env 格式转换为 JSON 格式保存
  const envToJson = (envString: string): string => {
    try {
      const lines = envString.split("\n");
      const env: Record<string, string> = {};

      lines.forEach((line) => {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith("#")) return;

        const equalIndex = trimmed.indexOf("=");
        if (equalIndex > 0) {
          const key = trimmed.substring(0, equalIndex).trim();
          const value = trimmed.substring(equalIndex + 1).trim();
          env[key] = value;
        }
      });

      return JSON.stringify({ env }, null, 2);
    } catch {
      return value;
    }
  };

  const displayValue = jsonToEnv(value);

  const handleChange = (envString: string) => {
    const jsonString = envToJson(envString);
    onChange(jsonString);
  };

  const handleFormat = () => {
    if (!value.trim()) return;

    try {
      // 重新格式化
      const envString = jsonToEnv(value);
      const formatted = envString
        .split("\n")
        .filter((l) => l.trim())
        .join("\n");
      const jsonString = envToJson(formatted);
      onChange(jsonString);
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
        <Label htmlFor="geminiConfig">
          {t("provider.geminiConfig", { defaultValue: "Gemini 配置" })}
        </Label>
      </div>
      <textarea
        id="geminiConfig"
        value={displayValue}
        onChange={(e) => handleChange(e.target.value)}
        placeholder={`GOOGLE_GEMINI_BASE_URL=https://your-api-endpoint.com/
GEMINI_API_KEY=sk-your-api-key-here
GEMINI_MODEL=gemini-2.5-pro`}
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
        <p className="text-xs text-muted-foreground">
          {t("provider.geminiConfigHint", {
            defaultValue: "使用 .env 格式配置 Gemini",
          })}
        </p>
      </div>
    </div>
  );
}
