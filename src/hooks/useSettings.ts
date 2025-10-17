import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { settingsApi, type AppType } from "@/lib/api";
import { useSettingsQuery, useSaveSettingsMutation } from "@/lib/query";
import type { Settings } from "@/types";
import {
  useSettingsForm,
  type SettingsFormState,
} from "./useSettingsForm";
import {
  useDirectorySettings,
  type ResolvedDirectories,
} from "./useDirectorySettings";
import { useSettingsMetadata } from "./useSettingsMetadata";

type Language = "zh" | "en";

interface SaveResult {
  requiresRestart: boolean;
}

export interface UseSettingsResult {
  settings: SettingsFormState | null;
  isLoading: boolean;
  isSaving: boolean;
  isPortable: boolean;
  appConfigDir?: string;
  resolvedDirs: ResolvedDirectories;
  requiresRestart: boolean;
  updateSettings: (updates: Partial<SettingsFormState>) => void;
  updateDirectory: (app: AppType, value?: string) => void;
  updateAppConfigDir: (value?: string) => void;
  browseDirectory: (app: AppType) => Promise<void>;
  browseAppConfigDir: () => Promise<void>;
  resetDirectory: (app: AppType) => Promise<void>;
  resetAppConfigDir: () => Promise<void>;
  saveSettings: () => Promise<SaveResult | null>;
  resetSettings: () => void;
  acknowledgeRestart: () => void;
}

export type { SettingsFormState, ResolvedDirectories };

const sanitizeDir = (value?: string | null): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

/**
 * useSettings - 组合层
 * 负责：
 * - 组合 useSettingsForm、useDirectorySettings、useSettingsMetadata
 * - 保存设置逻辑
 * - 重置设置逻辑
 */
export function useSettings(): UseSettingsResult {
  const { t } = useTranslation();
  const { data } = useSettingsQuery();
  const saveMutation = useSaveSettingsMutation();

  // 1️⃣ 表单状态管理
  const {
    settings,
    isLoading: isFormLoading,
    initialLanguage,
    updateSettings,
    resetSettings: resetForm,
    syncLanguage,
  } = useSettingsForm();

  // 2️⃣ 目录管理
  const {
    appConfigDir,
    resolvedDirs,
    isLoading: isDirectoryLoading,
    initialAppConfigDir,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    resetAllDirectories,
  } = useDirectorySettings({
    settings,
    onUpdateSettings: updateSettings,
  });

  // 3️⃣ 元数据管理
  const {
    isPortable,
    requiresRestart,
    isLoading: isMetadataLoading,
    acknowledgeRestart,
    setRequiresRestart,
  } = useSettingsMetadata();

  // 重置设置
  const resetSettings = useCallback(() => {
    resetForm(data ?? null);
    syncLanguage(initialLanguage);
    resetAllDirectories(
      sanitizeDir(data?.claudeConfigDir),
      sanitizeDir(data?.codexConfigDir),
    );
    setRequiresRestart(false);
  }, [
    data,
    initialLanguage,
    resetForm,
    syncLanguage,
    resetAllDirectories,
    setRequiresRestart,
  ]);

  // 保存设置
  const saveSettings = useCallback(async (): Promise<SaveResult | null> => {
    if (!settings) return null;
    try {
      const sanitizedAppDir = sanitizeDir(appConfigDir);
      const sanitizedClaudeDir = sanitizeDir(settings.claudeConfigDir);
      const sanitizedCodexDir = sanitizeDir(settings.codexConfigDir);
      const previousAppDir = initialAppConfigDir;

      const payload: Settings = {
        ...settings,
        claudeConfigDir: sanitizedClaudeDir,
        codexConfigDir: sanitizedCodexDir,
        language: settings.language,
      };

      await saveMutation.mutateAsync(payload);

      await settingsApi.setAppConfigDirOverride(sanitizedAppDir ?? null);

      try {
        if (payload.enableClaudePluginIntegration) {
          await settingsApi.applyClaudePluginConfig({ official: false });
        } else {
          await settingsApi.applyClaudePluginConfig({ official: true });
        }
      } catch (error) {
        console.warn(
          "[useSettings] Failed to sync Claude plugin config",
          error,
        );
        toast.error(
          t("notifications.syncClaudePluginFailed", {
            defaultValue: "同步 Claude 插件失败",
          }),
        );
      }

      try {
        if (typeof window !== "undefined") {
          window.localStorage.setItem("language", payload.language as Language);
        }
      } catch (error) {
        console.warn(
          "[useSettings] Failed to persist language preference",
          error,
        );
      }

      const appDirChanged = sanitizedAppDir !== (previousAppDir ?? undefined);
      setRequiresRestart(appDirChanged);

      return { requiresRestart: appDirChanged };
    } catch (error) {
      console.error("[useSettings] Failed to save settings", error);
      throw error;
    }
  }, [
    appConfigDir,
    initialAppConfigDir,
    saveMutation,
    settings,
    setRequiresRestart,
    t,
  ]);

  const isLoading = useMemo(
    () => isFormLoading || isDirectoryLoading || isMetadataLoading,
    [isFormLoading, isDirectoryLoading, isMetadataLoading],
  );

  return {
    settings,
    isLoading,
    isSaving: saveMutation.isPending,
    isPortable,
    appConfigDir,
    resolvedDirs,
    requiresRestart,
    updateSettings,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    saveSettings,
    resetSettings,
    acknowledgeRestart,
  };
}
