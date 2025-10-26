import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import McpFormModal from "@/components/mcp/McpFormModal";

const toastErrorMock = vi.hoisted(() => vi.fn());
const toastSuccessMock = vi.hoisted(() => vi.fn());
const getConfigMock = vi.hoisted(() => vi.fn().mockResolvedValue({ servers: {} }));

vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
    success: (...args: unknown[]) => toastSuccessMock(...args),
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock("@/config/mcpPresets", () => ({
  mcpPresets: [
    {
      id: "preset-stdio",
      server: { type: "stdio", command: "preset-cmd" },
    },
  ],
  getMcpPresetWithDescription: (preset: any) => ({
    ...preset,
    description: "Preset description",
    tags: ["preset"],
  }),
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, type = "button", ...rest }: any) => (
    <button type={type} onClick={onClick} {...rest}>
      {children}
    </button>
  ),
}));

vi.mock("@/components/ui/input", () => ({
  Input: ({ value, onChange, ...rest }: any) => (
    <input
      value={value}
      onChange={(event) => onChange?.({ target: { value: event.target.value } })}
      {...rest}
    />
  ),
}));

vi.mock("@/components/ui/textarea", () => ({
  Textarea: ({ value, onChange, ...rest }: any) => (
    <textarea
      value={value}
      onChange={(event) => onChange?.({ target: { value: event.target.value } })}
      {...rest}
    />
  ),
}));

vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children }: any) => <div>{children}</div>,
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <div>{children}</div>,
  DialogFooter: ({ children }: any) => <div>{children}</div>,
}));

vi.mock("@/components/mcp/McpWizardModal", () => ({
  default: () => null,
}));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    mcpApi: {
      ...actual.mcpApi,
      getConfig: (...args: unknown[]) => getConfigMock(...args),
    },
  };
});

describe("McpFormModal", () => {
  beforeEach(() => {
    toastErrorMock.mockReset();
    toastSuccessMock.mockReset();
    getConfigMock.mockReset();
    getConfigMock.mockResolvedValue({ servers: {} });
  });

  const renderForm = (props?: Partial<React.ComponentProps<typeof McpFormModal>>) => {
    const onSave = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(
      <McpFormModal
        appType="claude"
        onSave={onSave}
        onClose={onClose}
        existingIds={[]}
        {...props}
      />,
    );
    return { onSave, onClose };
  };

  it("应用预设后填充 ID 与配置内容", async () => {
    renderForm();
    await waitFor(() =>
      expect(screen.getByPlaceholderText("mcp.form.titlePlaceholder")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByText("preset-stdio"));

    const idInput = screen.getByPlaceholderText(
      "mcp.form.titlePlaceholder",
    ) as HTMLInputElement;
    expect(idInput.value).toBe("preset-stdio");

    const configTextarea = screen.getByPlaceholderText(
      "mcp.form.jsonPlaceholder",
    ) as HTMLTextAreaElement;
    expect(configTextarea.value).toBe('{\n  "type": "stdio",\n  "command": "preset-cmd"\n}');
  });

  it("在同步另一侧存在冲突时展示警告", async () => {
    getConfigMock.mockResolvedValue({ servers: { conflict: {} } });
    renderForm();

    const idInput = screen.getByPlaceholderText(
      "mcp.form.titlePlaceholder",
    ) as HTMLInputElement;
    fireEvent.change(idInput, { target: { value: "conflict" } });

    await waitFor(() => expect(getConfigMock).toHaveBeenCalled());

    const checkbox = screen.getByLabelText(
      'mcp.form.syncOtherSide:{"target":"apps.codex"}',
    ) as HTMLInputElement;
    fireEvent.click(checkbox);

    await waitFor(() =>
      expect(
        screen.getByText('mcp.form.willOverwriteWarning:{"target":"apps.codex"}'),
      ).toBeInTheDocument(),
    );
  });

  it("提交时清洗字段并调用 onSave", async () => {
    const { onSave } = renderForm();

    fireEvent.change(screen.getByPlaceholderText("mcp.form.titlePlaceholder"), {
      target: { value: " my-server " },
    });
    fireEvent.change(screen.getByPlaceholderText("mcp.form.namePlaceholder"), {
      target: { value: "   Friendly " },
    });

    fireEvent.click(screen.getByText("mcp.form.additionalInfo"));

    fireEvent.change(screen.getByPlaceholderText("mcp.form.descriptionPlaceholder"), {
      target: { value: " Description " },
    });
    fireEvent.change(screen.getByPlaceholderText("mcp.form.tagsPlaceholder"), {
      target: { value: " tag1 , tag2 " },
    });
    fireEvent.change(screen.getByPlaceholderText("mcp.form.homepagePlaceholder"), {
      target: { value: " https://example.com " },
    });
    fireEvent.change(screen.getByPlaceholderText("mcp.form.docsPlaceholder"), {
      target: { value: " https://docs.example.com " },
    });

    fireEvent.change(screen.getByPlaceholderText("mcp.form.jsonPlaceholder"), {
      target: { value: '{"type":"stdio","command":"run"}' },
    });

    const syncCheckbox = screen.getByLabelText(
      'mcp.form.syncOtherSide:{"target":"apps.codex"}',
    ) as HTMLInputElement;
    fireEvent.click(syncCheckbox);

    fireEvent.click(screen.getByText("common.add"));

    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    const [id, payload, options] = onSave.mock.calls[0];
    expect(id).toBe("my-server");
    expect(payload).toMatchObject({
      id: "my-server",
      name: "Friendly",
      description: "Description",
      homepage: "https://example.com",
      docs: "https://docs.example.com",
      tags: ["tag1", "tag2"],
      server: {
        type: "stdio",
        command: "run",
      },
    });
    expect(options).toEqual({ syncOtherSide: true });
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("缺少配置命令时阻止提交并提示错误", async () => {
    const { onSave } = renderForm();

    fireEvent.change(screen.getByPlaceholderText("mcp.form.titlePlaceholder"), {
      target: { value: "no-command" },
    });
    fireEvent.change(screen.getByPlaceholderText("mcp.form.jsonPlaceholder"), {
      target: { value: '{"type":"stdio"}' },
    });

    fireEvent.click(screen.getByText("common.add"));

    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    expect(onSave).not.toHaveBeenCalled();
    const [message] = toastErrorMock.mock.calls.at(-1) ?? [];
    expect(message).toBe("mcp.error.jsonInvalid");
  });
});

