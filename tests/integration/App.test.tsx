import { Suspense } from "react";
import { describe, it, expect, vi, beforeEach, beforeAll } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

const {
  toastSuccessMock,
  toastErrorMock,
  deleteProviderMock,
  addProviderMock,
  updateProviderMock,
  saveUsageScriptMock,
  onSwitchedMock,
  updateSortOrderMock,
  updateTrayMenuMock,
  openExternalMock,
  useProvidersQueryMock,
  useProviderActionsMock,
  providersDataMock,
  getAllMock,
  getCurrentMock,
  importDefaultMock,
} = vi.hoisted(() => {
  const deleteProviderMock = vi.fn();
  const addProviderMock = vi.fn();
  const updateProviderMock = vi.fn();
  const saveUsageScriptMock = vi.fn();
  const onSwitchedMock = vi.fn();
  const updateSortOrderMock = vi.fn();
  const updateTrayMenuMock = vi.fn();
  const openExternalMock = vi.fn();
  const useProvidersQueryMock = vi.fn();
  const getAllMock = vi.fn();
  const getCurrentMock = vi.fn();
  const importDefaultMock = vi.fn();
  const toastSuccessMock = vi.fn();
  const toastErrorMock = vi.fn();

  const providersDataMock = {
    claude: [
      {
        id: "claude-1",
        name: "Claude Default",
        settingsConfig: {},
        category: "default",
        sortIndex: 0,
      },
      {
        id: "claude-2",
        name: "Claude Custom",
        settingsConfig: {},
        category: "custom",
        sortIndex: 1,
      },
    ],
    codex: [
      {
        id: "codex-1",
        name: "Codex Default",
        settingsConfig: {},
        category: "default",
        sortIndex: 0,
      },
      {
        id: "codex-2",
        name: "Codex Secondary",
        settingsConfig: {},
        category: "custom",
        sortIndex: 1,
      },
    ],
  };

  const useProviderActionsMock = vi.fn(() => ({
    addProvider: addProviderMock,
    updateProvider: updateProviderMock,
    deleteProvider: deleteProviderMock,
    saveUsageScript: saveUsageScriptMock,
  }));

  return {
    toastSuccessMock,
    toastErrorMock,
    deleteProviderMock,
    addProviderMock,
    updateProviderMock,
    saveUsageScriptMock,
    onSwitchedMock,
    updateSortOrderMock,
    updateTrayMenuMock,
    openExternalMock,
    useProvidersQueryMock,
    useProviderActionsMock,
    providersDataMock,
    getAllMock,
    getCurrentMock,
    importDefaultMock,
  };
});

vi.mock("@/lib/query", () => ({
  useProvidersQuery: (...args: unknown[]) => useProvidersQueryMock(...args),
}));

vi.mock("@/hooks/useProviderActions", () => ({
  useProviderActions: () => useProviderActionsMock(),
}));

vi.mock("@/lib/api", () => ({
  providersApi: {
    onSwitched: onSwitchedMock,
    updateSortOrder: updateSortOrderMock,
    updateTrayMenu: updateTrayMenuMock,
    getAll: getAllMock,
    getCurrent: getCurrentMock,
    importDefault: importDefaultMock,
  },
  settingsApi: {
    openExternal: openExternalMock,
  },
}));

vi.mock("sonner", () => ({
  toast: {
    success: toastSuccessMock,
    error: toastErrorMock,
  },
}));

vi.mock("react-i18next", async () => {
  const actual = await vi.importActual<typeof import("react-i18next")>("react-i18next");
  return {
    ...actual,
    useTranslation: () => ({
      t: (key: string, options?: Record<string, unknown>) =>
        options?.defaultValue ?? key,
    }),
  };
});


