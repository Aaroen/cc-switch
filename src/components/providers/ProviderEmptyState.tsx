import { Users } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";

interface ProviderEmptyStateProps {
  onCreate?: () => void;
}

export function ProviderEmptyState({ onCreate }: ProviderEmptyStateProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-muted-foreground/30 p-10 text-center">
      <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
        <Users className="h-7 w-7 text-muted-foreground" />
      </div>
      <h3 className="text-lg font-semibold">
        {t("provider.noProviders", { defaultValue: "暂无供应商" })}
      </h3>
      <p className="mt-2 max-w-sm text-sm text-muted-foreground">
        {t("provider.noProvidersDescription", {
          defaultValue: "开始添加一个供应商以快速完成切换。",
        })}
      </p>
      {onCreate && (
        <Button className="mt-6" onClick={onCreate}>
          {t("provider.addProvider", { defaultValue: "添加供应商" })}
        </Button>
      )}
    </div>
  );
}
