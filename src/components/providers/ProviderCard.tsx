import { useMemo } from "react";
import { GripVertical, Link } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from "@dnd-kit/core";
import type { Provider } from "@/types";
import type { AppType } from "@/lib/api";
import { cn } from "@/lib/utils";
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
        "rounded-lg border bg-card p-4 shadow-sm transition-[box-shadow,transform] duration-200",
        isCurrent
          ? "border-primary/70 bg-primary/5"
          : "border-border hover:border-primary/40",
        dragHandleProps?.isDragging &&
          "cursor-grabbing border-primary/60 shadow-lg",
      )}
    >
      <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex flex-1 items-start gap-3">
          <button
            type="button"
            className={cn(
              "mt-1 flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-md border text-muted-foreground transition-all duration-200",
              isEditMode
                ? "border-muted hover:border-primary hover:text-foreground opacity-100"
                : "border-transparent opacity-0 pointer-events-none",
              dragHandleProps?.isDragging && "border-primary text-primary",
            )}
            aria-label={t("provider.dragHandle")}
            {...(dragHandleProps?.attributes ?? {})}
            {...(dragHandleProps?.listeners ?? {})}
          >
            <GripVertical className="h-4 w-4" />
          </button>

          <div className="space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-base font-semibold leading-none">
                {provider.name}
              </h3>
              {isCurrent && (
                <span className="rounded-full bg-green-500/10 px-2 py-0.5 text-xs font-medium text-green-500 dark:text-green-400">
                  {t("provider.currentlyUsing")}
                </span>
              )}
            </div>

            {displayUrl && (
              <button
                type="button"
                onClick={handleOpenWebsite}
                className="inline-flex items-center text-sm text-blue-400 transition-colors hover:underline dark:text-blue-300"
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
