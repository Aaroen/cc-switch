import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { homeDir, join } from "@tauri-apps/api/path";
import { settingsApi, type AppType } from "@/lib/api";
import { useSettingsQuery, useSaveSettingsMutation } from "@/lib/query";
import type { Settings } from "@/types";

type Language = "zh" | "en";

export type SettingsFormState = Omit<Settings, "language"> & {
  language: Language;
};

type DirectoryKey = "appConfig" | "claude" | "codex";

export interface ResolvedDirectories {
  appConfig: string;
  claude: string;
  codex: string;
}

interface SaveResult {
  requiresRestart: boolean;
}

export interface UseSettingsResult {
  settings: SettingsFormState | null;
  isLoading: boolean;
  isSaving: boolean;
  isPortable: boolean;
  configPath: string;
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
  openConfigFolder: () => Promise<void>;
  saveSettings: () => Promise<SaveResult | null>;
  resetSettings: () => void;
  acknowledgeRestart: () => void;
}

const normalizeLanguage = (lang?: string | null): Language => {
  if (!lang) return "zh";
  return lang === "en" ? "en" : "zh";
};

const sanitizeDir = (value?: string | null): string | undefined => {
  if (!value) return undefined;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

const computeDefaultAppConfigDir = async (): Promise<string | undefined> => {
  try {
    const home = await homeDir();
    return await join(home, ".cc-switch");
  } catch (error) {
    console.error(
      "[useSettings] Failed to resolve default app config dir",
      error,
    );
    return undefined;
  }
};

const computeDefaultConfigDir = async (
  app: AppType,
): Promise<string | undefined> => {
  try {
    const home = await homeDir();
    const folder = app === "claude" ? ".claude" : ".codex";
    return await join(home, folder);
  } catch (error) {
    console.error("[useSettings] Failed to resolve default config dir", error);
    return undefined;
  }
};

export function useSettings(): UseSettingsResult {
  const { t, i18n } = useTranslation();
  const { data, isLoading } = useSettingsQuery();
  const saveMutation = useSaveSettingsMutation();

  const [settingsState, setSettingsState] = useState<SettingsFormState | null>(
    null,
  );
  const [appConfigDir, setAppConfigDir] = useState<string | undefined>(
    undefined,
  );
  const [configPath, setConfigPath] = useState("");
  const [isPortable, setIsPortable] = useState(false);
  const [requiresRestart, setRequiresRestart] = useState(false);
  const [resolvedDirs, setResolvedDirs] = useState<ResolvedDirectories>({
    appConfig: "",
    claude: "",
    codex: "",
  });
  const [isAuxiliaryLoading, setIsAuxiliaryLoading] = useState(true);

  const defaultsRef = useRef<ResolvedDirectories>({
    appConfig: "",
    claude: "",
    codex: "",
  });
  const initialLanguageRef = useRef<Language>("zh");
  const initialAppConfigDirRef = useRef<string | undefined>(undefined);

  const readPersistedLanguage = useCallback((): Language => {
    if (typeof window !== "undefined") {
      const stored = window.localStorage.getItem("language");
      if (stored === "en" || stored === "zh") {
        return stored;
      }
    }
    return normalizeLanguage(i18n.language);
  }, [i18n.language]);

  const syncLanguage = useCallback(
    (lang: Language) => {
      const current = normalizeLanguage(i18n.language);
      if (current !== lang) {
        void i18n.changeLanguage(lang);
      }
    },
    [i18n],
  );

  // 初始化设置数据
  useEffect(() => {
    if (!data) return;

    const normalizedLanguage = normalizeLanguage(
      data.language ?? readPersistedLanguage(),
    );

    const normalized: SettingsFormState = {
      ...data,
      showInTray: data.showInTray ?? true,
      minimizeToTrayOnClose: data.minimizeToTrayOnClose ?? true,
      enableClaudePluginIntegration:
        data.enableClaudePluginIntegration ?? false,
      claudeConfigDir: sanitizeDir(data.claudeConfigDir),
      codexConfigDir: sanitizeDir(data.codexConfigDir),
      language: normalizedLanguage,
    };

    setSettingsState(normalized);
    initialLanguageRef.current = normalizedLanguage;
    syncLanguage(normalizedLanguage);
  }, [data, readPersistedLanguage, syncLanguage]);

  // 加载辅助信息（目录、配置路径、便携模式）
  useEffect(() => {
    let active = true;
    setIsAuxiliaryLoading(true);

    const load = async () => {
      try {
        const [
          overrideRaw,
          appConfigPath,
          claudeDir,
          codexDir,
          portable,
          defaultAppConfig,
          defaultClaudeDir,
          defaultCodexDir,
        ] = await Promise.all([
          settingsApi.getAppConfigDirOverride(),
          settingsApi.getAppConfigPath(),
          settingsApi.getConfigDir("claude"),
          settingsApi.getConfigDir("codex"),
          settingsApi.isPortable(),
          computeDefaultAppConfigDir(),
          computeDefaultConfigDir("claude"),
          computeDefaultConfigDir("codex"),
        ]);

        if (!active) return;

        const normalizedOverride = sanitizeDir(overrideRaw ?? undefined);

        defaultsRef.current = {
          appConfig: defaultAppConfig ?? "",
          claude: defaultClaudeDir ?? "",
          codex: defaultCodexDir ?? "",
        };

        setAppConfigDir(normalizedOverride);
        initialAppConfigDirRef.current = normalizedOverride;

        setResolvedDirs({
          appConfig: normalizedOverride ?? defaultsRef.current.appConfig,
          claude: claudeDir || defaultsRef.current.claude,
          codex: codexDir || defaultsRef.current.codex,
        });

        setConfigPath(appConfigPath || "");
        setIsPortable(portable);
      } catch (error) {
        console.error("[useSettings] Failed to load directory info", error);
      } finally {
        if (active) {
          setIsAuxiliaryLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, []);

  const updateSettings = useCallback(
    (updates: Partial<SettingsFormState>) => {
      setSettingsState((prev) => {
        const base =
          prev ??
          ({
            showInTray: true,
            minimizeToTrayOnClose: true,
            enableClaudePluginIntegration: false,
            language: readPersistedLanguage(),
          } as SettingsFormState);

        const next: SettingsFormState = {
          ...base,
          ...updates,
        };

        if (updates.language) {
          const normalized = normalizeLanguage(updates.language);
          next.language = normalized;
          syncLanguage(normalized);
        }

        return next;
      });
    },
    [readPersistedLanguage, syncLanguage],
  );

  const updateDirectoryState = useCallback(
    (key: DirectoryKey, value?: string) => {
      const sanitized = sanitizeDir(value);
      if (key === "appConfig") {
        setAppConfigDir(sanitized);
      } else {
        setSettingsState((prev) => {
          if (!prev) return prev;
          if (key === "claude") {
            return {
              ...prev,
              claudeConfigDir: sanitized,
            };
          }
          return {
            ...prev,
            codexConfigDir: sanitized,
          };
        });
      }

      setResolvedDirs((prev) => ({
        ...prev,
        [key]: sanitized ?? defaultsRef.current[key],
      }));
    },
    [],
  );

  const updateAppConfigDir = useCallback(
    (value?: string) => {
      updateDirectoryState("appConfig", value);
    },
    [updateDirectoryState],
  );

  const updateDirectory = useCallback(
    (app: AppType, value?: string) => {
      updateDirectoryState(app === "claude" ? "claude" : "codex", value);
    },
    [updateDirectoryState],
  );

  const browseDirectory = useCallback(
    async (app: AppType) => {
      const key: DirectoryKey = app === "claude" ? "claude" : "codex";
      const currentValue =
        key === "claude"
          ? (settingsState?.claudeConfigDir ?? resolvedDirs.claude)
          : (settingsState?.codexConfigDir ?? resolvedDirs.codex);

      try {
        const picked = await settingsApi.selectConfigDirectory(currentValue);
        const sanitized = sanitizeDir(picked ?? undefined);
        if (!sanitized) return;
        updateDirectoryState(key, sanitized);
      } catch (error) {
        console.error("[useSettings] Failed to pick directory", error);
        toast.error(
          t("settings.selectFileFailed", {
            defaultValue: "选择目录失败",
          }),
        );
      }
    },
    [settingsState, resolvedDirs, t, updateDirectoryState],
  );

  const browseAppConfigDir = useCallback(async () => {
    const currentValue = appConfigDir ?? resolvedDirs.appConfig;
    try {
      const picked = await settingsApi.selectConfigDirectory(currentValue);
      const sanitized = sanitizeDir(picked ?? undefined);
      if (!sanitized) return;
      updateDirectoryState("appConfig", sanitized);
    } catch (error) {
      console.error("[useSettings] Failed to pick app config directory", error);
      toast.error(
        t("settings.selectFileFailed", {
          defaultValue: "选择目录失败",
        }),
      );
    }
  }, [appConfigDir, resolvedDirs.appConfig, t, updateDirectoryState]);

  const resetDirectory = useCallback(
    async (app: AppType) => {
      const key: DirectoryKey = app === "claude" ? "claude" : "codex";
      if (!defaultsRef.current[key]) {
        const fallback = await computeDefaultConfigDir(app);
        if (fallback) {
          defaultsRef.current = {
            ...defaultsRef.current,
            [key]: fallback,
          };
        }
      }
      updateDirectoryState(key, undefined);
    },
    [updateDirectoryState],
  );

  const resetAppConfigDir = useCallback(async () => {
    if (!defaultsRef.current.appConfig) {
      const fallback = await computeDefaultAppConfigDir();
      if (fallback) {
        defaultsRef.current = {
          ...defaultsRef.current,
          appConfig: fallback,
        };
      }
    }
    updateDirectoryState("appConfig", undefined);
  }, [updateDirectoryState]);

  const openConfigFolder = useCallback(async () => {
    try {
      await settingsApi.openAppConfigFolder();
    } catch (error) {
      console.error("[useSettings] Failed to open config folder", error);
      toast.error(
        t("settings.openFolderFailed", {
          defaultValue: "打开目录失败",
        }),
      );
    }
  }, [t]);

  const resetSettings = useCallback(() => {
    if (!data) return;

    const normalizedLanguage = normalizeLanguage(
      data.language ?? readPersistedLanguage(),
    );

    const normalized: SettingsFormState = {
      ...data,
      showInTray: data.showInTray ?? true,
      minimizeToTrayOnClose: data.minimizeToTrayOnClose ?? true,
      enableClaudePluginIntegration:
        data.enableClaudePluginIntegration ?? false,
      claudeConfigDir: sanitizeDir(data.claudeConfigDir),
      codexConfigDir: sanitizeDir(data.codexConfigDir),
      language: normalizedLanguage,
    };

    setSettingsState(normalized);
    syncLanguage(initialLanguageRef.current);
    setAppConfigDir(initialAppConfigDirRef.current);
    setResolvedDirs({
      appConfig:
        initialAppConfigDirRef.current ?? defaultsRef.current.appConfig,
      claude: normalized.claudeConfigDir ?? defaultsRef.current.claude,
      codex: normalized.codexConfigDir ?? defaultsRef.current.codex,
    });
    setRequiresRestart(false);
  }, [data, readPersistedLanguage, syncLanguage]);

  const acknowledgeRestart = useCallback(() => {
    setRequiresRestart(false);
  }, []);

  const saveSettings = useCallback(async (): Promise<SaveResult | null> => {
    if (!settingsState) return null;
    try {
      const sanitizedAppDir = sanitizeDir(appConfigDir);
      const sanitizedClaudeDir = sanitizeDir(settingsState.claudeConfigDir);
      const sanitizedCodexDir = sanitizeDir(settingsState.codexConfigDir);
      const previousAppDir = initialAppConfigDirRef.current;
      const payload: Settings = {
        ...settingsState,
        claudeConfigDir: sanitizedClaudeDir,
        codexConfigDir: sanitizedCodexDir,
        language: settingsState.language,
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

      initialLanguageRef.current = payload.language as Language;
      setSettingsState((prev) =>
        prev
          ? {
              ...prev,
              claudeConfigDir: sanitizedClaudeDir,
              codexConfigDir: sanitizedCodexDir,
              language: payload.language as Language,
            }
          : prev,
      );

      setResolvedDirs({
        appConfig: sanitizedAppDir ?? defaultsRef.current.appConfig,
        claude: sanitizedClaudeDir ?? defaultsRef.current.claude,
        codex: sanitizedCodexDir ?? defaultsRef.current.codex,
      });
      setAppConfigDir(sanitizedAppDir);

      const appDirChanged = sanitizedAppDir !== (previousAppDir ?? undefined);
      initialAppConfigDirRef.current = sanitizedAppDir;
      setRequiresRestart(appDirChanged);

      return { requiresRestart: appDirChanged };
    } catch (error) {
      console.error("[useSettings] Failed to save settings", error);
      throw error;
    }
  }, [appConfigDir, saveMutation, settingsState, t]);

  const isBusy = useMemo(
    () => isLoading || isAuxiliaryLoading,
    [isLoading, isAuxiliaryLoading],
  );

  return {
    settings: settingsState,
    isLoading: isBusy,
    isSaving: saveMutation.isPending,
    isPortable,
    configPath,
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
    openConfigFolder,
    saveSettings,
    resetSettings,
    acknowledgeRestart,
  };
}
