import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { providersApi, settingsApi, type AppType } from "@/lib/api";
import type { Provider, Settings } from "@/types";

export const useAddProviderMutation = (appType: AppType) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (providerInput: Omit<Provider, "id">) => {
      const newProvider: Provider = {
        ...providerInput,
        id: crypto.randomUUID(),
        createdAt: Date.now(),
      };
      await providersApi.add(newProvider, appType);
      return newProvider;
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appType] });
      await providersApi.updateTrayMenu();
      toast.success(
        t("notifications.providerAdded", {
          defaultValue: "供应商已添加",
        }),
      );
    },
    onError: (error: Error) => {
      toast.error(
        t("notifications.addFailed", {
          defaultValue: "添加供应商失败: {{error}}",
          error: error.message,
        }),
      );
    },
  });
};

export const useUpdateProviderMutation = (appType: AppType) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (provider: Provider) => {
      await providersApi.update(provider, appType);
      return provider;
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appType] });
      toast.success(
        t("notifications.updateSuccess", {
          defaultValue: "供应商更新成功",
        }),
      );
    },
    onError: (error: Error) => {
      toast.error(
        t("notifications.updateFailed", {
          defaultValue: "更新供应商失败: {{error}}",
          error: error.message,
        }),
      );
    },
  });
};

export const useDeleteProviderMutation = (appType: AppType) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (providerId: string) => {
      await providersApi.delete(providerId, appType);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appType] });
      await providersApi.updateTrayMenu();
      toast.success(
        t("notifications.deleteSuccess", {
          defaultValue: "供应商已删除",
        }),
      );
    },
    onError: (error: Error) => {
      toast.error(
        t("notifications.deleteFailed", {
          defaultValue: "删除供应商失败: {{error}}",
          error: error.message,
        }),
      );
    },
  });
};

export const useSwitchProviderMutation = (appType: AppType) => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (providerId: string) => {
      return await providersApi.switch(providerId, appType);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["providers", appType] });
      await providersApi.updateTrayMenu();
      toast.success(
        t("notifications.switchSuccess", {
          defaultValue: "切换供应商成功",
          appName: t(`apps.${appType}`, { defaultValue: appType }),
        }),
      );
    },
    onError: (error: Error) => {
      toast.error(
        t("notifications.switchFailed", {
          defaultValue: "切换供应商失败: {{error}}",
          error: error.message,
        }),
      );
    },
  });
};

export const useSaveSettingsMutation = () => {
  const queryClient = useQueryClient();
  const { t } = useTranslation();

  return useMutation({
    mutationFn: async (settings: Settings) => {
      await settingsApi.save(settings);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["settings"] });
      toast.success(
        t("notifications.settingsSaved", {
          defaultValue: "设置已保存",
        }),
      );
    },
    onError: (error: Error) => {
      toast.error(
        t("notifications.settingsSaveFailed", {
          defaultValue: "保存设置失败: {{error}}",
          error: error.message,
        }),
      );
    },
  });
};
