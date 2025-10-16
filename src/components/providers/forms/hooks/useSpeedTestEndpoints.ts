import { useMemo } from "react";
import type { AppType } from "@/lib/api";
import type { ProviderPreset } from "@/config/providerPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";

type PresetEntry = {
  id: string;
  preset: ProviderPreset | CodexProviderPreset;
};

interface UseSpeedTestEndpointsProps {
  appType: AppType;
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  baseUrl: string;
  codexBaseUrl: string;
  initialData?: {
    settingsConfig?: Record<string, unknown>;
  };
}

export interface EndpointCandidate {
  url: string;
}

/**
 * 收集端点测速弹窗的初始端点列表
 *
 * 收集来源：
 * 1. 当前选中的 Base URL
 * 2. 编辑模式下的初始数据 URL
 * 3. 预设中的 endpointCandidates
 */
export function useSpeedTestEndpoints({
  appType,
  selectedPresetId,
  presetEntries,
  baseUrl,
  codexBaseUrl,
  initialData,
}: UseSpeedTestEndpointsProps) {
  const claudeEndpoints = useMemo<EndpointCandidate[]>(() => {
    if (appType !== "claude") return [];

    const map = new Map<string, EndpointCandidate>();
    const add = (url?: string) => {
      if (!url) return;
      const sanitized = url.trim().replace(/\/+$/, "");
      if (!sanitized || map.has(sanitized)) return;
      map.set(sanitized, { url: sanitized });
    };

    // 1. 当前 Base URL
    if (baseUrl) {
      add(baseUrl);
    }

    // 2. 编辑模式：初始数据中的 URL
    if (initialData && typeof initialData.settingsConfig === "object") {
      const envUrl = (initialData.settingsConfig as any)?.env
        ?.ANTHROPIC_BASE_URL;
      if (typeof envUrl === "string") {
        add(envUrl);
      }
    }

    // 3. 预设中的 endpointCandidates
    if (selectedPresetId && selectedPresetId !== "custom") {
      const entry = presetEntries.find((item) => item.id === selectedPresetId);
      if (entry) {
        const preset = entry.preset as ProviderPreset;
        // 添加预设自己的 baseUrl
        const presetEnv = (preset.settingsConfig as any)?.env
          ?.ANTHROPIC_BASE_URL;
        if (typeof presetEnv === "string") {
          add(presetEnv);
        }
        // 添加预设的候选端点
        if (Array.isArray((preset as any).endpointCandidates)) {
          for (const u of (preset as any).endpointCandidates as string[]) {
            add(u);
          }
        }
      }
    }

    return Array.from(map.values());
  }, [appType, baseUrl, initialData, selectedPresetId, presetEntries]);

  const codexEndpoints = useMemo<EndpointCandidate[]>(() => {
    if (appType !== "codex") return [];

    const map = new Map<string, EndpointCandidate>();
    const add = (url?: string) => {
      if (!url) return;
      const sanitized = url.trim().replace(/\/+$/, "");
      if (!sanitized || map.has(sanitized)) return;
      map.set(sanitized, { url: sanitized });
    };

    // 1. 当前 Codex Base URL
    if (codexBaseUrl) {
      add(codexBaseUrl);
    }

    // 2. 编辑模式：初始数据中的 URL
    const initialCodexConfig =
      initialData && typeof initialData.settingsConfig?.config === "string"
        ? (initialData.settingsConfig as any).config
        : "";
    // 从 TOML 中提取 base_url
    const match = /base_url\s*=\s*["']([^"']+)["']/i.exec(initialCodexConfig);
    if (match?.[1]) {
      add(match[1]);
    }

    // 3. 预设中的 endpointCandidates
    if (selectedPresetId && selectedPresetId !== "custom") {
      const entry = presetEntries.find((item) => item.id === selectedPresetId);
      if (entry) {
        const preset = entry.preset as CodexProviderPreset;
        // 添加预设自己的 baseUrl
        const presetConfig = preset.config || "";
        const presetMatch = /base_url\s*=\s*["']([^"']+)["']/i.exec(
          presetConfig
        );
        if (presetMatch?.[1]) {
          add(presetMatch[1]);
        }
        // 添加预设的候选端点
        if (Array.isArray((preset as any).endpointCandidates)) {
          for (const u of (preset as any).endpointCandidates as string[]) {
            add(u);
          }
        }
      }
    }

    return Array.from(map.values());
  }, [appType, codexBaseUrl, initialData, selectedPresetId, presetEntries]);

  return appType === "codex" ? codexEndpoints : claudeEndpoints;
}
