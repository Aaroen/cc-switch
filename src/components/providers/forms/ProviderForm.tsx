import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { useTheme } from "@/components/theme-provider";
import JsonEditor from "@/components/JsonEditor";
import { providerSchema, type ProviderFormData } from "@/lib/schemas/provider";
import type { AppType } from "@/lib/api";
import type { ProviderCategory } from "@/types";
import { providerPresets, type ProviderPreset } from "@/config/providerPresets";
import {
  codexProviderPresets,
  type CodexProviderPreset,
} from "@/config/codexProviderPresets";
import { applyTemplateValues } from "@/utils/providerConfigUtils";
import ApiKeyInput from "@/components/ProviderForm/ApiKeyInput";
import EndpointSpeedTest from "@/components/ProviderForm/EndpointSpeedTest";
import { Zap } from "lucide-react";
import { useProviderCategory, useApiKeyState, useBaseUrlState, useModelState } from "./hooks";

const CLAUDE_DEFAULT_CONFIG = JSON.stringify({ env: {}, config: {} }, null, 2);
const CODEX_DEFAULT_CONFIG = JSON.stringify({ auth: {}, config: "" }, null, 2);

type PresetEntry = {
  id: string;
  preset: ProviderPreset | CodexProviderPreset;
};

interface ProviderFormProps {
  appType: AppType;
  submitLabel: string;
  onSubmit: (values: ProviderFormValues) => void;
  onCancel: () => void;
  initialData?: {
    name?: string;
    websiteUrl?: string;
    settingsConfig?: Record<string, unknown>;
  };
}

