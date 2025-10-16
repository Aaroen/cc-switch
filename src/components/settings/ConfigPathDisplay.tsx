import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";

interface ConfigPathDisplayProps {
  path: string;
  onOpen: () => Promise<void> | void;
}

export function ConfigPathDisplay({ path, onOpen }: ConfigPathDisplayProps) {
  const { t } = useTranslation();

  return (
    <section className="space-y-2">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">
          {t("settings.configFileLocation")}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.configFileLocationHint", {
            defaultValue: "显示当前使用的配置文件路径。",
          })}
        </p>
      </header>
      <div className="flex items-center gap-2">
        <span className="flex-1 truncate rounded-md border border-dashed border-border bg-muted/40 px-3 py-2 text-xs font-mono">
          {path || t("common.loading")}
        </span>
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={onOpen}
          title={t("settings.openFolder")}
        >
          <FolderOpen className="h-4 w-4" />
        </Button>
      </div>
    </section>
  );
}
