import { renderHook, act, waitFor } from "@testing-library/react";
import { describe, it, expect, beforeEach, vi } from "vitest";
import { useMcpActions } from "@/hooks/useMcpActions";
import type { McpServer } from "@/types";

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const getConfigMock = vi.fn();
const setEnabledMock = vi.fn();
const upsertServerInConfigMock = vi.fn();
const deleteServerInConfigMock = vi.fn();

vi.mock("@/lib/api", () => ({
  mcpApi: {
    getConfig: (...args: unknown[]) => getConfigMock(...args),
    setEnabled: (...args: unknown[]) => setEnabledMock(...args),
    upsertServerInConfig: (...args: unknown[]) => upsertServerInConfigMock(...args),
    deleteServerInConfig: (...args: unknown[]) => deleteServerInConfigMock(...args),
  },
}));

const createServer = (overrides: Partial<McpServer> = {}): McpServer => ({
  id: "server-1",
  name: "Test Server",
  description: "desc",
  enabled: false,
  server: {
    type: "stdio",
    command: "run.sh",
    args: [],
    env: {},
  },
  ...overrides,
});

const mockConfigResponse = (servers: Record<string, McpServer>) => ({
  configPath: "/mock/config.json",
  servers,
});

const createDeferred = <T,>() => {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
};

describe("useMcpActions", () => {
  beforeEach(() => {
    getConfigMock.mockReset();
    setEnabledMock.mockReset();
    upsertServerInConfigMock.mockReset();
    deleteServerInConfigMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();

    getConfigMock.mockResolvedValue(mockConfigResponse({}));
    setEnabledMock.mockResolvedValue(true);
    upsertServerInConfigMock.mockResolvedValue(true);
    deleteServerInConfigMock.mockResolvedValue(true);
  });

  const renderUseMcpActions = () => renderHook(() => useMcpActions("claude"));

  it("reloads servers and toggles loading state", async () => {
    const server = createServer();
    const deferred = createDeferred<ReturnType<typeof mockConfigResponse>>();
    getConfigMock.mockReturnValueOnce(deferred.promise);
    const { result } = renderUseMcpActions();

    let reloadPromise: Promise<void> | undefined;
    await act(async () => {
      reloadPromise = result.current.reload();
    });
    await waitFor(() => expect(result.current.loading).toBe(true));
    deferred.resolve(mockConfigResponse({ [server.id]: server }));
    await act(async () => {
      await reloadPromise;
    });

    expect(getConfigMock).toHaveBeenCalledWith("claude");
    expect(result.current.loading).toBe(false);
    expect(result.current.servers).toEqual({ [server.id]: server });
  });

  it("shows toast error when reload fails", async () => {
    const error = new Error("load failed");
    getConfigMock.mockRejectedValueOnce(error);
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.reload();
    });

    expect(toastErrorMock).toHaveBeenCalledWith("load failed", { duration: 6000 });
    expect(result.current.servers).toEqual({});
    expect(result.current.loading).toBe(false);
  });

  it("toggles enabled flag optimistically and emits success toasts", async () => {
    const server = createServer({ enabled: false });
    getConfigMock.mockResolvedValueOnce(mockConfigResponse({ [server.id]: server }));
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.reload();
    });

    await act(async () => {
      await result.current.toggleEnabled(server.id, true);
    });

    expect(setEnabledMock).toHaveBeenCalledWith("claude", server.id, true);
    expect(result.current.servers[server.id].enabled).toBe(true);
    expect(toastSuccessMock).toHaveBeenLastCalledWith("mcp.msg.enabled", { duration: 1500 });

    await act(async () => {
      await result.current.toggleEnabled(server.id, false);
    });

    expect(setEnabledMock).toHaveBeenLastCalledWith("claude", server.id, false);
    expect(result.current.servers[server.id].enabled).toBe(false);
    expect(toastSuccessMock).toHaveBeenLastCalledWith("mcp.msg.disabled", { duration: 1500 });
  });

  it("rolls back state and shows error toast when toggle fails", async () => {
    const server = createServer({ enabled: false });
    getConfigMock.mockResolvedValueOnce(mockConfigResponse({ [server.id]: server }));
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.reload();
    });

    setEnabledMock.mockRejectedValueOnce(new Error("toggle failed"));

    await act(async () => {
      await result.current.toggleEnabled(server.id, true);
    });

    expect(result.current.servers[server.id].enabled).toBe(false);
    expect(toastErrorMock).toHaveBeenCalledWith("toggle failed", { duration: 6000 });
  });

  it("saves server configuration and refreshes list", async () => {
    const serverInput = createServer({ id: "old-id", enabled: true });
    const savedServer = { ...serverInput, id: "new-server" };
    getConfigMock.mockResolvedValueOnce(mockConfigResponse({ [savedServer.id]: savedServer }));
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.saveServer("new-server", serverInput, { syncOtherSide: true });
    });

    expect(upsertServerInConfigMock).toHaveBeenCalledWith(
      "claude",
      "new-server",
      { ...serverInput, id: "new-server" },
      { syncOtherSide: true },
    );
    expect(result.current.servers["new-server"]).toEqual(savedServer);
    expect(toastSuccessMock).toHaveBeenCalledWith("mcp.msg.saved", { duration: 1500 });
  });

  it("propagates error when saveServer fails", async () => {
    const serverInput = createServer({ id: "input-id" });
    const failure = new Error("cannot save");
    upsertServerInConfigMock.mockRejectedValueOnce(failure);
    const { result } = renderUseMcpActions();

    let captured: unknown;
    await act(async () => {
      try {
        await result.current.saveServer("server-1", serverInput);
      } catch (err) {
        captured = err;
      }
    });

    expect(upsertServerInConfigMock).toHaveBeenCalled();
    expect(getConfigMock).not.toHaveBeenCalled();
    expect(captured).toBe(failure);
    expect(toastErrorMock).toHaveBeenCalledWith("cannot save", { duration: 6000 });
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("deletes server and refreshes list", async () => {
    const server = createServer();
    getConfigMock.mockResolvedValueOnce(mockConfigResponse({ [server.id]: server }));
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.reload();
    });

    getConfigMock.mockResolvedValueOnce(mockConfigResponse({}));

    await act(async () => {
      await result.current.deleteServer(server.id);
    });

    expect(deleteServerInConfigMock).toHaveBeenCalledWith("claude", server.id);
    expect(result.current.servers[server.id]).toBeUndefined();
    expect(toastSuccessMock).toHaveBeenCalledWith("mcp.msg.deleted", { duration: 1500 });
  });

  it("propagates delete error and keeps state", async () => {
    const server = createServer();
    getConfigMock.mockResolvedValueOnce(mockConfigResponse({ [server.id]: server }));
    const { result } = renderUseMcpActions();

    await act(async () => {
      await result.current.reload();
    });

    const failure = new Error("delete failed");
    deleteServerInConfigMock.mockRejectedValueOnce(failure);

    let captured: unknown;
    await act(async () => {
      try {
        await result.current.deleteServer(server.id);
      } catch (err) {
        captured = err;
      }
    });

    expect(deleteServerInConfigMock).toHaveBeenCalledWith("claude", server.id);
    expect(result.current.servers[server.id]).toEqual(server);
    expect(captured).toBe(failure);
    expect(toastErrorMock).toHaveBeenCalledWith("delete failed", { duration: 6000 });
  });
});
