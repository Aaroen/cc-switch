import React, { useState } from "react";
import { Play, Wand2, Eye, EyeOff } from "lucide-react";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

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
const generatePresetTemplates = (
  t: (key: string) => string,
): Record<string, string> => ({
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
    },
  );

  // 控制 API Key 的显示/隐藏
  const [showApiKey, setShowApiKey] = useState(false);
  const [showAccessToken, setShowAccessToken] = useState(false);

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
        script.apiKey,
        script.baseUrl,
        script.accessToken,
        script.userId,
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
          },
        );
      }
    } catch (error: any) {
      toast.error(
        `${t("usageScript.testFailed")}: ${error?.message || t("common.unknown")}`,
        {
          duration: 5000,
        },
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
        },
      );
    }
  };

  const handleUsePreset = (presetName: string) => {
    const preset = PRESET_TEMPLATES[presetName];
    if (preset) {
      // 根据模板类型清空不同的字段
      if (presetName === TEMPLATE_KEYS.CUSTOM) {
        // 自定义：清空所有凭证字段
        setScript({
          ...script,
          code: preset,
          apiKey: undefined,
          baseUrl: undefined,
          accessToken: undefined,
          userId: undefined,
        });
      } else if (presetName === TEMPLATE_KEYS.GENERAL) {
        // 通用：保留 apiKey 和 baseUrl，清空 NewAPI 字段
        setScript({
          ...script,
          code: preset,
          accessToken: undefined,
          userId: undefined,
        });
      } else if (presetName === TEMPLATE_KEYS.NEW_API) {
        // NewAPI：清空 apiKey（NewAPI 不使用通用的 apiKey）
        setScript({
          ...script,
          code: preset,
          apiKey: undefined,
        });
      }
      setSelectedTemplate(presetName); // 记录选择的模板
    }
  };

  // 判断是否应该显示凭证配置区域
  const shouldShowCredentialsConfig =
    selectedTemplate === TEMPLATE_KEYS.GENERAL || selectedTemplate === TEMPLATE_KEYS.NEW_API;

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
                <Label className="mb-2">
                  {t("usageScript.presetTemplate")}
                </Label>
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

              {/* 凭证配置区域：通用和 NewAPI 模板显示 */}
              {shouldShowCredentialsConfig && (
                <div className="space-y-4 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
                  <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                    {t("usageScript.credentialsConfig")}
                  </h4>

                  {/* 通用模板：显示 apiKey + baseUrl */}
                  {selectedTemplate === TEMPLATE_KEYS.GENERAL && (
                    <>
                      <div className="space-y-2">
                        <Label htmlFor="usage-api-key">
                          API Key
                        </Label>
                        <div className="relative">
                          <Input
                            id="usage-api-key"
                            type={showApiKey ? "text" : "password"}
                            value={script.apiKey || ""}
                            onChange={(e) =>
                              setScript({ ...script, apiKey: e.target.value })
                            }
                            placeholder="sk-xxxxx"
                            autoComplete="off"
                          />
                          {script.apiKey && (
                            <button
                              type="button"
                              onClick={() => setShowApiKey(!showApiKey)}
                              className="absolute inset-y-0 right-0 flex items-center pr-3 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
                              aria-label={showApiKey ? t("apiKeyInput.hide") : t("apiKeyInput.show")}
                            >
                              {showApiKey ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                          )}
                        </div>
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="usage-base-url">
                          Base URL
                        </Label>
                        <Input
                          id="usage-base-url"
                          type="text"
                          value={script.baseUrl || ""}
                          onChange={(e) =>
                            setScript({ ...script, baseUrl: e.target.value })
                          }
                          placeholder="https://api.example.com"
                          autoComplete="off"
                        />
                      </div>
                    </>
                  )}

                  {/* NewAPI 模板：显示 baseUrl + accessToken + userId */}
                  {selectedTemplate === TEMPLATE_KEYS.NEW_API && (
                    <>
                      <div className="space-y-2">
                        <Label htmlFor="usage-newapi-base-url">
                          Base URL
                        </Label>
                        <Input
                          id="usage-newapi-base-url"
                          type="text"
                          value={script.baseUrl || ""}
                          onChange={(e) =>
                            setScript({ ...script, baseUrl: e.target.value })
                          }
                          placeholder="https://api.newapi.com"
                          autoComplete="off"
                        />
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="usage-access-token">
                          {t("usageScript.accessToken")}
                        </Label>
                        <div className="relative">
                          <Input
                            id="usage-access-token"
                            type={showAccessToken ? "text" : "password"}
                            value={script.accessToken || ""}
                            onChange={(e) =>
                              setScript({ ...script, accessToken: e.target.value })
                            }
                            placeholder={t("usageScript.accessTokenPlaceholder")}
                            autoComplete="off"
                          />
                          {script.accessToken && (
                            <button
                              type="button"
                              onClick={() => setShowAccessToken(!showAccessToken)}
                              className="absolute inset-y-0 right-0 flex items-center pr-3 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
                              aria-label={showAccessToken ? t("apiKeyInput.hide") : t("apiKeyInput.show")}
                            >
                              {showAccessToken ? <EyeOff size={16} /> : <Eye size={16} />}
                            </button>
                          )}
                        </div>
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="usage-user-id">
                          {t("usageScript.userId")}
                        </Label>
                        <Input
                          id="usage-user-id"
                          type="text"
                          value={script.userId || ""}
                          onChange={(e) =>
                            setScript({ ...script, userId: e.target.value })
                          }
                          placeholder={t("usageScript.userIdPlaceholder")}
                          autoComplete="off"
                        />
                      </div>
                    </>
                  )}
                </div>
              )}

              {/* 脚本编辑器 */}
              <div>
                <Label className="mb-2">
                  {t("usageScript.queryScript")}
                </Label>
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
                <div className="space-y-2">
                  <Label htmlFor="usage-timeout">
                    {t("usageScript.timeoutSeconds")}
                  </Label>
                  <Input
                    id="usage-timeout"
                    type="number"
                    min={2}
                    max={30}
                    value={script.timeout || 10}
                    onChange={(e) =>
                      setScript({
                        ...script,
                        timeout: parseInt(e.target.value),
                      })
                    }
                  />
                </div>

                {/* 🆕 自动查询间隔 */}
                <div className="space-y-2">
                  <Label htmlFor="usage-auto-interval">
                    {t("usageScript.autoQueryInterval")}
                  </Label>
                  <Input
                    id="usage-auto-interval"
                    type="number"
                    min={0}
                    max={1440}
                    step={1}
                    value={script.autoQueryInterval || 0}
                    onChange={(e) =>
                      setScript({
                        ...script,
                        autoQueryInterval: parseInt(e.target.value) || 0,
                      })
                    }
                  />
                  <p className="text-xs text-muted-foreground">
                    {t("usageScript.autoQueryIntervalHint")}
                  </p>
                </div>
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
