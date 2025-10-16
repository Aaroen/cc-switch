import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import { Plus, Settings } from "lucide-react";
import type { Provider, UsageScript } from "@/types";
import {
  useProvidersQuery,
  useAddProviderMutation,
  useUpdateProviderMutation,
  useDeleteProviderMutation,
  useSwitchProviderMutation,
} from "@/lib/query";
import { providersApi, type AppType } from "@/lib/api";
import { extractErrorMessage } from "@/utils/errorUtils";
import { AppSwitcher } from "@/components/AppSwitcher";
import { ModeToggle } from "@/components/mode-toggle";
import { ProviderList } from "@/components/providers/ProviderList";
import { AddProviderDialog } from "@/components/providers/AddProviderDialog";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import { UpdateBadge } from "@/components/UpdateBadge";
import UsageScriptModal from "@/components/UsageScriptModal";
import McpPanel from "@/components/mcp/McpPanel";
import { Button } from "@/components/ui/button";

interface ProviderSwitchEvent {
  appType: string;
  providerId: string;
}

function App() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [activeApp, setActiveApp] = useState<AppType>("claude");
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [isAddOpen, setIsAddOpen] = useState(false);
  const [isMcpOpen, setIsMcpOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [usageProvider, setUsageProvider] = useState<Provider | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<Provider | null>(null);

  const { data, isLoading, refetch } = useProvidersQuery(activeApp);
  const providers = useMemo(() => data?.providers ?? {}, [data]);
  const currentProviderId = data?.currentProviderId ?? "";

  const addProviderMutation = useAddProviderMutation(activeApp);
  const updateProviderMutation = useUpdateProviderMutation(activeApp);
  const deleteProviderMutation = useDeleteProviderMutation(activeApp);
  const switchProviderMutation = useSwitchProviderMutation(activeApp);

  useEffect(() => {
    let unsubscribe: (() => void) | undefined;

    const setupListener = async () => {
      try {
        unsubscribe = await window.api.onProviderSwitched(
          async (event: ProviderSwitchEvent) => {
            if (event.appType === activeApp) {
              await refetch();
            }
          },
        );
      } catch (error) {
        console.error("[App] Failed to subscribe provider switch event", error);
      }
    };

    setupListener();
    return () => {
      unsubscribe?.();
    };
  }, [activeApp, refetch]);

  const handleNotify = useCallback(
    (message: string, type: "success" | "error", duration?: number) => {
      const options = duration ? { duration } : undefined;
      if (type === "error") {
        toast.error(message, options);
      } else {
        toast.success(message, options);
      }
    },
    [],
  );

  const handleOpenWebsite = useCallback(
    async (url: string) => {
      try {
        await window.api.openExternal(url);
      } catch (error) {
        const detail =
          extractErrorMessage(error) ||
          t("notifications.openLinkFailed", {
            defaultValue: "链接打开失败",
          });
        toast.error(detail);
      }
    },
    [t],
  );

  const handleAddProvider = useCallback(
    async (provider: Omit<Provider, "id">) => {
      await addProviderMutation.mutateAsync(provider);
    },
    [addProviderMutation],
  );

  const handleEditProvider = useCallback(
    async (provider: Provider) => {
      try {
        await updateProviderMutation.mutateAsync(provider);
        await providersApi.updateTrayMenu();
        setEditingProvider(null);
      } catch {
        // 错误提示由 mutation 统一处理
      }
    },
    [updateProviderMutation],
  );

  const handleSyncClaudePlugin = useCallback(
    async (provider: Provider) => {
      if (activeApp !== "claude") return;

      try {
        const settings = await window.api.getSettings();
        if (!settings?.enableClaudePluginIntegration) {
          return;
        }

        const isOfficial = provider.category === "official";
        await window.api.applyClaudePluginConfig({ official: isOfficial });

        toast.success(
          isOfficial
            ? t("notifications.appliedToClaudePlugin", {
                defaultValue: "已同步为官方配置",
              })
            : t("notifications.removedFromClaudePlugin", {
                defaultValue: "已移除 Claude 插件配置",
              }),
          { duration: 2200 },
        );
      } catch (error) {
        const detail =
          extractErrorMessage(error) ||
          t("notifications.syncClaudePluginFailed", {
            defaultValue: "同步 Claude 插件失败",
          });
        toast.error(detail, { duration: 4200 });
      }
    },
    [activeApp, t],
  );

  const handleSwitchProvider = useCallback(
    async (provider: Provider) => {
      try {
        await switchProviderMutation.mutateAsync(provider.id);
        await handleSyncClaudePlugin(provider);
      } catch {
        // 错误提示由 mutation 与同步函数处理
      }
    },
    [switchProviderMutation, handleSyncClaudePlugin],
  );

  const handleRequestDelete = useCallback((provider: Provider) => {
    setConfirmDelete(provider);
  }, []);

  const handleConfirmDelete = useCallback(async () => {
    if (!confirmDelete) return;
    try {
      await deleteProviderMutation.mutateAsync(confirmDelete.id);
    } finally {
      setConfirmDelete(null);
    }
  }, [confirmDelete, deleteProviderMutation]);

  const handleImportSuccess = useCallback(async () => {
    await refetch();
    try {
      await providersApi.updateTrayMenu();
    } catch (error) {
      console.error("[App] Failed to refresh tray menu", error);
    }
  }, [refetch]);

  const handleSaveUsageScript = useCallback(
    async (provider: Provider, script: UsageScript) => {
      try {
        const updatedProvider: Provider = {
          ...provider,
          meta: {
            ...provider.meta,
            usage_script: script,
          },
        };

        await providersApi.update(updatedProvider, activeApp);
        await queryClient.invalidateQueries({
          queryKey: ["providers", activeApp],
        });
        toast.success(
          t("provider.usageSaved", {
            defaultValue: "用量查询配置已保存",
          }),
        );
      } catch (error) {
        const detail =
          extractErrorMessage(error) ||
          t("provider.usageSaveFailed", {
            defaultValue: "用量查询配置保存失败",
          });
        toast.error(detail);
      }
    },
    [activeApp, queryClient, t],
  );

  return (
    <div className="flex h-screen flex-col bg-gray-50 dark:bg-gray-950">
      <header className="flex-shrink-0 border-b border-gray-200 bg-white px-6 py-4 dark:border-gray-800 dark:bg-gray-900">
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <a
              href="https://github.com/farion1231/cc-switch"
              target="_blank"
              rel="noreferrer"
              className="text-xl font-semibold text-blue-500 transition-colors hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
            >
              CC Switch
            </a>
            <ModeToggle />
            <Button
              variant="ghost"
              size="icon"
              onClick={() => setIsSettingsOpen(true)}
            >
              <Settings className="h-4 w-4" />
            </Button>
            <UpdateBadge onClick={() => setIsSettingsOpen(true)} />
          </div>

          <div className="flex flex-wrap items-center gap-3">
            <AppSwitcher activeApp={activeApp} onSwitch={setActiveApp} />
            <Button
              variant="outline"
              onClick={() => setIsMcpOpen(true)}
            >
              MCP
            </Button>
            <Button onClick={() => setIsAddOpen(true)}>
              <Plus className="h-4 w-4" />
              {t("header.addProvider", { defaultValue: "添加供应商" })}
            </Button>
          </div>
        </div>
      </header>

      <main className="flex-1 overflow-y-auto">
        <div className="mx-auto max-w-4xl px-6 py-6">
          <ProviderList
            providers={providers}
            currentProviderId={currentProviderId}
            appType={activeApp}
            isLoading={isLoading}
            onSwitch={handleSwitchProvider}
            onEdit={setEditingProvider}
            onDelete={handleRequestDelete}
            onConfigureUsage={setUsageProvider}
            onOpenWebsite={handleOpenWebsite}
            onCreate={() => setIsAddOpen(true)}
          />
        </div>
      </main>

      <AddProviderDialog
        open={isAddOpen}
        onOpenChange={setIsAddOpen}
        appType={activeApp}
        onSubmit={handleAddProvider}
      />

      <EditProviderDialog
        open={Boolean(editingProvider)}
        provider={editingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setEditingProvider(null);
          }
        }}
        onSubmit={handleEditProvider}
      />

      {usageProvider && (
        <UsageScriptModal
          provider={usageProvider}
          appType={activeApp}
          onClose={() => setUsageProvider(null)}
          onSave={(script) => {
            void handleSaveUsageScript(usageProvider, script);
          }}
          onNotify={handleNotify}
        />
      )}

      <ConfirmDialog
        isOpen={Boolean(confirmDelete)}
        title={t("confirm.deleteProvider", { defaultValue: "删除供应商" })}
        message={
          confirmDelete
            ? t("confirm.deleteProviderMessage", {
                name: confirmDelete.name,
                defaultValue: `确定删除 ${confirmDelete.name} 吗？`,
              })
            : ""
        }
        onConfirm={() => void handleConfirmDelete()}
        onCancel={() => setConfirmDelete(null)}
      />

      <SettingsDialog
        open={isSettingsOpen}
        onOpenChange={setIsSettingsOpen}
        onImportSuccess={handleImportSuccess}
      />

      {isMcpOpen && (
        <McpPanel
          appType={activeApp}
          onClose={() => setIsMcpOpen(false)}
          onNotify={handleNotify}
        />
      )}
    </div>
  );
}

export default App;
