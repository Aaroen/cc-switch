import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import type { ProviderPreset } from "@/config/providerPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";

type PresetEntry = {
  id: string;
  preset: ProviderPreset | CodexProviderPreset;
};

interface ProviderPresetSelectorProps {
  selectedPresetId: string | null;
  groupedPresets: Record<string, PresetEntry[]>;
  categoryKeys: string[];
  presetCategoryLabels: Record<string, string>;
  onPresetChange: (value: string) => void;
}

export function ProviderPresetSelector({
  selectedPresetId,
  groupedPresets,
  categoryKeys,
  presetCategoryLabels,
  onPresetChange,
}: ProviderPresetSelectorProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-3">
      <FormLabel>
        {t("providerPreset.label")}
      </FormLabel>
      <div className="flex flex-wrap gap-2">
        {/* 自定义按钮 */}
        <button
          type="button"
          onClick={() => onPresetChange("custom")}
          className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
            selectedPresetId === "custom"
              ? "bg-emerald-500 text-white dark:bg-emerald-600"
              : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
          }`}
        >
          {t("providerPreset.custom")}
        </button>

        {/* 预设按钮 */}
        {categoryKeys.map((category) => {
          const entries = groupedPresets[category];
          if (!entries || entries.length === 0) return null;
          return entries.map((entry) => (
            <button
              key={entry.id}
              type="button"
              onClick={() => onPresetChange(entry.id)}
              className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                selectedPresetId === entry.id
                  ? "bg-emerald-500 text-white dark:bg-emerald-600"
                  : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
              }`}
              title={
                presetCategoryLabels[category] ??
                t("providerPreset.categoryOther", {
                  defaultValue: "其他",
                })
              }
            >
              {entry.preset.name}
            </button>
          ));
        })}
      </div>
      <p className="text-xs text-muted-foreground">
        {t("providerPreset.helper", {
          defaultValue: "选择预设后可继续调整下方字段。",
        })}
      </p>
    </div>
  );
}