export function ProviderForm({
  appType,
  submitLabel,
  onSubmit,
  onCancel,
  initialData,
}: ProviderFormProps) {
  const { t } = useTranslation();
  const { theme } = useTheme();
  const isEditMode = Boolean(initialData);

  const [selectedPresetId, setSelectedPresetId] = useState<string | null>(
    initialData ? null : "custom",
  );
  const [activePreset, setActivePreset] = useState<{
    id: string;
    category?: ProviderCategory;
  } | null>(null);
  const [isEndpointModalOpen, setIsEndpointModalOpen] = useState(false);

  // 使用 category hook
  const { category } = useProviderCategory({
    appType,
    selectedPresetId,
    isEditMode,
  });

  useEffect(() => {
    setSelectedPresetId(initialData ? null : "custom");
    setActivePreset(null);
  }, [appType, initialData]);

  const defaultValues: ProviderFormData = useMemo(
    () => ({
      name: initialData?.name ?? "",
      websiteUrl: initialData?.websiteUrl ?? "",
      settingsConfig: initialData?.settingsConfig
        ? JSON.stringify(initialData.settingsConfig, null, 2)
        : appType === "codex"
          ? CODEX_DEFAULT_CONFIG
          : CLAUDE_DEFAULT_CONFIG,
    }),
    [initialData, appType],
  );

  const form = useForm<ProviderFormData>({
    resolver: zodResolver(providerSchema),
    defaultValues,
    mode: "onSubmit",
  });

  // 使用 API Key hook
  const { apiKey, handleApiKeyChange, showApiKey: shouldShowApiKey } = useApiKeyState({
    initialConfig: form.watch("settingsConfig"),
    onConfigChange: (config) => form.setValue("settingsConfig", config),
    selectedPresetId,
  });

  // 使用 Base URL hook
  const {
    baseUrl,
    // codexBaseUrl,  // TODO: 等 Codex 支持时使用
    handleClaudeBaseUrlChange,
    // handleCodexBaseUrlChange, // TODO: 等 Codex 支持时使用
  } = useBaseUrlState({
    appType,
    category,
    settingsConfig: form.watch("settingsConfig"),
    codexConfig: "", // TODO: 从 settingsConfig 中提取 codex config
    onSettingsConfigChange: (config) => form.setValue("settingsConfig", config),
    onCodexConfigChange: () => {
      // TODO: 更新 codex config
    },
  });

  // 使用 Model hook
  const { claudeModel, claudeSmallFastModel, handleModelChange } = useModelState({
    settingsConfig: form.watch("settingsConfig"),
    onConfigChange: (config) => form.setValue("settingsConfig", config),
  });

  useEffect(() => {
    form.reset(defaultValues);
  }, [defaultValues, form]);

  const isDarkMode = useMemo(() => {
    if (theme === "dark") return true;
    if (theme === "light") return false;
    return typeof window !== "undefined"
      ? window.document.documentElement.classList.contains("dark")
      : false;
  }, [theme]);

  const handleSubmit = (values: ProviderFormData) => {
    const payload: ProviderFormValues = {
      ...values,
      name: values.name.trim(),
      websiteUrl: values.websiteUrl?.trim() ?? "",
      settingsConfig: values.settingsConfig.trim(),
    };

    if (activePreset) {
      payload.presetId = activePreset.id;
      if (activePreset.category) {
        payload.presetCategory = activePreset.category;
      }
    }

    onSubmit(payload);
  };

  const presetCategoryLabels: Record<string, string> = useMemo(
    () => ({
      official: t("providerPreset.categoryOfficial", {
        defaultValue: "官方推荐",
      }),
      cn_official: t("providerPreset.categoryCnOfficial", {
        defaultValue: "国内官方",
      }),
      aggregator: t("providerPreset.categoryAggregator", {
        defaultValue: "聚合服务",
      }),
      third_party: t("providerPreset.categoryThirdParty", {
        defaultValue: "第三方",
      }),
    }),
    [t],
  );

  const presetEntries = useMemo(() => {
    if (appType === "codex") {
      return codexProviderPresets.map<PresetEntry>((preset, index) => ({
        id: `codex-${index}`,
        preset,
      }));
    }
    return providerPresets.map<PresetEntry>((preset, index) => ({
      id: `claude-${index}`,
      preset,
    }));
  }, [appType]);

  const groupedPresets = useMemo(() => {
    return presetEntries.reduce<Record<string, PresetEntry[]>>((acc, entry) => {
      const category = entry.preset.category ?? "others";
      if (!acc[category]) {
        acc[category] = [];
      }
      acc[category].push(entry);
      return acc;
    }, {});
  }, [presetEntries]);

  const categoryKeys = useMemo(() => {
    return Object.keys(groupedPresets).filter(
      (key) => key !== "custom" && groupedPresets[key]?.length,
    );
  }, [groupedPresets]);

  // 判断是否显示端点测速（仅第三方和自定义类别）
  const shouldShowSpeedTest =
    category === "third_party" || category === "custom";

  const handlePresetChange = (value: string) => {
    setSelectedPresetId(value);
    if (value === "custom") {
      setActivePreset(null);
      form.reset(defaultValues);
      return;
    }

    const entry = presetEntries.find((item) => item.id === value);
    if (!entry) {
      return;
    }

    setActivePreset({
      id: value,
      category: entry.preset.category,
    });

    if (appType === "codex") {
      const preset = entry.preset as CodexProviderPreset;
      const config = {
        auth: preset.auth ?? {},
        config: preset.config ?? "",
      };

      form.reset({
        name: preset.name,
        websiteUrl: preset.websiteUrl ?? "",
        settingsConfig: JSON.stringify(config, null, 2),
      });
      return;
    }

    const preset = entry.preset as ProviderPreset;
    const config = applyTemplateValues(
      preset.settingsConfig,
      preset.templateValues,
    );

    form.reset({
      name: preset.name,
      websiteUrl: preset.websiteUrl ?? "",
      settingsConfig: JSON.stringify(config, null, 2),
    });
  };

  return (
    <Form {...form}>
      <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-6">
        {/* 预设供应商选择（仅新增模式显示） */}
        {!initialData && (
          <div className="space-y-3">
            <FormLabel>
              {t("providerPreset.label", { defaultValue: "预设供应商" })}
            </FormLabel>
            <div className="flex flex-wrap gap-2">
              {/* 自定义按钮 */}
              <button
                type="button"
                onClick={() => handlePresetChange("custom")}
                className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  selectedPresetId === "custom"
                    ? "bg-emerald-500 text-white dark:bg-emerald-600"
                    : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                }`}
              >
                {t("providerPreset.custom", { defaultValue: "自定义配置" })}
              </button>

              {/* 预设按钮 */}
              {categoryKeys.map((category) => {
                const entries = groupedPresets[category];
                if (!entries || entries.length === 0) return null;
                return entries.map((entry) => (
                  <button
                    key={entry.id}
                    type="button"
                    onClick={() => handlePresetChange(entry.id)}
                    className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                      selectedPresetId === entry.id
                        ? "bg-emerald-500 text-white dark:bg-emerald-600"
                        : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                    }`}
                    title={
                      presetCategoryLabels[category] ??
                      t("providerPreset.categoryOther", {
                        defaultValue: "其他",
                      })
                    }
                  >
                    {entry.preset.name}
                  </button>
                ));
              })}
            </div>
            <p className="text-xs text-muted-foreground">
              {t("providerPreset.helper", {
                defaultValue: "选择预设后可继续调整下方字段。",
              })}
            </p>
          </div>
        )}

        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.name", { defaultValue: "供应商名称" })}
              </FormLabel>
              <FormControl>
                <Input
                  {...field}
                  placeholder={t("provider.namePlaceholder", {
                    defaultValue: "例如：Claude 官方",
                  })}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="websiteUrl"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.websiteUrl", { defaultValue: "官网链接" })}
              </FormLabel>
              <FormControl>
                <Input {...field} placeholder="https://" />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        {/* API Key 输入框（仅 Claude 且非编辑模式显示） */}
        {appType === "claude" && shouldShowApiKey(form.watch("settingsConfig"), isEditMode) && (
          <div>
            <ApiKeyInput
              value={apiKey}
              onChange={handleApiKeyChange}
              required={category !== "official"}
              placeholder={
                category === "official"
                  ? t("providerForm.officialNoApiKey", { defaultValue: "官方供应商无需 API Key" })
                  : t("providerForm.apiKeyAutoFill", { defaultValue: "输入 API Key，将自动填充到配置" })
              }
              disabled={category === "official"}
            />
          </div>
        )}

        {/* Base URL 输入框（仅 Claude 第三方/自定义显示） */}
        {appType === "claude" && shouldShowSpeedTest && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <FormLabel htmlFor="baseUrl">
                {t("providerForm.apiEndpoint", { defaultValue: "API 端点" })}
              </FormLabel>
              <button
                type="button"
                onClick={() => setIsEndpointModalOpen(true)}
                className="flex items-center gap-1 text-xs text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
              >
                <Zap className="h-3.5 w-3.5" />
                {t("providerForm.manageAndTest", { defaultValue: "管理和测速" })}
              </button>
            </div>
            <Input
              id="baseUrl"
              type="url"
              value={baseUrl}
              onChange={(e) => handleClaudeBaseUrlChange(e.target.value)}
              placeholder={t("providerForm.apiEndpointPlaceholder", { defaultValue: "https://api.example.com" })}
              autoComplete="off"
            />
            <div className="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg">
              <p className="text-xs text-amber-600 dark:text-amber-400">
                {t("providerForm.apiHint", { defaultValue: "API 端点地址用于连接服务器" })}
              </p>
            </div>
          </div>
        )}

        {/* 端点测速弹窗 - Claude */}
        {appType === "claude" && shouldShowSpeedTest && isEndpointModalOpen && (
          <EndpointSpeedTest
            appType={appType}
            value={baseUrl}
            onChange={handleClaudeBaseUrlChange}
            initialEndpoints={[{ url: baseUrl }]}
            visible={isEndpointModalOpen}
            onClose={() => setIsEndpointModalOpen(false)}
          />
        )}

        {/* 模型选择器（仅 Claude 非官方供应商显示） */}
        {appType === "claude" && category !== "official" && (
          <div className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* ANTHROPIC_MODEL */}
              <div className="space-y-2">
                <FormLabel htmlFor="claudeModel">
                  {t("providerForm.anthropicModel", { defaultValue: "主模型" })}
                </FormLabel>
                <Input
                  id="claudeModel"
                  type="text"
                  value={claudeModel}
                  onChange={(e) => handleModelChange("ANTHROPIC_MODEL", e.target.value)}
                  placeholder={t("providerForm.modelPlaceholder", {
                    defaultValue: "claude-3-7-sonnet-20250219"
                  })}
                  autoComplete="off"
                />
              </div>

              {/* ANTHROPIC_SMALL_FAST_MODEL */}
              <div className="space-y-2">
                <FormLabel htmlFor="claudeSmallFastModel">
                  {t("providerForm.anthropicSmallFastModel", {
                    defaultValue: "快速模型"
                  })}
                </FormLabel>
                <Input
                  id="claudeSmallFastModel"
                  type="text"
                  value={claudeSmallFastModel}
                  onChange={(e) => handleModelChange("ANTHROPIC_SMALL_FAST_MODEL", e.target.value)}
                  placeholder={t("providerForm.smallModelPlaceholder", {
                    defaultValue: "claude-3-5-haiku-20241022"
                  })}
                  autoComplete="off"
                />
              </div>
            </div>
            <p className="text-xs text-muted-foreground">
              {t("providerForm.modelHelper", {
                defaultValue: "可选：指定默认使用的 Claude 模型，留空则使用系统默认。",
              })}
            </p>
          </div>
        )}

        <FormField
          control={form.control}
          name="settingsConfig"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.configJson", { defaultValue: "配置 JSON" })}
              </FormLabel>
              <FormControl>
                <div className="rounded-md border">
                  <JsonEditor
                    value={field.value}
                    onChange={field.onChange}
                    placeholder={
                      appType === "codex"
                        ? CODEX_DEFAULT_CONFIG
                        : CLAUDE_DEFAULT_CONFIG
                    }
                    darkMode={isDarkMode}
                    rows={14}
                    showValidation
                  />
                </div>
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <div className="flex justify-end gap-2">
          <Button variant="outline" type="button" onClick={onCancel}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button type="submit">{submitLabel}</Button>
        </div>
      </form>
    </Form>
  );
}

export type ProviderFormValues = ProviderFormData & {
  presetId?: string;
  presetCategory?: ProviderCategory;
};
