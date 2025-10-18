import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import type { Provider } from "@/types";
import {
  ProviderForm,
  type ProviderFormValues,
} from "@/components/providers/forms/ProviderForm";
import type { AppType } from "@/lib/api";

interface EditProviderDialogProps {
  open: boolean;
  provider: Provider | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (provider: Provider) => Promise<void> | void;
  appType: AppType;
}

export function EditProviderDialog({
  open,
  provider,
  onOpenChange,
  onSubmit,
  appType,
}: EditProviderDialogProps) {
  const { t } = useTranslation();

  const handleSubmit = useCallback(
    async (values: ProviderFormValues) => {
      if (!provider) return;

      const parsedConfig = JSON.parse(values.settingsConfig) as Record<
        string,
        unknown
      >;

      const updatedProvider: Provider = {
        ...provider,
        name: values.name.trim(),
        websiteUrl: values.websiteUrl?.trim() || undefined,
        settingsConfig: parsedConfig,
        ...(values.presetCategory ? { category: values.presetCategory } : {}),
      };

      await onSubmit(updatedProvider);
      onOpenChange(false);
    },
    [onSubmit, onOpenChange, provider],
  );

  if (!provider) {
    return null;
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>
            {t("provider.editProvider", { defaultValue: "编辑供应商" })}
          </DialogTitle>
          <DialogDescription>
            {t("provider.editDescription", {
              defaultValue: "更新配置后将立即应用到当前供应商。",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 py-4">
          <ProviderForm
            appType={appType}
            submitLabel={t("common.save", { defaultValue: "保存" })}
            onSubmit={handleSubmit}
            onCancel={() => onOpenChange(false)}
            initialData={{
              name: provider.name,
              websiteUrl: provider.websiteUrl,
              settingsConfig: provider.settingsConfig,
            }}
            showButtons={false}
          />
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button type="submit" form="provider-form">
            {t("common.save", { defaultValue: "保存" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
