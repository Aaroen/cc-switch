import { useMemo } from "react";
import {
  AlertCircle,
  CheckCircle2,
  FolderOpen,
  Loader2,
  Save,
  XCircle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import type { ImportStatus } from "@/hooks/useImportExport";

interface ImportExportSectionProps {
  status: ImportStatus;
  selectedFile: string;
  errorMessage: string | null;
  backupId: string | null;
  isImporting: boolean;
  onSelectFile: () => Promise<void>;
  onImport: () => Promise<void>;
  onExport: () => Promise<void>;
  onClear: () => void;
}

export function ImportExportSection({
  status,
  selectedFile,
  errorMessage,
  backupId,
  isImporting,
  onSelectFile,
  onImport,
  onExport,
  onClear,
}: ImportExportSectionProps) {
  const { t } = useTranslation();

  const selectedFileName = useMemo(() => {
    if (!selectedFile) return "";
    const segments = selectedFile.split(/[\\/]/);
    return segments[segments.length - 1] || selectedFile;
  }, [selectedFile]);

  return (
    <section className="space-y-4">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">{t("settings.importExport")}</h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.importExportHint")}
        </p>
      </header>

      <div className="space-y-3 rounded-lg border border-border-default p-4">
        <Button
          type="button"
          className="w-full"
          variant="secondary"
          onClick={onExport}
        >
          <Save className="mr-2 h-4 w-4" />
          {t("settings.exportConfig")}
        </Button>

        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              variant="outline"
              className="flex-1 min-w-[180px]"
              onClick={onSelectFile}
            >
              <FolderOpen className="mr-2 h-4 w-4" />
              {t("settings.selectConfigFile")}
            </Button>
            <Button
              type="button"
              disabled={!selectedFile || isImporting}
              onClick={onImport}
            >
              {isImporting ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {t("settings.importing")}
                </span>
              ) : (
                t("settings.import")
              )}
            </Button>
            {selectedFile ? (
              <Button type="button" variant="ghost" onClick={onClear}>
                <XCircle className="mr-2 h-4 w-4" />
                {t("common.clear")}
              </Button>
            ) : null}
          </div>

          {selectedFile ? (
            <p className="truncate rounded-md bg-muted/40 px-3 py-2 text-xs font-mono text-muted-foreground">
              {selectedFileName}
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">
              {t("settings.noFileSelected")}
            </p>
          )}
        </div>

        <ImportStatusMessage
          status={status}
          errorMessage={errorMessage}
          backupId={backupId}
        />
      </div>
    </section>
  );
}

interface ImportStatusMessageProps {
  status: ImportStatus;
  errorMessage: string | null;
  backupId: string | null;
}

function ImportStatusMessage({
  status,
  errorMessage,
  backupId,
}: ImportStatusMessageProps) {
  const { t } = useTranslation();

  if (status === "idle") {
    return null;
  }

  const baseClass =
    "flex items-start gap-2 rounded-md border px-3 py-2 text-xs leading-relaxed";

  if (status === "importing") {
    return (
      <div className={`${baseClass} border-border-default bg-muted/40`}>
        <Loader2 className="mt-0.5 h-4 w-4 animate-spin text-muted-foreground" />
        <div>
          <p className="font-medium">{t("settings.importing")}</p>
          <p className="text-muted-foreground">{t("common.loading")}</p>
        </div>
      </div>
    );
  }

  if (status === "success") {
    return (
      <div
        className={`${baseClass} border-green-200 bg-green-100/70 text-green-700`}
      >
        <CheckCircle2 className="mt-0.5 h-4 w-4" />
        <div className="space-y-1">
          <p className="font-medium">{t("settings.importSuccess")}</p>
          {backupId ? (
            <p className="text-xs">
              {t("settings.backupId")}: {backupId}
            </p>
          ) : null}
          <p>{t("settings.autoReload")}</p>
        </div>
      </div>
    );
  }

  const message = errorMessage || t("settings.importFailed");

  return (
    <div className={`${baseClass} border-red-200 bg-red-100/70 text-red-600`}>
      <AlertCircle className="mt-0.5 h-4 w-4" />
      <div className="space-y-1">
        <p className="font-medium">{t("settings.importFailed")}</p>
        <p>{message}</p>
      </div>
    </div>
  );
}
