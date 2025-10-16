import { useCallback, useEffect, useState } from "react";
import { Download, ExternalLink, Info, Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { getVersion } from "@tauri-apps/api/app";
import { settingsApi } from "@/lib/api";
import { useUpdate } from "@/contexts/UpdateContext";
import { relaunchApp } from "@/lib/updater";

interface AboutSectionProps {
  isPortable: boolean;
}

export function AboutSection({ isPortable }: AboutSectionProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [isLoadingVersion, setIsLoadingVersion] = useState(true);
  const [isDownloading, setIsDownloading] = useState(false);

  const {
    hasUpdate,
    updateInfo,
    updateHandle,
    checkUpdate,
    resetDismiss,
    isChecking,
  } = useUpdate();

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const loaded = await getVersion();
        if (active) {
          setVersion(loaded);
        }
      } catch (error) {
        console.error("[AboutSection] Failed to get version", error);
        if (active) {
          setVersion(null);
        }
      } finally {
        if (active) {
          setIsLoadingVersion(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, []);

  const handleOpenReleaseNotes = useCallback(async () => {
    try {
      const targetVersion = updateInfo?.availableVersion ?? version ?? "";
      const displayVersion = targetVersion.startsWith("v")
        ? targetVersion
        : targetVersion
          ? `v${targetVersion}`
          : "";

      if (!displayVersion) {
        await settingsApi.openExternal(
          "https://github.com/farion1231/cc-switch/releases",
        );
        return;
      }

      await settingsApi.openExternal(
        `https://github.com/farion1231/cc-switch/releases/tag/${displayVersion}`,
      );
    } catch (error) {
      console.error("[AboutSection] Failed to open release notes", error);
      toast.error(
        t("settings.openReleaseNotesFailed", {
          defaultValue: "打开更新日志失败",
        }),
      );
    }
  }, [t, updateInfo?.availableVersion, version]);

  const handleCheckUpdate = useCallback(async () => {
    if (hasUpdate && updateHandle) {
      if (isPortable) {
        try {
          await settingsApi.checkUpdates();
        } catch (error) {
          console.error("[AboutSection] Portable update failed", error);
        }
        return;
      }

      setIsDownloading(true);
      try {
        resetDismiss();
        await updateHandle.downloadAndInstall();
        await relaunchApp();
      } catch (error) {
        console.error("[AboutSection] Update failed", error);
        toast.error(
          t("settings.updateFailed", {
            defaultValue: "更新安装失败，已尝试打开下载页面。",
          }),
        );
        try {
          await settingsApi.checkUpdates();
        } catch (fallbackError) {
          console.error(
            "[AboutSection] Failed to open fallback updater",
            fallbackError,
          );
        }
      } finally {
        setIsDownloading(false);
      }
      return;
    }

    try {
      const available = await checkUpdate();
      if (!available) {
        toast.success(t("settings.upToDate", { defaultValue: "已是最新版本" }));
      }
    } catch (error) {
      console.error("[AboutSection] Check update failed", error);
      toast.error(
        t("settings.checkUpdateFailed", {
          defaultValue: "检查更新失败，请稍后重试。",
        }),
      );
    }
  }, [checkUpdate, hasUpdate, isPortable, resetDismiss, t, updateHandle]);

  const displayVersion =
    version ?? t("common.unknown", { defaultValue: "未知" });

  return (
    <section className="space-y-4">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">{t("common.about")}</h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.aboutHint", {
            defaultValue: "查看版本信息与更新状态。",
          })}
        </p>
      </header>

      <div className="space-y-4 rounded-lg border border-border p-4">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <p className="text-sm font-medium text-foreground">CC Switch</p>
            <p className="text-xs text-muted-foreground">
              {t("common.version")}{" "}
              {isLoadingVersion ? (
                <Loader2 className="inline h-3 w-3 animate-spin" />
              ) : (
                `v${displayVersion}`
              )}
            </p>
            {isPortable ? (
              <p className="inline-flex items-center gap-1 text-xs text-muted-foreground">
                <Info className="h-3 w-3" />
                {t("settings.portableMode", {
                  defaultValue: "当前为便携版，更新需手动下载。",
                })}
              </p>
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleOpenReleaseNotes}
            >
              <ExternalLink className="mr-2 h-4 w-4" />
              {t("settings.releaseNotes")}
            </Button>
            <Button
              type="button"
              size="sm"
              onClick={handleCheckUpdate}
              disabled={isChecking || isDownloading}
            >
              {isDownloading ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {t("settings.updating", { defaultValue: "安装更新..." })}
                </span>
              ) : hasUpdate ? (
                <span className="inline-flex items-center gap-2">
                  <Download className="h-4 w-4" />
                  {t("settings.updateTo", {
                    defaultValue: "更新到 {{version}}",
                    version: updateInfo?.availableVersion ?? "",
                  })}
                </span>
              ) : isChecking ? (
                <span className="inline-flex items-center gap-2">
                  <RefreshCw className="h-4 w-4 animate-spin" />
                  {t("settings.checking", { defaultValue: "检查中..." })}
                </span>
              ) : (
                t("settings.checkForUpdates")
              )}
            </Button>
          </div>
        </div>

        {hasUpdate && updateInfo ? (
          <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
            <p>
              {t("settings.updateAvailable", {
                defaultValue: "检测到新版本：{{version}}",
                version: updateInfo.availableVersion,
              })}
            </p>
            {updateInfo.notes ? (
              <p className="mt-1 line-clamp-3">{updateInfo.notes}</p>
            ) : null}
          </div>
        ) : null}
      </div>
    </section>
  );
}