vi.mock("@/components/providers/ProviderList", () => ({
  ProviderList: ({
    providers,
    currentProviderId,
    onSwitch,
    onEdit,
    onDelete,
    onDuplicate,
    onConfigureUsage,
    onOpenWebsite,
    onCreate,
  }: any) => (
    <div>
      <div data-testid="provider-list">{JSON.stringify(providers)}</div>
      <button onClick={() => onSwitch({ id: currentProviderId })}>switch</button>
      <button onClick={() => onEdit({ id: currentProviderId })}>edit</button>
      <button onClick={() => onDelete({ id: currentProviderId })}>delete</button>
      <button
        onClick={() =>
          onDuplicate(
            providers[currentProviderId] ?? {
              id: "claude-1",
              name: "Claude Default",
              settingsConfig: {},
              sortIndex: 0,
            },
          )
        }
      >
        duplicate
      </button>
      <button onClick={() => onConfigureUsage({ id: currentProviderId })}>
        usage
      </button>
      <button onClick={() => onOpenWebsite("https://example.com")}>
        open-website
      </button>
      <button onClick={() => onCreate?.()}>create</button>
    </div>
  ),
}));

vi.mock("@/components/providers/AddProviderDialog", () => ({
  AddProviderDialog: ({ open, onOpenChange, onSubmit, appType }: any) =>
    open ? (
      <div data-testid="add-provider-dialog">
        <button onClick={() => onSubmit({ name: "New Provider", appType })}>
          add-provider
        </button>
        <button onClick={() => onOpenChange(false)}>close-add</button>
      </div>
    ) : null,
}));

vi.mock("@/components/providers/EditProviderDialog", () => ({
  EditProviderDialog: ({ open, provider, onSubmit, onOpenChange }: any) =>
    open ? (
      <div data-testid="edit-provider-dialog">
        <span>{provider?.id}</span>
        <button
          onClick={() =>
            onSubmit({
              id: provider?.id,
              name: `${provider?.name}-edited`,
            })
          }
        >
          save-edit
        </button>
        <button onClick={() => onOpenChange(false)}>close-edit</button>
      </div>
    ) : null,
}));

vi.mock("@/components/UsageScriptModal", () => ({
  default: ({ isOpen, provider, onSave, onClose }: any) =>
    isOpen ? (
      <div data-testid="usage-modal">
        <span>{provider?.id}</span>
        <button onClick={() => onSave("script-code")}>save-script</button>
        <button onClick={() => onClose()}>close-usage</button>
      </div>
    ) : null,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({ isOpen, onConfirm, onCancel }: any) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <button onClick={() => onConfirm()}>confirm-delete</button>
        <button onClick={() => onCancel()}>cancel-delete</button>
      </div>
    ) : null,
}));

vi.mock("@/components/settings/SettingsDialog", () => ({
  SettingsDialog: ({ open, onOpenChange, onImportSuccess }: any) =>
    open ? (
      <div data-testid="settings-dialog">
        <button onClick={() => onImportSuccess?.()}>settings-on-import</button>
        <button onClick={() => onOpenChange(false)}>close-settings</button>
      </div>
    ) : (
      <button onClick={() => onOpenChange(true)}>open-settings</button>
    ),
}));

vi.mock("@/components/AppSwitcher", () => ({
  AppSwitcher: ({ activeApp, onSwitch }: any) => (
    <div data-testid="app-switcher">
      <span>{activeApp}</span>
      <button onClick={() => onSwitch("codex")}>switch-app</button>
    </div>
  ),
}));

vi.mock("@/components/UpdateBadge", () => ({
  UpdateBadge: ({ onClick }: any) => (
    <button onClick={onClick}>update-badge</button>
  ),
}));

vi.mock("@/components/mcp/McpPanel", () => ({
  default: ({ open, onOpenChange }: any) =>
    open ? (
      <div data-testid="mcp-panel">
        <button onClick={() => onOpenChange(false)}>close-mcp</button>
      </div>
    ) : (
      <button onClick={() => onOpenChange(true)}>open-mcp</button>
    ),
}));

const mockRefetch = vi.fn();

const queryClient = new QueryClient();
let AppComponent: typeof import("@/App").default;

