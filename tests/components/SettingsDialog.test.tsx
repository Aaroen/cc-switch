import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import { createContext, useContext } from "react";
import { SettingsDialog } from "@/components/settings/SettingsDialog";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const tMock = vi.fn((key: string) => key);
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: tMock }),
}));

interface SettingsMock {
  settings: any;
  isLoading: boolean;
  isSaving: boolean;
  isPortable: boolean;
  appConfigDir?: string;
  resolvedDirs: Record<string, string>;
  requiresRestart: boolean;
  updateSettings: ReturnType<typeof vi.fn>;
  updateDirectory: ReturnType<typeof vi.fn>;
  updateAppConfigDir: ReturnType<typeof vi.fn>;
  browseDirectory: ReturnType<typeof vi.fn>;
  browseAppConfigDir: ReturnType<typeof vi.fn>;
  resetDirectory: ReturnType<typeof vi.fn>;
  resetAppConfigDir: ReturnType<typeof vi.fn>;
  saveSettings: ReturnType<typeof vi.fn>;
  resetSettings: ReturnType<typeof vi.fn>;
  acknowledgeRestart: ReturnType<typeof vi.fn>;
}

const createSettingsMock = (overrides: Partial<SettingsMock> = {}) => {
  const base: SettingsMock = {
    settings: {
      showInTray: true,
      minimizeToTrayOnClose: true,
      enableClaudePluginIntegration: false,
      language: "zh",
      claudeConfigDir: "/claude",
      codexConfigDir: "/codex",
    },
    isLoading: false,
    isSaving: false,
    isPortable: false,
    appConfigDir: "/app-config",
    resolvedDirs: {
      claude: "/claude",
      codex: "/codex",
    },
    requiresRestart: false,
    updateSettings: vi.fn(),
    updateDirectory: vi.fn(),
    updateAppConfigDir: vi.fn(),
    browseDirectory: vi.fn(),
    browseAppConfigDir: vi.fn(),
    resetDirectory: vi.fn(),
    resetAppConfigDir: vi.fn(),
    saveSettings: vi.fn().mockResolvedValue({ requiresRestart: false }),
    resetSettings: vi.fn(),
    acknowledgeRestart: vi.fn(),
  };

  return { ...base, ...overrides };
};

interface ImportExportMock {
  selectedFile: string;
  status: string;
  errorMessage: string | null;
  backupId: string | null;
  isImporting: boolean;
  selectImportFile: ReturnType<typeof vi.fn>;
  importConfig: ReturnType<typeof vi.fn>;
  exportConfig: ReturnType<typeof vi.fn>;
  clearSelection: ReturnType<typeof vi.fn>;
  resetStatus: ReturnType<typeof vi.fn>;
}

const createImportExportMock = (overrides: Partial<ImportExportMock> = {}) => {
  const base: ImportExportMock = {
    selectedFile: "",
    status: "idle",
    errorMessage: null,
    backupId: null,
    isImporting: false,
    selectImportFile: vi.fn(),
    importConfig: vi.fn(),
    exportConfig: vi.fn(),
    clearSelection: vi.fn(),
    resetStatus: vi.fn(),
  };

  return { ...base, ...overrides };
};

let settingsMock = createSettingsMock();
let importExportMock = createImportExportMock();

vi.mock("@/hooks/useSettings", () => ({
  useSettings: () => settingsMock,
}));

vi.mock("@/hooks/useImportExport", () => ({
  useImportExport: () => importExportMock,
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    restart: vi.fn().mockResolvedValue(true),
  },
}));

const TabsContext = createContext<{ value: string; onValueChange?: (value: string) => void }>({
  value: "general",
});

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: any) => (open ? <div data-testid="dialog-root">{children}</div> : null),
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
}));

vi.mock("@/components/ui/tabs", () => {
  return {
    Tabs: ({ value, onValueChange, children }: any) => (
      <TabsContext.Provider value={{ value, onValueChange }}>
        <div data-testid="tabs">{children}</div>
      </TabsContext.Provider>
    ),
    TabsList: ({ children }: any) => <div>{children}</div>,
    TabsTrigger: ({ value, children }: any) => {
      const ctx = useContext(TabsContext);
      return (
        <button type="button" onClick={() => ctx.onValueChange?.(value)}>
          {children}
        </button>
      );
    },
    TabsContent: ({ value, children }: any) => {
      const ctx = useContext(TabsContext);
      if (ctx.value !== value) return null;
      return <div data-testid={`tab-${value}`}>{children}</div>;
    },
  };
});

