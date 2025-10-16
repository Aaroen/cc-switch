import { BarChart3, Check, Play, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ProviderActionsProps {
  isCurrent: boolean;
  onSwitch: () => void;
  onEdit: () => void;
  onConfigureUsage: () => void;
  onDelete: () => void;
}

export function ProviderActions({
  isCurrent,
  onSwitch,
  onEdit,
  onConfigureUsage,
  onDelete,
}: ProviderActionsProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center gap-2">
      <Button
        size="sm"
        variant={isCurrent ? "secondary" : "default"}
        onClick={onSwitch}
        disabled={isCurrent}
        className={cn(
          "w-[96px]",
          isCurrent && "text-muted-foreground hover:text-muted-foreground",
        )}
      >
        {isCurrent ? (
          <>
            <Check className="h-4 w-4" />
            {t("provider.inUse", { defaultValue: "已启用" })}
          </>
        ) : (
          <>
            <Play className="h-4 w-4" />
            {t("provider.enable", { defaultValue: "启用" })}
          </>
        )}
      </Button>

      <Button size="sm" variant="outline" onClick={onEdit}>
        {t("common.edit", { defaultValue: "编辑" })}
      </Button>

      <Button
        size="sm"
        variant="outline"
        onClick={onConfigureUsage}
        title={t("provider.configureUsage", { defaultValue: "配置用量查询" })}
      >
        <BarChart3 className="h-4 w-4" />
      </Button>

      <Button
        size="sm"
        variant="ghost"
        onClick={onDelete}
        disabled={isCurrent}
        className={cn(
          "hover:text-red-500 hover:bg-red-100 dark:hover:text-red-400 dark:hover:bg-red-500/10",
          isCurrent &&
            "text-muted-foreground hover:text-muted-foreground hover:bg-transparent",
        )}
      >
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
}
