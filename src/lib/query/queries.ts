import {
  useQuery,
  type UseQueryResult,
  keepPreviousData,
} from "@tanstack/react-query";
import { providersApi, settingsApi, usageApi, type AppType } from "@/lib/api";
import type { Provider, Settings, UsageResult } from "@/types";

const sortProviders = (
  providers: Record<string, Provider>,
): Record<string, Provider> => {
  const sortedEntries = Object.values(providers)
    .sort((a, b) => {
      const indexA = a.sortIndex ?? Number.MAX_SAFE_INTEGER;
      const indexB = b.sortIndex ?? Number.MAX_SAFE_INTEGER;
      if (indexA !== indexB) {
        return indexA - indexB;
      }

      const timeA = a.createdAt ?? 0;
      const timeB = b.createdAt ?? 0;
      if (timeA === timeB) {
        return a.name.localeCompare(b.name, "zh-CN");
      }
      return timeA - timeB;
    })
    .map((provider) => [provider.id, provider] as const);

  return Object.fromEntries(sortedEntries);
};

export interface ProvidersQueryData {
  providers: Record<string, Provider>;
  currentProviderId: string;
}

export const useProvidersQuery = (
  appType: AppType,
): UseQueryResult<ProvidersQueryData> => {
  return useQuery({
    queryKey: ["providers", appType],
    placeholderData: keepPreviousData,
    queryFn: async () => {
      let providers: Record<string, Provider> = {};
      let currentProviderId = "";

      try {
        providers = await providersApi.getAll(appType);
      } catch (error) {
        console.error("获取供应商列表失败:", error);
      }

      try {
        currentProviderId = await providersApi.getCurrent(appType);
      } catch (error) {
        console.error("获取当前供应商失败:", error);
      }

      if (Object.keys(providers).length === 0) {
        try {
          const success = await providersApi.importDefault(appType);
          if (success) {
            providers = await providersApi.getAll(appType);
            currentProviderId = await providersApi.getCurrent(appType);
          }
        } catch (error) {
          console.error("导入默认配置失败:", error);
        }
      }

      return {
        providers: sortProviders(providers),
        currentProviderId,
      };
    },
  });
};

export const useSettingsQuery = (): UseQueryResult<Settings> => {
  return useQuery({
    queryKey: ["settings"],
    queryFn: async () => settingsApi.get(),
  });
};

export const useUsageQuery = (
  providerId: string,
  appType: AppType,
  enabled: boolean = true,
): UseQueryResult<UsageResult> => {
  return useQuery({
    queryKey: ["usage", providerId, appType],
    queryFn: async () => usageApi.query(providerId, appType),
    enabled: enabled && !!providerId,
    refetchOnWindowFocus: false,
    staleTime: 5 * 60 * 1000, // 5分钟
  });
};
