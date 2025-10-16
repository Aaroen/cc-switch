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
        ...(values.presetCategory
          ? { category: values.presetCategory }
          : {}),
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
      <DialogContent className="max-w-2xl">
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
        />
      </DialogContent>
    </Dialog>
  );
}
