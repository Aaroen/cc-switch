import React, { useState } from "react";
import { Play, Wand2 } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { Provider, UsageScript } from "@/types";
import { usageApi, type AppId } from "@/lib/api";
import JsonEditor from "./JsonEditor";
import * as prettier from "prettier/standalone";
import * as parserBabel from "prettier/parser-babel";
import * as pluginEstree from "prettier/plugins/estree";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface UsageScriptModalProps {
  provider: Provider;
  appId: AppId;
  isOpen: boolean;
  onClose: () => void;
  onSave: (script: UsageScript) => void;
}

// 预设模板键名（用于国际化）
const TEMPLATE_KEYS = {
  CUSTOM: "custom",
  GENERAL: "general",
  NEW_API: "newapi",
} as const;

// 生成预设模板的函数（支持国际化）
const generatePresetTemplates = (t: (key: string) => string): Record<string, string> => ({
  [TEMPLATE_KEYS.CUSTOM]: `({
  request: {
    url: "",
    method: "GET",
    headers: {}
  },
  extractor: function(response) {
    return {
      remaining: 0,
      unit: "USD"
    };
  }
})`,

  [TEMPLATE_KEYS.GENERAL]: `({
  request: {
    url: "{{baseUrl}}/user/balance",
    method: "GET",
    headers: {
      "Authorization": "Bearer {{apiKey}}",
      "User-Agent": "cc-switch/1.0"
    }
  },
  extractor: function(response) {
    return {
      isValid: response.is_active || true,
      remaining: response.balance,
      unit: "USD"
    };
  }
})`,

  [TEMPLATE_KEYS.NEW_API]: `({
  request: {
    url: "{{baseUrl}}/api/user/self",
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      "Authorization": "Bearer {{accessToken}}",
      "New-Api-User": "{{userId}}"
    },
  },
  extractor: function (response) {
    if (response.success && response.data) {
      return {
        planName: response.data.group || "${t("usageScript.defaultPlan")}",
        remaining: response.data.quota / 500000,
        used: response.data.used_quota / 500000,
        total: (response.data.quota + response.data.used_quota) / 500000,
        unit: "USD",
      };
    }
    return {
      isValid: false,
      invalidMessage: response.message || "${t("usageScript.queryFailedMessage")}"
    };
  },
})`,
});

// 模板名称国际化键映射
const TEMPLATE_NAME_KEYS: Record<string, string> = {
  [TEMPLATE_KEYS.CUSTOM]: "usageScript.templateCustom",
  [TEMPLATE_KEYS.GENERAL]: "usageScript.templateGeneral",
  [TEMPLATE_KEYS.NEW_API]: "usageScript.templateNewAPI",
};

