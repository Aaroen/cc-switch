import type { AppType } from "@/lib/api/types";
import type { Provider } from "@/types";

type ProvidersByApp = Record<AppType, Record<string, Provider>>;
type CurrentProviderState = Record<AppType, string>;

const createDefaultProviders = (): ProvidersByApp => ({
  claude: {
    "claude-1": {
      id: "claude-1",
      name: "Claude Default",
      settingsConfig: {},
      category: "official",
      sortIndex: 0,
      createdAt: Date.now(),
    },
    "claude-2": {
      id: "claude-2",
      name: "Claude Custom",
      settingsConfig: {},
      category: "custom",
      sortIndex: 1,
      createdAt: Date.now() + 1,
    },
  },
  codex: {
    "codex-1": {
      id: "codex-1",
      name: "Codex Default",
      settingsConfig: {},
      category: "official",
      sortIndex: 0,
      createdAt: Date.now(),
    },
    "codex-2": {
      id: "codex-2",
      name: "Codex Secondary",
      settingsConfig: {},
      category: "custom",
      sortIndex: 1,
      createdAt: Date.now() + 1,
    },
  },
});

const createDefaultCurrent = (): CurrentProviderState => ({
  claude: "claude-1",
  codex: "codex-1",
});

let providers = createDefaultProviders();
let current = createDefaultCurrent();

const cloneProviders = (value: ProvidersByApp) =>
  JSON.parse(JSON.stringify(value)) as ProvidersByApp;

export const resetProviderState = () => {
  providers = createDefaultProviders();
  current = createDefaultCurrent();
};

export const getProviders = (appType: AppType) =>
  cloneProviders(providers)[appType] ?? {};

export const getCurrentProviderId = (appType: AppType) => current[appType] ?? "";

export const setCurrentProviderId = (appType: AppType, providerId: string) => {
  current[appType] = providerId;
};

export const updateProviders = (appType: AppType, data: Record<string, Provider>) => {
  providers[appType] = cloneProviders({ [appType]: data } as ProvidersByApp)[appType];
};

export const setProviders = (appType: AppType, data: Record<string, Provider>) => {
  providers[appType] = JSON.parse(JSON.stringify(data)) as Record<string, Provider>;
};

export const addProvider = (appType: AppType, provider: Provider) => {
  providers[appType] = providers[appType] ?? {};
  providers[appType][provider.id] = provider;
};

export const updateProvider = (appType: AppType, provider: Provider) => {
  if (!providers[appType]) return;
  providers[appType][provider.id] = {
    ...providers[appType][provider.id],
    ...provider,
  };
};

export const deleteProvider = (appType: AppType, providerId: string) => {
  if (!providers[appType]) return;
  delete providers[appType][providerId];
  if (current[appType] === providerId) {
    const fallback = Object.keys(providers[appType])[0] ?? "";
    current[appType] = fallback;
  }
};

export const updateSortOrder = (
  appType: AppType,
  updates: { id: string; sortIndex: number }[],
) => {
  if (!providers[appType]) return;
  updates.forEach(({ id, sortIndex }) => {
    const provider = providers[appType][id];
    if (provider) {
      providers[appType][id] = { ...provider, sortIndex };
    }
  });
};

export const listProviders = (appType: AppType) =>
  JSON.parse(JSON.stringify(providers[appType] ?? {})) as Record<string, Provider>;

