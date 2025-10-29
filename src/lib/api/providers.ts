import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Provider } from "@/types";
import type { AppType } from "./types";

export interface ProviderSortUpdate {
  id: string;
  sortIndex: number;
}

export interface ProviderSwitchEvent {
  appType: AppType;
  providerId: string;
}

export const providersApi = {
  async getAll(appType: AppType): Promise<Record<string, Provider>> {
    return await invoke("get_providers", { app: appType });
  },

  async getCurrent(appType: AppType): Promise<string> {
    return await invoke("get_current_provider", { app: appType });
  },

  async add(provider: Provider, appType: AppType): Promise<boolean> {
    return await invoke("add_provider", { provider, app: appType });
  },

  async update(provider: Provider, appType: AppType): Promise<boolean> {
    return await invoke("update_provider", { provider, app: appType });
  },

  async delete(id: string, appType: AppType): Promise<boolean> {
    return await invoke("delete_provider", { id, app: appType });
  },

  async switch(id: string, appType: AppType): Promise<boolean> {
    return await invoke("switch_provider", { id, app: appType });
  },

  async importDefault(appType: AppType): Promise<boolean> {
    return await invoke("import_default_config", { app: appType });
  },

  async updateTrayMenu(): Promise<boolean> {
    return await invoke("update_tray_menu");
  },

  async updateSortOrder(
    updates: ProviderSortUpdate[],
    appType: AppType,
  ): Promise<boolean> {
    return await invoke("update_providers_sort_order", { updates, app: appType });
  },

  async onSwitched(
    handler: (event: ProviderSwitchEvent) => void,
  ): Promise<UnlistenFn> {
    return await listen("provider-switched", (event) => {
      const payload = event.payload as ProviderSwitchEvent;
      handler(payload);
    });
  },
};