vi.mock("@/components/settings/LanguageSettings", () => ({
  LanguageSettings: ({ value, onChange }: any) => (
    <div>
      <span>language:{value}</span>
      <button onClick={() => onChange("en")}>change-language</button>
    </div>
  ),
}));

vi.mock("@/components/settings/ThemeSettings", () => ({
  ThemeSettings: () => <div>theme-settings</div>,
}));

vi.mock("@/components/settings/WindowSettings", () => ({
  WindowSettings: ({ onChange }: any) => (
    <button onClick={() => onChange({ minimizeToTrayOnClose: false })}>
      window-settings
    </button>
  ),
}));

vi.mock("@/components/settings/DirectorySettings", () => ({
  DirectorySettings: ({ onBrowseDirectory }: any) => (
    <button onClick={() => onBrowseDirectory("claude")}>browse-directory</button>
  ),
}));

vi.mock("@/components/settings/AboutSection", () => ({
  AboutSection: ({ isPortable }: any) => <div>about:{String(isPortable)}</div>,
}));

let settingsApi: any;

describe("SettingsDialog Component", () => {
  beforeEach(async () => {
    tMock.mockImplementation((key: string) => key);
    settingsMock = createSettingsMock();
    importExportMock = createImportExportMock();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    settingsApi = (await import("@/lib/api")).settingsApi;
    settingsApi.restart.mockClear();
  });

  it("should not render form content when loading", () => {
    settingsMock = createSettingsMock({ settings: null, isLoading: true });

    render(<SettingsDialog open={true} onOpenChange={vi.fn()} />);

    expect(screen.queryByText("language:zh")).not.toBeInTheDocument();
    expect(screen.getByText("settings.title")).toBeInTheDocument();
  });

  it("should render general and advanced tabs and trigger child callbacks", () => {
    const onOpenChange = vi.fn();
    importExportMock = createImportExportMock({ selectedFile: "" });

    render(<SettingsDialog open={true} onOpenChange={onOpenChange} />);

    expect(screen.getByText("language:zh")).toBeInTheDocument();
    expect(screen.getByText("theme-settings")).toBeInTheDocument();

    fireEvent.click(screen.getByText("change-language"));
    expect(settingsMock.updateSettings).toHaveBeenCalledWith({ language: "en" });

    fireEvent.click(screen.getByText("window-settings"));
    expect(settingsMock.updateSettings).toHaveBeenCalledWith({ minimizeToTrayOnClose: false });

    fireEvent.click(screen.getByText("settings.tabAdvanced"));
    fireEvent.click(screen.getByRole("button", { name: "settings.selectConfigFile" }));

    expect(importExportMock.selectImportFile).toHaveBeenCalled();
  });

  it("should call saveSettings and close dialog when clicking save", async () => {
    const onOpenChange = vi.fn();
    importExportMock = createImportExportMock();

    render(<SettingsDialog open={true} onOpenChange={onOpenChange} />);

    fireEvent.click(screen.getByText("common.save"));

    await waitFor(() => {
      expect(settingsMock.saveSettings).toHaveBeenCalledTimes(1);
      expect(importExportMock.clearSelection).toHaveBeenCalledTimes(1);
      expect(importExportMock.resetStatus).toHaveBeenCalledTimes(2);
      expect(settingsMock.acknowledgeRestart).toHaveBeenCalledTimes(1);
      expect(onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  it("should reset settings and close dialog when clicking cancel", () => {
    const onOpenChange = vi.fn();

    render(<SettingsDialog open={true} onOpenChange={onOpenChange} />);

    fireEvent.click(screen.getByText("common.cancel"));

    expect(settingsMock.resetSettings).toHaveBeenCalledTimes(1);
    expect(settingsMock.acknowledgeRestart).toHaveBeenCalledTimes(1);
    expect(importExportMock.clearSelection).toHaveBeenCalledTimes(1);
    expect(importExportMock.resetStatus).toHaveBeenCalledTimes(2);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("should show restart prompt and allow immediate restart after save", async () => {
    settingsMock = createSettingsMock({
      requiresRestart: true,
      saveSettings: vi.fn().mockResolvedValue({ requiresRestart: true }),
    });

    render(<SettingsDialog open={true} onOpenChange={vi.fn()} />);

    expect(await screen.findByText("settings.restartRequired")).toBeInTheDocument();

    fireEvent.click(screen.getByText("settings.restartNow"));

    await waitFor(() => {
      expect(toastSuccessMock).toHaveBeenCalledWith("settings.devModeRestartHint");
    });
  });
});
