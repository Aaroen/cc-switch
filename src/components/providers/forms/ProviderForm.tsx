import { useEffect, useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
import {
  providerPresets,
  type ProviderPreset,
} from "@/config/providerPresets";
import {
  codexProviderPresets,
  type CodexProviderPreset,
} from "@/config/codexProviderPresets";
import { applyTemplateValues } from "@/utils/providerConfigUtils";

const CLAUDE_DEFAULT_CONFIG = JSON.stringify({ env: {}, config: {} }, null, 2);
const CODEX_DEFAULT_CONFIG = JSON.stringify(
  { auth: {}, config: "" },
  null,
  2,
);

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
  const [selectedPresetId, setSelectedPresetId] = useState<string>("custom");
  const [activePreset, setActivePreset] = useState<{
    id: string;
    category?: ProviderCategory;
  } | null>(null);

  useEffect(() => {
    setSelectedPresetId("custom");
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

  const handlePresetChange = (value: string) => {
    setSelectedPresetId(value);
    if (value === "custom") {
      setActivePreset(null);
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
        <div className="space-y-2">
          <FormLabel>
            {t("providerPreset.label", { defaultValue: "预设供应商" })}
          </FormLabel>
          <Select value={selectedPresetId} onValueChange={handlePresetChange}>
            <SelectTrigger>
              <SelectValue
                placeholder={t("providerPreset.placeholder", {
                  defaultValue: "选择一个预设",
                })}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="custom">
                {t("providerPreset.custom", { defaultValue: "自定义配置" })}
              </SelectItem>

              {categoryKeys.map((category) => {
                const entries = groupedPresets[category];
                if (!entries || entries.length === 0) return null;
                return (
                  <SelectGroup key={category}>
                    <SelectLabel>
                      {presetCategoryLabels[category] ??
                        t("providerPreset.categoryOther", {
                          defaultValue: "其他",
                        })}
                    </SelectLabel>
                    {entries.map((entry) => (
                      <SelectItem key={entry.id} value={entry.id}>
                        {entry.preset.name}
                      </SelectItem>
                    ))}
                  </SelectGroup>
                );
              })}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {t("providerPreset.helper", {
              defaultValue: "选择预设后可继续调整下方字段。",
            })}
          </p>
        </div>

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
