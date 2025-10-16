import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Provider } from "@/types";
import type { AppType } from "@/lib/api";
import {
  ProviderForm,
  type ProviderFormValues,
} from "@/components/providers/forms/ProviderForm";

interface AddProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  appType: AppType;
  onSubmit: (provider: Omit<Provider, "id">) => Promise<void> | void;
}

export function AddProviderDialog({
  open,
  onOpenChange,
  appType,
  onSubmit,
}: AddProviderDialogProps) {
  const { t } = useTranslation();

  const handleSubmit = useCallback(
    async (values: ProviderFormValues) => {
      const parsedConfig = JSON.parse(values.settingsConfig) as Record<
        string,
        unknown
      >;

      const providerData: Omit<Provider, "id"> = {
        name: values.name.trim(),
        websiteUrl: values.websiteUrl?.trim() || undefined,
        settingsConfig: parsedConfig,
        ...(values.presetCategory ? { category: values.presetCategory } : {}),
      };

      await onSubmit(providerData);
      onOpenChange(false);
    },
    [onSubmit, onOpenChange],
  );

  const submitLabel =
    appType === "claude"
      ? t("provider.addClaudeProvider", { defaultValue: "添加 Claude 供应商" })
      : t("provider.addCodexProvider", { defaultValue: "添加 Codex 供应商" });

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{submitLabel}</DialogTitle>
          <DialogDescription>
            {t("provider.addDescription", {
              defaultValue: "填写信息后即可在列表中快速切换供应商。",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto -mx-6 px-6">
          <ProviderForm
            appType={appType}
            submitLabel={t("common.add", { defaultValue: "添加" })}
            onSubmit={handleSubmit}
            onCancel={() => onOpenChange(false)}
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}
