import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { Provider, CustomEndpoint } from "@/types";
import type { AppType } from "@/lib/api";
import {
  ProviderForm,
  type ProviderFormValues,
} from "@/components/providers/forms/ProviderForm";
import { providerPresets } from "@/config/providerPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";

interface AddProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appType: AppType;
  onSubmit: (provider: Omit<Provider, "id">) => Promise<void> | void;
}

export function AddProviderDialog({
  open,
  onOpenChange,
  appType,
  onSubmit,
}: AddProviderDialogProps) {
  const { t } = useTranslation();

  const handleSubmit = useCallback(
    async (values: ProviderFormValues) => {
      const parsedConfig = JSON.parse(values.settingsConfig) as Record<
        string,
        unknown
      >;

      // 构造基础提交数据
      const providerData: Omit<Provider, "id"> = {
        name: values.name.trim(),
        websiteUrl: values.websiteUrl?.trim() || undefined,
        settingsConfig: parsedConfig,
        ...(values.presetCategory ? { category: values.presetCategory } : {}),
      };

      // 收集端点候选（仅新增供应商时）
      // 1. 从预设配置中获取 endpointCandidates
      // 2. 从当前配置中提取 baseUrl (ANTHROPIC_BASE_URL 或 Codex base_url)
      const urlSet = new Set<string>();

      const addUrl = (rawUrl?: string) => {
        const url = (rawUrl || "").trim().replace(/\/+$/, "");
        if (url && url.startsWith("http")) {
          urlSet.add(url);
        }
      };

      // 如果选择了预设，获取预设中的 endpointCandidates
      if (values.presetId) {
        if (appType === "claude") {
          const presets = providerPresets;
          const presetIndex = parseInt(values.presetId.replace("claude-", ""));
          if (
            !isNaN(presetIndex) &&
            presetIndex >= 0 &&
            presetIndex < presets.length
          ) {
            const preset = presets[presetIndex];
            if (preset?.endpointCandidates) {
              preset.endpointCandidates.forEach(addUrl);
            }
          }
        } else if (appType === "codex") {
          const presets = codexProviderPresets;
          const presetIndex = parseInt(values.presetId.replace("codex-", ""));
          if (
            !isNaN(presetIndex) &&
            presetIndex >= 0 &&
            presetIndex < presets.length
          ) {
            const preset = presets[presetIndex];
            if ((preset as any).endpointCandidates) {
              (preset as any).endpointCandidates.forEach(addUrl);
            }
          }
        }
      }

      // 从当前配置中提取 baseUrl
      if (appType === "claude") {
        const env = parsedConfig.env as Record<string, any> | undefined;
        if (env?.ANTHROPIC_BASE_URL) {
          addUrl(env.ANTHROPIC_BASE_URL);
        }
      } else if (appType === "codex") {
        // Codex 的 baseUrl 在 config.toml 字符串中
        const config = parsedConfig.config as string | undefined;
        if (config) {
          const baseUrlMatch = config.match(/base_url\s*=\s*["']([^"']+)["']/);
          if (baseUrlMatch?.[1]) {
            addUrl(baseUrlMatch[1]);
          }
        }
      }

      // 如果收集到了端点，添加到 meta.custom_endpoints
      const urls = Array.from(urlSet);
      if (urls.length > 0) {
        const now = Date.now();
        const customEndpoints: Record<string, CustomEndpoint> = {};
        urls.forEach((url) => {
          customEndpoints[url] = {
            url,
            addedAt: now,
            lastUsed: undefined,
          };
        });

        providerData.meta = {
          custom_endpoints: customEndpoints,
        };
      }

      await onSubmit(providerData);
      onOpenChange(false);
    },
    [appType, onSubmit, onOpenChange],
  );

  const submitLabel =
    appType === "claude"
      ? t("provider.addClaudeProvider", { defaultValue: "添加 Claude 供应商" })
      : t("provider.addCodexProvider", { defaultValue: "添加 Codex 供应商" });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{submitLabel}</DialogTitle>
          <DialogDescription>
            {t("provider.addDescription", {
              defaultValue: "填写信息后即可在列表中快速切换供应商。",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 py-4">
          <ProviderForm
            appType={appType}
            submitLabel={t("common.add", { defaultValue: "添加" })}
            onSubmit={handleSubmit}
            onCancel={() => onOpenChange(false)}
            showButtons={false}
          />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button type="submit" form="provider-form">
            <Plus className="h-4 w-4" />
            {t("common.add", { defaultValue: "添加" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
