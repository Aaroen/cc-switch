import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { settingsApi } from "@/lib/api";

export interface UseSettingsMetadataResult {
  configPath: string;
  isPortable: boolean;
  requiresRestart: boolean;
  isLoading: boolean;
  openConfigFolder: () => Promise<void>;
  acknowledgeRestart: () => void;
  setRequiresRestart: (value: boolean) => void;
}

/**
 * useSettingsMetadata - 元数据管理
 * 负责：
 * - configPath（配置文件路径）
 * - isPortable（便携模式）
 * - requiresRestart（需要重启标志）
 * - 打开配置文件夹
 */
export function useSettingsMetadata(): UseSettingsMetadataResult {
  const { t } = useTranslation();

  const [configPath, setConfigPath] = useState("");
  const [isPortable, setIsPortable] = useState(false);
  const [requiresRestart, setRequiresRestart] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  // 加载元数据
  useEffect(() => {
    let active = true;
    setIsLoading(true);

    const load = async () => {
      try {
        const [appConfigPath, portable] = await Promise.all([
          settingsApi.getAppConfigPath(),
          settingsApi.isPortable(),
        ]);

        if (!active) return;

        setConfigPath(appConfigPath || "");
        setIsPortable(portable);
      } catch (error) {
        console.error(
          "[useSettingsMetadata] Failed to load metadata",
          error,
        );
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, []);

  const openConfigFolder = useCallback(async () => {
    try {
      await settingsApi.openAppConfigFolder();
    } catch (error) {
      console.error(
        "[useSettingsMetadata] Failed to open config folder",
        error,
      );
      toast.error(
        t("settings.openFolderFailed", {
          defaultValue: "打开目录失败",
        }),
      );
    }
  }, [t]);

  const acknowledgeRestart = useCallback(() => {
    setRequiresRestart(false);
  }, []);

  return {
    configPath,
    isPortable,
    requiresRestart,
    isLoading,
    openConfigFolder,
    acknowledgeRestart,
    setRequiresRestart,
  };
}
