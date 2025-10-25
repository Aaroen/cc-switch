import { renderHook, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useImportExport } from "@/hooks/useImportExport";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const openFileDialogMock = vi.fn();
const importConfigMock = vi.fn();

vi.mock("@/lib/api", () => ({
  settingsApi: {
    openFileDialog: (...args: unknown[]) => openFileDialogMock(...args),
    importConfigFromFile: (...args: unknown[]) => importConfigMock(...args),
    saveFileDialog: vi.fn(),
    exportConfigToFile: vi.fn(),
  },
}));

describe("useImportExport Hook (edge cases)", () => {
  beforeEach(() => {
    openFileDialogMock.mockReset();
    importConfigMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps state unchanged when file dialog resolves to null", async () => {
    openFileDialogMock.mockResolvedValue(null);
    const { result } = renderHook(() => useImportExport());

    await act(async () => {
      await result.current.selectImportFile();
    });

    expect(result.current.selectedFile).toBe("");
    expect(result.current.status).toBe("idle");
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("resetStatus clears errors but preserves selected file", async () => {
    openFileDialogMock.mockResolvedValue("/config.json");
    importConfigMock.mockResolvedValue({ success: false, message: "broken" });
    const { result } = renderHook(() => useImportExport());

    await act(async () => {
      await result.current.selectImportFile();
    });

    await act(async () => {
      await result.current.importConfig();
    });

    act(() => {
      result.current.resetStatus();
    });

    expect(result.current.selectedFile).toBe("/config.json");
    expect(result.current.status).toBe("idle");
    expect(result.current.errorMessage).toBeNull();
    expect(result.current.backupId).toBeNull();
  });

});
