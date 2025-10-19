import { BarChart3, Check, Edit, Play, Trash2 } from "lucide-react";
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
          "w-20",
          isCurrent && "text-muted-foreground hover:text-muted-foreground",
        )}
      >
        {isCurrent ? (
          <>
            <Check className="h-4 w-4" />
            {t("provider.inUse")}
          </>
        ) : (
          <>
            <Play className="h-4 w-4" />
            {t("provider.enable")}
          </>
        )}
      </Button>

      <div className="flex items-center gap-0">
        <Button
          size="sm"
          variant="ghost"
          onClick={onEdit}
          title={t("common.edit")}
          className="px-2 hover:bg-muted"
        >
          <Edit className="h-4 w-4" />
        </Button>

        <Button
          size="sm"
          variant="ghost"
          onClick={onConfigureUsage}
          title={t("provider.configureUsage")}
          className="px-2 hover:bg-muted"
        >
          <BarChart3 className="h-4 w-4" />
        </Button>

        <Button
          size="sm"
          variant="ghost"
          onClick={onDelete}
          disabled={isCurrent}
          title={t("common.delete")}
          className={cn(
            "px-2 hover:bg-muted hover:text-red-500 dark:hover:text-red-400",
            isCurrent && "text-muted-foreground hover:text-muted-foreground hover:bg-transparent",
          )}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
