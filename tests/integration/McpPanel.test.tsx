import React from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import McpPanel from "@/components/mcp/McpPanel";
import type { McpServer } from "@/types";
import { createTestQueryClient } from "../utils/testQueryClient";

const toastSuccessMock = vi.hoisted(() => vi.fn());
const toastErrorMock = vi.hoisted(() => vi.fn());

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

const importFromClaudeMock = vi.hoisted(() => vi.fn().mockResolvedValue(1));
const importFromCodexMock = vi.hoisted(() => vi.fn().mockResolvedValue(1));

const toggleEnabledMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const saveServerMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const deleteServerMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));
const reloadMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined));

const baseServers: Record<string, McpServer> = {
  sample: {
    id: "sample",
    name: "Sample Claude Server",
    enabled: true,
    apps: { claude: true, codex: false, gemini: false },
    server: {
      type: "stdio",
      command: "claude-server",
    },
  },
};

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    mcpApi: {
      ...actual.mcpApi,
      importFromClaude: (...args: unknown[]) =>
        importFromClaudeMock(...args),
      importFromCodex: (...args: unknown[]) => importFromCodexMock(...args),
    },
  };
});

vi.mock("@/components/mcp/McpListItem", () => ({
  default: ({ id, server, onToggle, onEdit, onDelete }: any) => (
    <div data-testid={`mcp-item-${id}`}>
      <span>{server.name || id}</span>
      <button
        type="button"
        onClick={() => onToggle(id, !server.enabled)}
        data-testid={`toggle-${id}`}
      >
        toggle
      </button>
      <button type="button" onClick={() => onEdit(id)} data-testid={`edit-${id}`}>
        edit
      </button>
      <button
        type="button"
        onClick={() => onDelete(id)}
        data-testid={`delete-${id}`}
      >
        delete
      </button>
    </div>
  ),
}));

vi.mock("@/components/mcp/McpFormModal", () => ({
  default: ({ onSave, onClose }: any) => (
    <div data-testid="mcp-form">
      <button
        type="button"
        onClick={() =>
          onSave(
            "new-server",
            {
              id: "new-server",
              name: "New Server",
              enabled: true,
              server: { type: "stdio", command: "new.cmd" },
            },
            { syncOtherSide: true },
          )
        }
      >
        submit-form
      </button>
      <button type="button" onClick={onClose}>
        close-form
      </button>
    </div>
  ),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, ...rest }: any) => (
    <button type="button" onClick={onClick} {...rest}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ open, children }: any) => (open ? <div>{children}</div> : null),
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/ConfirmDialog", () => ({
  ConfirmDialog: ({ isOpen, onConfirm }: any) =>
    isOpen ? (
      <div data-testid="confirm-dialog">
        <button type="button" onClick={onConfirm}>
          confirm-delete
        </button>
      </div>
    ) : null,
}));

const renderPanel = (props?: Partial<React.ComponentProps<typeof McpPanel>>) => {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <McpPanel open onOpenChange={() => {}} appId="claude" {...props} />
    </QueryClientProvider>,
  );
};

const useMcpActionsMock = vi.hoisted(() => vi.fn());

vi.mock("@/hooks/useMcpActions", () => ({
  useMcpActions: (...args: unknown[]) => useMcpActionsMock(...args),
}));

describe("McpPanel integration", () => {
  beforeEach(() => {
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    importFromClaudeMock.mockClear();
    importFromClaudeMock.mockResolvedValue(1);
    importFromCodexMock.mockClear();
    importFromCodexMock.mockResolvedValue(1);

    toggleEnabledMock.mockClear();
    saveServerMock.mockClear();
    deleteServerMock.mockClear();
    reloadMock.mockClear();

    useMcpActionsMock.mockReturnValue({
      servers: baseServers,
      loading: false,
      reload: reloadMock,
      toggleEnabled: toggleEnabledMock,
      saveServer: saveServerMock,
      deleteServer: deleteServerMock,
    });
  });

  it("加载并切换 MCP 启用状态", async () => {
    renderPanel();

    await waitFor(() =>
      expect(screen.getByTestId("mcp-item-sample")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("toggle-sample"));

    await waitFor(() =>
      expect(toggleEnabledMock).toHaveBeenCalledWith("sample", false),
    );
  });

  it("新增 MCP 并触发保存与同步选项", async () => {
    renderPanel();
    await waitFor(() =>
      expect(
        screen.getByText((content) => content.startsWith("mcp.serverCount")),
      ).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByText("mcp.add"));
    await waitFor(() => expect(screen.getByTestId("mcp-form")).toBeInTheDocument());

    fireEvent.click(screen.getByText("submit-form"));

    await waitFor(() =>
      expect(screen.queryByTestId("mcp-form")).not.toBeInTheDocument(),
    );
    await waitFor(() =>
      expect(saveServerMock).toHaveBeenCalledWith(
        "new-server",
        expect.objectContaining({ id: "new-server" }),
        { syncOtherSide: true },
      ),
    );
  });

  it("删除 MCP 并发送确认请求", async () => {
    renderPanel();
    await waitFor(() =>
      expect(screen.getByTestId("mcp-item-sample")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("delete-sample"));
    await waitFor(() =>
      expect(screen.getByTestId("confirm-dialog")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByText("confirm-delete"));

    await waitFor(() => expect(deleteServerMock).toHaveBeenCalledWith("sample"));
  });
});
