import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Server, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { type AppId } from "@/lib/api";
import { McpServer } from "@/types";
import { useMcpActions } from "@/hooks/useMcpActions";
import McpListItem from "./McpListItem";
import McpFormModal from "./McpFormModal";
import { ConfirmDialog } from "../ConfirmDialog";

interface McpPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appId: AppId;
}

/**
 * MCP 管理面板
 * 采用与主界面一致的设计风格，右上角添加按钮，每个 MCP 占一行
 */
const McpPanel: React.FC<McpPanelProps> = ({ open, onOpenChange, appId }) => {
  const { t } = useTranslation();
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  } | null>(null);

  // Use MCP actions hook
  const { servers, loading, reload, toggleEnabled, saveServer, deleteServer } = useMcpActions(appId);

  useEffect(() => {
    const setup = async () => {
      try {
        // Initialize: only import existing MCPs from corresponding client
        if (appId === "claude") {
          const mcpApi = await import("@/lib/api").then((m) => m.mcpApi);
          await mcpApi.importFromClaude();
        } else if (appId === "codex") {
          const mcpApi = await import("@/lib/api").then((m) => m.mcpApi);
          await mcpApi.importFromCodex();
        }
      } catch (e) {
        console.warn("MCP initialization import failed (ignored)", e);
      } finally {
        await reload();
      }
    };
    setup();
    // Re-initialize when appId changes
  }, [appId, reload]);

  const handleEdit = (id: string) => {
    setEditingId(id);
    setIsFormOpen(true);
  };

  const handleAdd = () => {
    setEditingId(null);
    setIsFormOpen(true);
  };

  const handleDelete = (id: string) => {
    setConfirmDialog({
      isOpen: true,
      title: t("mcp.confirm.deleteTitle"),
      message: t("mcp.confirm.deleteMessage", { id }),
      onConfirm: async () => {
        try {
          await deleteServer(id);
          setConfirmDialog(null);
        } catch (e) {
          // Error already handled by useMcpActions
        }
      },
    });
  };

  const handleSave = async (
    id: string,
    server: McpServer,
    options?: { syncOtherSide?: boolean },
  ) => {
    await saveServer(id, server, options);
    setIsFormOpen(false);
    setEditingId(null);
  };

  const handleCloseForm = () => {
    setIsFormOpen(false);
    setEditingId(null);
  };

  const serverEntries = useMemo(
    () => Object.entries(servers) as Array<[string, McpServer]>,
    [servers],
  );

  const enabledCount = useMemo(
    () => serverEntries.filter(([_, server]) => server.enabled).length,
    [serverEntries],
  );

  const panelTitle = appId === "claude" ? t("mcp.claudeTitle") : t("mcp.codexTitle");

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-3xl max-h-[85vh] min-h-[600px] flex flex-col">
          <DialogHeader>
            <div className="flex items-center justify-between pr-8">
              <DialogTitle>{panelTitle}</DialogTitle>
              <Button type="button" variant="mcp" onClick={handleAdd}>
                <Plus size={16} />
                {t("mcp.add")}
              </Button>
            </div>
          </DialogHeader>

          {/* Info Section */}
          <div className="flex-shrink-0 px-6 py-4">
            <div className="text-sm text-gray-500 dark:text-gray-400">
              {t("mcp.serverCount", { count: Object.keys(servers).length })} ·{" "}
              {t("mcp.enabledCount", { count: enabledCount })}
            </div>
          </div>

          {/* Content - Scrollable */}
          <div className="flex-1 overflow-y-auto px-6 pb-4">
            {loading ? (
              <div className="text-center py-12 text-gray-500 dark:text-gray-400">
                {t("mcp.loading")}
              </div>
            ) : (
              (() => {
                const hasAny = serverEntries.length > 0;
                if (!hasAny) {
                  return (
                    <div className="text-center py-12">
                      <div className="w-16 h-16 mx-auto mb-4 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center">
                        <Server
                          size={24}
                          className="text-gray-400 dark:text-gray-500"
                        />
                      </div>
                      <h3 className="text-lg font-medium text-gray-900 dark:text-gray-100 mb-2">
                        {t("mcp.empty")}
                      </h3>
                      <p className="text-gray-500 dark:text-gray-400 text-sm">
                        {t("mcp.emptyDescription")}
                      </p>
                    </div>
                  );
                }

                return (
                  <div className="space-y-3">
                    {/* 已安装 */}
                    {serverEntries.map(([id, server]) => (
                      <McpListItem
                        key={`installed-${id}`}
                        id={id}
                        server={server}
                        onToggle={toggleEnabled}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                      />
                    ))}

                    {/* 预设已移至"新增 MCP"面板中展示与套用 */}
                  </div>
                );
              })()
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="mcp"
              onClick={() => onOpenChange(false)}
            >
              <Check size={16} />
              {t("common.done")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Form Modal */}
      {isFormOpen && (
        <McpFormModal
          appId={appId}
          editingId={editingId || undefined}
          initialData={editingId ? servers[editingId] : undefined}
          existingIds={Object.keys(servers)}
          onSave={handleSave}
          onClose={handleCloseForm}
        />
      )}

      {/* Confirm Dialog */}
      {confirmDialog && (
        <ConfirmDialog
          isOpen={confirmDialog.isOpen}
          title={confirmDialog.title}
          message={confirmDialog.message}
          onConfirm={confirmDialog.onConfirm}
          onCancel={() => setConfirmDialog(null)}
        />
      )}
    </>
  );
};

export default McpPanel;
