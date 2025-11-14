import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Download, FileJson, FileCode, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { mcpApi } from "@/lib/api";
import type { AppId } from "@/lib/api/types";

interface McpImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportComplete?: () => void;
}

/**
 * MCP 导入对话框
 * 支持从 Claude/Codex/Gemini 的 live 配置导入 MCP 服务器
 */
const McpImportDialog: React.FC<McpImportDialogProps> = ({
  open,
  onOpenChange,
  onImportComplete,
}) => {
  const { t } = useTranslation();
  const [importing, setImporting] = useState(false);
  const [selectedSource, setSelectedSource] = useState<AppId | null>(null);

  const handleImport = async (source: AppId) => {
    setImporting(true);
    setSelectedSource(source);

    try {
      let count = 0;

      switch (source) {
        case "claude":
          count = await mcpApi.importFromClaude();
          break;
        case "codex":
          count = await mcpApi.importFromCodex();
          break;
        case "gemini":
          count = await mcpApi.importFromGemini();
          break;
      }

      if (count > 0) {
        toast.success(t("mcp.unifiedPanel.import.success", { count }));
        onImportComplete?.();
        onOpenChange(false);
      } else {
        toast.info(t("mcp.unifiedPanel.import.noServersFound"));
      }
    } catch (error) {
      toast.error(t("common.error"), {
        description: String(error),
      });
    } finally {
      setImporting(false);
      setSelectedSource(null);
    }
  };

  const importSources = [
    {
      id: "claude" as AppId,
      name: t("mcp.unifiedPanel.apps.claude"),
      description: t("mcp.unifiedPanel.import.fromClaudeDesc"),
      icon: FileJson,
      color: "text-blue-600 dark:text-blue-400",
      bgColor: "bg-blue-100 dark:bg-blue-900/30",
    },
    {
      id: "codex" as AppId,
      name: t("mcp.unifiedPanel.apps.codex"),
      description: t("mcp.unifiedPanel.import.fromCodexDesc"),
      icon: FileCode,
      color: "text-green-600 dark:text-green-400",
      bgColor: "bg-green-100 dark:bg-green-900/30",
    },
    {
      id: "gemini" as AppId,
      name: t("mcp.unifiedPanel.apps.gemini"),
      description: t("mcp.unifiedPanel.import.fromGeminiDesc"),
      icon: Sparkles,
      color: "text-purple-600 dark:text-purple-400",
      bgColor: "bg-purple-100 dark:bg-purple-900/30",
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("mcp.unifiedPanel.import.title")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <p className="text-sm text-gray-600 dark:text-gray-400">
            {t("mcp.unifiedPanel.import.description")}
          </p>

          <div className="space-y-3">
            {importSources.map((source) => {
              const Icon = source.icon;
              const isImporting = importing && selectedSource === source.id;

              return (
                <button
                  key={source.id}
                  type="button"
                  onClick={() => handleImport(source.id)}
                  disabled={importing}
                  className={`w-full p-4 rounded-lg border-2 transition-all duration-200 text-left ${
                    importing && selectedSource !== source.id
                      ? "opacity-50 cursor-not-allowed"
                      : "hover:border-gray-400 dark:hover:border-gray-500 cursor-pointer"
                  } ${
                    isImporting
                      ? "border-blue-500 dark:border-blue-400"
                      : "border-gray-200 dark:border-gray-700"
                  }`}
                >
                  <div className="flex items-start gap-4">
                    <div
                      className={`flex-shrink-0 w-12 h-12 rounded-lg ${source.bgColor} flex items-center justify-center`}
                    >
                      <Icon className={`w-6 h-6 ${source.color}`} />
                    </div>

                    <div className="flex-1 min-w-0">
                      <h3 className="font-medium text-gray-900 dark:text-gray-100 mb-1">
                        {source.name}
                      </h3>
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {source.description}
                      </p>
                    </div>

                    {isImporting ? (
                      <div className="flex-shrink-0">
                        <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500" />
                      </div>
                    ) : (
                      <div className="flex-shrink-0">
                        <Download className="w-5 h-5 text-gray-400" />
                      </div>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
            disabled={importing}
          >
            {t("common.cancel")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default McpImportDialog;
