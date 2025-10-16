import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppType } from "@/lib/api";

export interface ProviderSwitchedPayload {
  appType: AppType;
  providerId: string;
}

export const tauriEvents = {
  onProviderSwitched: async (
    handler: (payload: ProviderSwitchedPayload) => void,
  ): Promise<UnlistenFn> => {
    return await listen("provider-switched", (event) => {
      handler(event.payload as ProviderSwitchedPayload);
    });
  },
};
