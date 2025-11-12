import { useMemo } from "react";
import type { AppId } from "@/lib/api";
import type { ProviderPreset } from "@/config/claudeProviderPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { ProviderMeta, EndpointCandidate } from "@/types";

type PresetEntry = {
  id: string;
  preset: ProviderPreset | CodexProviderPreset;
};

interface UseSpeedTestEndpointsProps {
  appId: AppId;
  selectedPresetId: string | null;
  presetEntries: PresetEntry[];
  baseUrl: string;
  codexBaseUrl: string;
  initialData?: {
    settingsConfig?: Record<string, unknown>;
    meta?: ProviderMeta;
  };
}

/**
 * 收集端点测速弹窗的初始端点列表
 *
 * 收集来源：
 * 1. 编辑模式：从 meta.custom_endpoints 读取已保存的端点（优先）
 * 2. 当前选中的 Base URL
 * 3. 编辑模式下的初始数据 URL
 * 4. 预设中的 endpointCandidates
 */
export function useSpeedTestEndpoints({
  appId,
  selectedPresetId,
  presetEntries,
  baseUrl,
  codexBaseUrl,
  initialData,
}: UseSpeedTestEndpointsProps) {
  const claudeEndpoints = useMemo<EndpointCandidate[]>(() => {
    // Reuse this branch for Claude and Gemini (non-Codex)
    if (appId !== "claude" && appId !== "gemini") return [];

    const map = new Map<string, EndpointCandidate>();
    // 所有端点都标记为 isCustom: true，给用户完全的管理自由
    const add = (url?: string) => {
      if (!url) return;
      const sanitized = url.trim().replace(/\/+$/, "");
      if (!sanitized || map.has(sanitized)) return;
      map.set(sanitized, { url: sanitized, isCustom: true });
    };

    // 1. 编辑模式：从 meta.custom_endpoints 读取已保存的端点（优先）
    if (initialData?.meta?.custom_endpoints) {
      const customEndpoints = initialData.meta.custom_endpoints;
      for (const url of Object.keys(customEndpoints)) {
        add(url);
      }
    }

    // 2. 当前 Base URL
    if (baseUrl) {
      add(baseUrl);
    }

    // 3. 编辑模式：初始数据中的 URL
    if (initialData && typeof initialData.settingsConfig === "object") {
      const configEnv = initialData.settingsConfig as {
        env?: { ANTHROPIC_BASE_URL?: string; GOOGLE_GEMINI_BASE_URL?: string };
      };
      const envUrls = [
        configEnv.env?.ANTHROPIC_BASE_URL,
        configEnv.env?.GOOGLE_GEMINI_BASE_URL,
      ];
      envUrls.forEach((u) => {
        if (typeof u === "string") add(u);
      });
    }

    // 4. 预设中的 endpointCandidates（也允许用户删除）
    if (selectedPresetId && selectedPresetId !== "custom") {
      const entry = presetEntries.find((item) => item.id === selectedPresetId);
      if (entry) {
        const preset = entry.preset as ProviderPreset & {
          settingsConfig?: { env?: { GOOGLE_GEMINI_BASE_URL?: string } };
          endpointCandidates?: string[];
        };
        // 添加预设自己的 baseUrl（兼容 Claude/Gemini）
        const presetEnv = preset.settingsConfig as {
          env?: {
            ANTHROPIC_BASE_URL?: string;
            GOOGLE_GEMINI_BASE_URL?: string;
          };
        };
        const presetUrls = [
          presetEnv?.env?.ANTHROPIC_BASE_URL,
          presetEnv?.env?.GOOGLE_GEMINI_BASE_URL,
        ];
        presetUrls.forEach((u) => add(u));
        // 添加预设的候选端点
        if (preset.endpointCandidates) {
          preset.endpointCandidates.forEach((url) => add(url));
        }
      }
    }

    return Array.from(map.values());
  }, [appId, baseUrl, initialData, selectedPresetId, presetEntries]);

  const codexEndpoints = useMemo<EndpointCandidate[]>(() => {
    if (appId !== "codex") return [];

    const map = new Map<string, EndpointCandidate>();
    // 所有端点都标记为 isCustom: true，给用户完全的管理自由
    const add = (url?: string) => {
      if (!url) return;
      const sanitized = url.trim().replace(/\/+$/, "");
      if (!sanitized || map.has(sanitized)) return;
      map.set(sanitized, { url: sanitized, isCustom: true });
    };

    // 1. 编辑模式：从 meta.custom_endpoints 读取已保存的端点（优先）
    if (initialData?.meta?.custom_endpoints) {
      const customEndpoints = initialData.meta.custom_endpoints;
      for (const url of Object.keys(customEndpoints)) {
        add(url);
      }
    }

    // 2. 当前 Codex Base URL
    if (codexBaseUrl) {
      add(codexBaseUrl);
    }

    // 3. 编辑模式：初始数据中的 URL
    const initialCodexConfig = initialData?.settingsConfig as
      | {
          config?: string;
        }
      | undefined;
    const configStr = initialCodexConfig?.config ?? "";
    // 从 TOML 中提取 base_url
    const match = /base_url\s*=\s*["']([^"']+)["']/i.exec(configStr);
    if (match?.[1]) {
      add(match[1]);
    }

    // 4. 预设中的 endpointCandidates（也允许用户删除）
    if (selectedPresetId && selectedPresetId !== "custom") {
      const entry = presetEntries.find((item) => item.id === selectedPresetId);
      if (entry) {
        const preset = entry.preset as CodexProviderPreset;
        // 添加预设自己的 baseUrl
        const presetConfig = preset.config || "";
        const presetMatch = /base_url\s*=\s*["']([^"']+)["']/i.exec(
          presetConfig,
        );
        if (presetMatch?.[1]) {
          add(presetMatch[1]);
        }
        // 添加预设的候选端点
        if (preset.endpointCandidates) {
          preset.endpointCandidates.forEach((url) => add(url));
        }
      }
    }

    return Array.from(map.values());
  }, [appId, codexBaseUrl, initialData, selectedPresetId, presetEntries]);

  return appId === "codex" ? codexEndpoints : claudeEndpoints;
}
