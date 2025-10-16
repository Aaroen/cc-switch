import { useMemo } from "react";
import type { AppType } from "@/lib/api";
import type { ProviderCategory } from "@/types";
import type { ProviderPreset } from "@/config/providerPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";

type PresetEntry = {
  id: string;
  preset: ProviderPreset | CodexProviderPreset;
};

interface UseApiKeyLinkProps {
  appType: AppType;
  category?: ProviderCategory;
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  formWebsiteUrl: string;
}

/**
 * 管理 API Key 获取链接的显示和 URL
 */
export function useApiKeyLink({
  appType,
  category,
  selectedPresetId,
  presetEntries,
  formWebsiteUrl,
}: UseApiKeyLinkProps) {
  // 判断是否显示 API Key 获取链接
  const shouldShowApiKeyLink = useMemo(() => {
    return (
      category !== "official" &&
      (category === "cn_official" ||
        category === "aggregator" ||
        category === "third_party")
    );
  }, [category]);

  // 获取当前供应商的网址（用于 API Key 链接）
  const getWebsiteUrl = useMemo(() => {
    if (selectedPresetId && selectedPresetId !== "custom") {
      const entry = presetEntries.find((item) => item.id === selectedPresetId);
      if (entry) {
        const preset = entry.preset;
        // 第三方供应商优先使用 apiKeyUrl
        return preset.category === "third_party"
          ? preset.apiKeyUrl || preset.websiteUrl || ""
          : preset.websiteUrl || "";
      }
    }
    return formWebsiteUrl || "";
  }, [selectedPresetId, presetEntries, formWebsiteUrl]);

  return {
    shouldShowApiKeyLink: appType === "claude"
      ? shouldShowApiKeyLink
      : appType === "codex"
        ? shouldShowApiKeyLink
        : false,
    websiteUrl: getWebsiteUrl,
  };
}
