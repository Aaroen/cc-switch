import { useMemo } from "react";
import { MoveVertical, Copy } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from "@dnd-kit/core";
import type { Provider } from "@/types";
import type { AppType } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ProviderActions } from "@/components/providers/ProviderActions";
import UsageFooter from "@/components/UsageFooter";

interface DragHandleProps {
  attributes: DraggableAttributes;
  listeners: DraggableSyntheticListeners;
  isDragging: boolean;
}

interface ProviderCardProps {
  provider: Provider;
  isCurrent: boolean;
  appType: AppType;
  isEditMode?: boolean;
  onSwitch: (provider: Provider) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onConfigureUsage: (provider: Provider) => void;
  onOpenWebsite: (url: string) => void;
  onDuplicate: (provider: Provider) => void;
  dragHandleProps?: DragHandleProps;
}

const extractApiUrl = (provider: Provider, fallbackText: string) => {
  if (provider.websiteUrl) {
    return provider.websiteUrl;
  }

  const config = provider.settingsConfig;

  if (config && typeof config === "object") {
    const envBase = (config as Record<string, any>)?.env?.ANTHROPIC_BASE_URL;
    if (typeof envBase === "string" && envBase.trim()) {
      return envBase;
    }

    const baseUrl = (config as Record<string, any>)?.config;

    if (typeof baseUrl === "string" && baseUrl.includes("base_url")) {
      const match = baseUrl.match(/base_url\s*=\s*['"]([^'"]+)['"]/);
      if (match?.[1]) {
        return match[1];
      }
    }
  }

  return fallbackText;
};

export function ProviderCard({
  provider,
  isCurrent,
  appType,
  isEditMode = false,
  onSwitch,
  onEdit,
  onDelete,
  onConfigureUsage,
  onOpenWebsite,
  onDuplicate,
  dragHandleProps,
}: ProviderCardProps) {
  const { t } = useTranslation();

  const fallbackUrlText = t("provider.notConfigured", {
    defaultValue: "未配置接口地址",
  });

  const displayUrl = useMemo(() => {
    return extractApiUrl(provider, fallbackUrlText);
  }, [provider, fallbackUrlText]);

  const usageEnabled = provider.meta?.usage_script?.enabled ?? false;

  const handleOpenWebsite = () => {
    if (!displayUrl || displayUrl === fallbackUrlText) {
      return;
    }
    onOpenWebsite(displayUrl);
  };

  return (
    <div
      className={cn(
        "rounded-lg bg-card p-4 shadow-sm",
        "transition-[border-color,background-color,box-shadow] duration-200",
        isCurrent
          ? "border-active border-border-active bg-primary/5"
          : "border border-border-default hover:border-border-hover",
        dragHandleProps?.isDragging &&
          "cursor-grabbing border-active border-border-dragging shadow-lg",
      )}
    >
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-start">
          <div
            className={cn(
              "flex items-center gap-1 overflow-hidden",
              "transition-[max-width,opacity] duration-200 ease-in-out",
              isEditMode ? "max-w-20 opacity-100" : "max-w-0 opacity-0",
            )}
            aria-hidden={!isEditMode}
          >
            <Button
              type="button"
              size="icon"
              variant="ghost"
              className={cn(
                "flex-shrink-0 cursor-grab active:cursor-grabbing",
                dragHandleProps?.isDragging && "cursor-grabbing",
              )}
              aria-label={t("provider.dragHandle")}
              disabled={!isEditMode}
              {...(dragHandleProps?.attributes ?? {})}
              {...(dragHandleProps?.listeners ?? {})}
            >
              <MoveVertical className="h-4 w-4" />
            </Button>

            <Button
              type="button"
              size="icon"
              variant="ghost"
              className="flex-shrink-0"
              onClick={() => onDuplicate(provider)}
              disabled={!isEditMode}
              aria-label={t("provider.duplicate")}
              title={t("provider.duplicate")}
            >
              <Copy className="h-4 w-4" />
            </Button>
          </div>

          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2 min-h-[20px]">
              <h3 className="text-base font-semibold leading-none">
                {provider.name}
              </h3>
              <span
                className={cn(
                  "rounded-full bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-500 dark:text-green-400 transition-opacity duration-200",
                  isCurrent ? "opacity-100" : "opacity-0 pointer-events-none",
                )}
              >
                {t("provider.currentlyUsing")}
              </span>
            </div>

            {displayUrl && (
              <button
                type="button"
                onClick={handleOpenWebsite}
                className="inline-flex items-center text-sm text-blue-500 transition-colors hover:underline dark:text-blue-400"
                title={displayUrl}
              >
                <span className="truncate">{displayUrl}</span>
              </button>
            )}
          </div>
        </div>

        <ProviderActions
          isCurrent={isCurrent}
          onSwitch={() => onSwitch(provider)}
          onEdit={() => onEdit(provider)}
          onConfigureUsage={() => onConfigureUsage(provider)}
          onDelete={() => onDelete(provider)}
        />
      </div>

      <UsageFooter
        providerId={provider.id}
        appType={appType}
        usageEnabled={usageEnabled}
      />
    </div>
  );
}