describe("App Integration", () => {
  beforeAll(async () => {
    const module = await import("@/App");
    AppComponent = module.default;
  });

  beforeEach(() => {
    queryClient.clear();
    mockRefetch.mockReset();
    mockRefetch.mockResolvedValue(undefined);
    useProvidersQueryMock.mockReset();
    getAllMock.mockReset();
    getCurrentMock.mockReset();
    importDefaultMock.mockReset();
    onSwitchedMock.mockResolvedValue(() => {});
    updateSortOrderMock.mockResolvedValue(undefined);
    updateTrayMenuMock.mockResolvedValue(undefined);
    openExternalMock.mockResolvedValue(undefined);

    useProvidersQueryMock.mockImplementation((appType: string) => {
      const providers = providersDataMock[appType as keyof typeof providersDataMock] || [];
      getAllMock.mockResolvedValue(Object.fromEntries(providers.map((provider) => [provider.id, provider])));
      getCurrentMock.mockResolvedValue(providers[0]?.id ?? "");
      importDefaultMock.mockResolvedValue(false);
      return {
        data: {
          providers: Object.fromEntries(
            providers.map((provider) => [provider.id, provider]),
          ),
          currentProviderId: providers[0]?.id ?? "",
        },
        isLoading: false,
        refetch: mockRefetch,
      };
    });
  });

  it("should render providers, open dialogs, and execute core flows", async () => {
    const { container } = render(
      <QueryClientProvider client={queryClient}>
        <Suspense fallback={<div data-testid="loading">loading</div>}>
          <AppComponent />
        </Suspense>
      </QueryClientProvider>,
    );

    // 初始加载后，应显示 ProviderList mock 渲染的 JSON
    await waitFor(() =>
      expect(screen.getByTestId("provider-list")).toBeInTheDocument(),
    );
    expect(screen.getByText("CC Switch")).toBeInTheDocument();

    // 打开设置对话框并触发导入成功回调
    fireEvent.click(screen.getByText("update-badge")); // open settings via badge
    expect(screen.getByTestId("settings-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("settings-on-import"));
    await waitFor(() => expect(mockRefetch).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(updateTrayMenuMock).toHaveBeenCalledTimes(1));
    fireEvent.click(screen.getByText("close-settings"));
    expect(screen.queryByTestId("settings-dialog")).not.toBeInTheDocument();

    // 切换到 codex 应用，确保 useProvidersQuery 被使用
    fireEvent.click(screen.getByText("switch-app"));
    await waitFor(() => {
      expect(useProvidersQueryMock).toHaveBeenCalledWith("codex");
    });

    // 添加供应商流程
    fireEvent.click(screen.getByText("header.addProvider"));
    expect(screen.getByTestId("add-provider-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("add-provider"));
    expect(addProviderMock).toHaveBeenCalledWith({ name: "New Provider", appType: "codex" });
    fireEvent.click(screen.getByText("close-add"));

    // 编辑供应商流程
    fireEvent.click(screen.getByText("edit"));
    expect(screen.getByTestId("edit-provider-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("save-edit"));
    expect(updateProviderMock).toHaveBeenCalledWith({
      id: "codex-1",
      name: "undefined-edited",
    });
    fireEvent.click(screen.getByText("close-edit"));

    // 删除供应商流程
    fireEvent.click(screen.getByText("delete"));
    expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument();
    fireEvent.click(screen.getByText("confirm-delete"));
    expect(deleteProviderMock).toHaveBeenCalledWith("codex-1");

    // 复制供应商流程（触发排序更新 + 添加）
    fireEvent.click(screen.getByText("duplicate"));
    await waitFor(() => {
      expect(updateSortOrderMock).toHaveBeenCalled();
    });
    expect(addProviderMock).toHaveBeenCalledTimes(2);

    // 使用脚本弹窗
    fireEvent.click(screen.getByText("usage"));
    expect(screen.getByTestId("usage-modal")).toBeInTheDocument();
    fireEvent.click(screen.getByText("save-script"));
    expect(saveUsageScriptMock).toHaveBeenCalledWith(
      { id: "codex-1" },
      "script-code",
    );
    fireEvent.click(screen.getByText("close-usage"));
    expect(screen.queryByTestId("usage-modal")).not.toBeInTheDocument();

    // 打开网站链接
    fireEvent.click(screen.getByText("open-website"));
    expect(openExternalMock).toHaveBeenCalledWith("https://example.com");

    // 确保页面保留核心元素
    expect(container.textContent).toContain("CC Switch");
  });
});