const UsageScriptModal: React.FC<UsageScriptModalProps> = ({
  provider,
  appId,
  isOpen,
  onClose,
  onSave,
}) => {
  const { t } = useTranslation();

  // 生成带国际化的预设模板
  const PRESET_TEMPLATES = generatePresetTemplates(t);

  const [script, setScript] = useState<UsageScript>(() => {
    return (
      provider.meta?.usage_script || {
        enabled: false,
        language: "javascript",
        code: PRESET_TEMPLATES[TEMPLATE_KEYS.GENERAL],
        timeout: 10,
      }
    );
  });

  const [testing, setTesting] = useState(false);

  // 跟踪当前选择的模板类型（用于控制高级配置的显示）
  // 初始化：如果已有 accessToken 或 userId，说明是 NewAPI 模板
  const [selectedTemplate, setSelectedTemplate] = useState<string | null>(
    () => {
      const existingScript = provider.meta?.usage_script;
      if (existingScript?.accessToken || existingScript?.userId) {
        return TEMPLATE_KEYS.NEW_API;
      }
      return null;
    }
  );

  const handleSave = () => {
    // 验证脚本格式
    if (script.enabled && !script.code.trim()) {
      toast.error(t("usageScript.scriptEmpty"));
      return;
    }

    // 基本的 JS 语法检查（检查是否包含 return 语句）
    if (script.enabled && !script.code.includes("return")) {
      toast.error(t("usageScript.mustHaveReturn"), { duration: 5000 });
      return;
    }

    onSave(script);
    onClose();
  };

  const handleTest = async () => {
    setTesting(true);
    try {
      // 使用当前编辑器中的脚本内容进行测试
      const result = await usageApi.testScript(
        provider.id,
        appId,
        script.code,
        script.timeout,
        script.accessToken,
        script.userId
      );
      if (result.success && result.data && result.data.length > 0) {
        // 显示所有套餐数据
        const summary = result.data
          .map((plan) => {
            const planInfo = plan.planName ? `[${plan.planName}]` : "";
            return `${planInfo} ${t("usage.remaining")} ${plan.remaining} ${plan.unit}`;
          })
          .join(", ");
        toast.success(`${t("usageScript.testSuccess")}${summary}`, {
          duration: 3000,
        });
      } else {
        toast.error(
          `${t("usageScript.testFailed")}: ${result.error || t("endpointTest.noResult")}`,
          {
            duration: 5000,
          }
        );
      }
    } catch (error: any) {
      toast.error(
        `${t("usageScript.testFailed")}: ${error?.message || t("common.unknown")}`,
        {
          duration: 5000,
        }
      );
    } finally {
      setTesting(false);
    }
  };

  const handleFormat = async () => {
    try {
      const formatted = await prettier.format(script.code, {
        parser: "babel",
        plugins: [parserBabel as any, pluginEstree as any],
        semi: true,
        singleQuote: false,
        tabWidth: 2,
        printWidth: 80,
      });
      setScript({ ...script, code: formatted.trim() });
      toast.success(t("usageScript.formatSuccess"), { duration: 1000 });
    } catch (error: any) {
      toast.error(
        `${t("usageScript.formatFailed")}: ${error?.message || t("jsonEditor.invalidJson")}`,
        {
          duration: 3000,
        }
      );
    }
  };

  const handleUsePreset = (presetName: string) => {
    const preset = PRESET_TEMPLATES[presetName];
    if (preset) {
      // 如果选择的不是 NewAPI 模板，清空高级配置字段
      if (presetName !== TEMPLATE_KEYS.NEW_API) {
        setScript({
          ...script,
          code: preset,
          accessToken: undefined,
          userId: undefined,
        });
      } else {
        setScript({ ...script, code: preset });
      }
      setSelectedTemplate(presetName); // 记录选择的模板
    }
  };

  // 判断是否应该显示高级配置（仅 NewAPI 模板需要）
  const shouldShowAdvancedConfig = selectedTemplate === TEMPLATE_KEYS.NEW_API;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-3xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            {t("usageScript.title")} - {provider.name}
          </DialogTitle>
        </DialogHeader>

        {/* Content - Scrollable */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          {/* 启用开关 */}
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={script.enabled}
              onChange={(e) =>
                setScript({ ...script, enabled: e.target.checked })
              }
              className="w-4 h-4"
            />
            <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
              {t("usageScript.enableUsageQuery")}
            </span>
          </label>

          {script.enabled && (
            <>
              {/* 预设模板选择 */}
              <div>
                <label className="block text-sm font-medium mb-2 text-gray-900 dark:text-gray-100">
                  {t("usageScript.presetTemplate")}
                </label>
                <div className="flex gap-2">
                  {Object.keys(PRESET_TEMPLATES).map((name) => {
                    const isSelected = selectedTemplate === name;
                    return (
                      <button
                        key={name}
                        onClick={() => handleUsePreset(name)}
                        className={`px-3 py-1.5 text-xs rounded transition-colors ${
                          isSelected
                            ? "bg-blue-500 text-white dark:bg-blue-600"
                            : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                        }`}
                      >
                        {t(TEMPLATE_NAME_KEYS[name])}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* 高级配置：Access Token 和 User ID（仅 NewAPI 模板显示） */}
              {shouldShowAdvancedConfig && (
                <div className="space-y-3 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
                  <label className="block">
                    <span className="text-xs text-gray-600 dark:text-gray-400">
                      {t("usageScript.accessToken")}
                    </span>
                    <input
                      type="text"
                      value={script.accessToken || ""}
                      onChange={(e) =>
                        setScript({ ...script, accessToken: e.target.value })
                      }
                      placeholder={t("usageScript.accessTokenPlaceholder")}
                      className="mt-1 w-full px-3 py-2 border border-border-default dark:border-border-default rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 text-sm"
                    />
                  </label>

                  <label className="block">
                    <span className="text-xs text-gray-600 dark:text-gray-400">
                      {t("usageScript.userId")}
                    </span>
                    <input
                      type="text"
                      value={script.userId || ""}
                      onChange={(e) =>
                        setScript({ ...script, userId: e.target.value })
                      }
                      placeholder={t("usageScript.userIdPlaceholder")}
                      className="mt-1 w-full px-3 py-2 border border-border-default dark:border-border-default rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100 text-sm"
                    />
                  </label>
                </div>
              )}

              {/* 脚本编辑器 */}
              <div>
                <label className="block text-sm font-medium mb-2 text-gray-900 dark:text-gray-100">
                  {t("usageScript.queryScript")}
                </label>
                <JsonEditor
                  value={script.code}
                  onChange={(code) => setScript({ ...script, code })}
                  height="300px"
                  language="javascript"
                />
                <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                  {t("usageScript.variablesHint", {
                    apiKey: "{{apiKey}}",
                    baseUrl: "{{baseUrl}}",
                  })}
                </p>
              </div>

              {/* 配置选项 */}
              <div className="grid grid-cols-2 gap-4">
                <label className="block">
                  <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {t("usageScript.timeoutSeconds")}
                  </span>
                  <input
                    type="number"
                    min="2"
                    max="30"
                    value={script.timeout || 10}
                    onChange={(e) =>
                      setScript({
                        ...script,
                        timeout: parseInt(e.target.value),
                      })
                    }
                    className="mt-1 w-full px-3 py-2 border border-border-default dark:border-border-default rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                  />
                </label>

                {/* 🆕 自动查询间隔 */}
                <label className="block">
                  <span className="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {t("usageScript.autoQueryInterval")}
                  </span>
                  <input
                    type="number"
                    min="0"
                    max="1440"
                    step="1"
                    value={script.autoQueryInterval || 0}
                    onChange={(e) =>
                      setScript({
                        ...script,
                        autoQueryInterval: parseInt(e.target.value) || 0,
                      })
                    }
                    className="mt-1 w-full px-3 py-2 border border-border-default dark:border-border-default rounded bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100"
                  />
                  <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                    {t("usageScript.autoQueryIntervalHint")}
                  </p>
                </label>
              </div>

              {/* 脚本说明 */}
              <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-sm text-gray-700 dark:text-gray-300">
                <h4 className="font-medium mb-2">
                  {t("usageScript.scriptHelp")}
                </h4>
                <div className="space-y-3 text-xs">
                  <div>
                    <strong>{t("usageScript.configFormat")}</strong>
                    <pre className="mt-1 p-2 bg-white/50 dark:bg-black/20 rounded text-[10px] overflow-x-auto">
                      {`({
  request: {
    url: "{{baseUrl}}/api/usage",
    method: "POST",
    headers: {
      "Authorization": "Bearer {{apiKey}}",
      "User-Agent": "cc-switch/1.0"
    },
    body: JSON.stringify({ key: "value" })  // ${t("usageScript.commentOptional")}
  },
  extractor: function(response) {
    // ${t("usageScript.commentResponseIsJson")}
    return {
      isValid: !response.error,
      remaining: response.balance,
      unit: "USD"
    };
  }
})`}
                    </pre>
                  </div>

                  <div>
                    <strong>{t("usageScript.extractorFormat")}</strong>
                    <ul className="mt-1 space-y-0.5 ml-2">
                      <li>{t("usageScript.fieldIsValid")}</li>
                      <li>{t("usageScript.fieldInvalidMessage")}</li>
                      <li>{t("usageScript.fieldRemaining")}</li>
                      <li>{t("usageScript.fieldUnit")}</li>
                      <li>{t("usageScript.fieldPlanName")}</li>
                      <li>{t("usageScript.fieldTotal")}</li>
                      <li>{t("usageScript.fieldUsed")}</li>
                      <li>{t("usageScript.fieldExtra")}</li>
                    </ul>
                  </div>

                  <div className="text-gray-600 dark:text-gray-400">
                    <strong>{t("usageScript.tips")}</strong>
                    <ul className="mt-1 space-y-0.5 ml-2">
                      <li>
                        {t("usageScript.tip1", {
                          apiKey: "{{apiKey}}",
                          baseUrl: "{{baseUrl}}",
                        })}
                      </li>
                      <li>{t("usageScript.tip2")}</li>
                      <li>{t("usageScript.tip3")}</li>
                    </ul>
                  </div>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <DialogFooter className="flex-col sm:flex-row sm:justify-between gap-3 pt-4">
          {/* Left side - Test and Format buttons */}
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleTest}
              disabled={!script.enabled || testing}
            >
              <Play size={14} />
              {testing ? t("usageScript.testing") : t("usageScript.testScript")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={handleFormat}
              disabled={!script.enabled}
              title={t("usageScript.format")}
            >
              <Wand2 size={14} />
              {t("usageScript.format")}
            </Button>
          </div>

          {/* Right side - Cancel and Save buttons */}
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={onClose}>
              {t("common.cancel")}
            </Button>
            <Button variant="default" size="sm" onClick={handleSave}>
              {t("usageScript.saveConfig")}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default UsageScriptModal;
