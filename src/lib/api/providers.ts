import { invoke } from "@tauri-apps/api/core";
import type { Provider } from "@/types";
import type { AppType } from "./types";

export interface ProviderSortUpdate {
  id: string;
  sortIndex: number;
}

export const providersApi = {
  async getAll(appType: AppType): Promise<Record<string, Provider>> {
    return await invoke("get_providers", { app_type: appType, app: appType });
  },

  async getCurrent(appType: AppType): Promise<string> {
    return await invoke("get_current_provider", {
      app_type: appType,
      app: appType,
    });
  },

  async add(provider: Provider, appType: AppType): Promise<boolean> {
    return await invoke("add_provider", {
      provider,
      app_type: appType,
      app: appType,
    });
  },

  async update(provider: Provider, appType: AppType): Promise<boolean> {
    return await invoke("update_provider", {
      provider,
      app_type: appType,
      app: appType,
    });
  },

  async delete(id: string, appType: AppType): Promise<boolean> {
    return await invoke("delete_provider", {
      id,
      app_type: appType,
      app: appType,
    });
  },

  async switch(id: string, appType: AppType): Promise<boolean> {
    return await invoke("switch_provider", {
      id,
      app_type: appType,
      app: appType,
    });
  },

  async importDefault(appType: AppType): Promise<boolean> {
    return await invoke("import_default_config", {
      app_type: appType,
      app: appType,
    });
  },

  async updateTrayMenu(): Promise<boolean> {
    return await invoke("update_tray_menu");
  },

  async updateSortOrder(
    updates: ProviderSortUpdate[],
    appType: AppType
  ): Promise<boolean> {
    return await invoke("update_providers_sort_order", {
      updates,
      app_type: appType,
      app: appType,
    });
  },
};
