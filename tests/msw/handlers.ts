import { http, HttpResponse } from "msw";
import type { AppType } from "@/lib/api/types";
import type { Provider, Settings } from "@/types";
import {
  addProvider,
  deleteProvider,
  getCurrentProviderId,
  getProviders,
  listProviders,
  resetProviderState,
  setCurrentProviderId,
  updateProvider,
  updateSortOrder,
  getSettings,
  setSettings,
  getAppConfigDirOverride,
  setAppConfigDirOverrideState,
} from "./state";

const TAURI_ENDPOINT = "http://tauri.local";

const withJson = async <T>(request: Request): Promise<T> => {
  try {
    const body = await request.text();
    if (!body) return {} as T;
    return JSON.parse(body) as T;
  } catch {
    return {} as T;
  }
};

const success = <T>(payload: T) => HttpResponse.json(payload as any);

export const handlers = [
  http.post(`${TAURI_ENDPOINT}/get_providers`, async ({ request }) => {
    const { app_type } = await withJson<{ app_type: AppType }>(request);
    return success(getProviders(app_type));
  }),

  http.post(`${TAURI_ENDPOINT}/get_current_provider`, async ({ request }) => {
    const { app_type } = await withJson<{ app_type: AppType }>(request);
    return success(getCurrentProviderId(app_type));
  }),

  http.post(`${TAURI_ENDPOINT}/update_providers_sort_order`, async ({ request }) => {
    const { updates = [], app_type } = await withJson<{
      updates: { id: string; sortIndex: number }[];
      app_type: AppType;
    }>(request);
    updateSortOrder(app_type, updates);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/update_tray_menu`, () => success(true)),

  http.post(`${TAURI_ENDPOINT}/switch_provider`, async ({ request }) => {
    const { id, app_type } = await withJson<{ id: string; app_type: AppType }>(
      request,
    );
    const providers = listProviders(app_type);
    if (!providers[id]) {
      return HttpResponse.json(false, { status: 404 });
    }
    setCurrentProviderId(app_type, id);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/add_provider`, async ({ request }) => {
    const { provider, app_type } = await withJson<{
      provider: Provider & { id?: string };
      app_type: AppType;
    }>(request);

    const newId = provider.id ?? `mock-${Date.now()}`;
    addProvider(app_type, { ...provider, id: newId });
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/update_provider`, async ({ request }) => {
    const { provider, app_type } = await withJson<{
      provider: Provider;
      app_type: AppType;
    }>(request);
    updateProvider(app_type, provider);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/delete_provider`, async ({ request }) => {
    const { id, app_type } = await withJson<{ id: string; app_type: AppType }>(
      request,
    );
    deleteProvider(app_type, id);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/import_default_config`, async () => {
    resetProviderState();
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/open_external`, () => success(true)),

  http.post(`${TAURI_ENDPOINT}/restart_app`, () => success(true)),

  http.post(`${TAURI_ENDPOINT}/get_settings`, () => success(getSettings())),

  http.post(`${TAURI_ENDPOINT}/save_settings`, async ({ request }) => {
    const { settings } = await withJson<{ settings: Settings }>(request);
    setSettings(settings);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/set_app_config_dir_override`, async ({ request }) => {
    const { path } = await withJson<{ path: string | null }>(request);
    setAppConfigDirOverrideState(path ?? null);
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/get_app_config_dir_override`, () =>
    success(getAppConfigDirOverride()),
  ),

  http.post(`${TAURI_ENDPOINT}/apply_claude_plugin_config`, async ({ request }) => {
    const { official } = await withJson<{ official: boolean }>(request);
    setSettings({ enableClaudePluginIntegration: !official });
    return success(true);
  }),

  http.post(`${TAURI_ENDPOINT}/get_config_dir`, async ({ request }) => {
    const { app_type } = await withJson<{ app_type: AppType }>(request);
    return success(app_type === "claude" ? "/default/claude" : "/default/codex");
  }),

  http.post(`${TAURI_ENDPOINT}/is_portable_mode`, () => success(false)),

  http.post(`${TAURI_ENDPOINT}/select_config_directory`, async ({ request }) => {
    const { default_path } = await withJson<{ default_path?: string }>(request);
    return success(default_path ? `${default_path}/picked` : "/mock/selected-dir");
  }),

  http.post(`${TAURI_ENDPOINT}/pick_directory`, async ({ request }) => {
    const { default_path } = await withJson<{ default_path?: string }>(request);
    return success(default_path ? `${default_path}/picked` : "/mock/selected-dir");
  }),

  http.post(`${TAURI_ENDPOINT}/open_file_dialog`, () =>
    success("/mock/import-settings.json"),
  ),

  http.post(`${TAURI_ENDPOINT}/import_config_from_file`, async ({ request }) => {
    const { filePath } = await withJson<{ filePath: string }>(request);
    if (!filePath) {
      return success({ success: false, message: "Missing file" });
    }
    setSettings({ language: "en" });
    return success({ success: true, backupId: "backup-123" });
  }),

  http.post(`${TAURI_ENDPOINT}/export_config_to_file`, async ({ request }) => {
    const { filePath } = await withJson<{ filePath: string }>(request);
    if (!filePath) {
      return success({ success: false, message: "Invalid destination" });
    }
    return success({ success: true, filePath });
  }),

  http.post(`${TAURI_ENDPOINT}/save_file_dialog`, () =>
    success("/mock/export-settings.json"),
  ),
];
