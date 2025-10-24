import { useTranslation } from "react-i18next";
import { FormLabel } from "@/components/ui/form";
import { ClaudeIcon, CodexIcon } from "@/components/BrandIcons";
import { Zap } from "lucide-react";
import type { ProviderPreset } from "@/config/providerPresets";
import type { CodexProviderPreset } from "@/config/codexProviderPresets";
import type { ProviderCategory } from "@/types";

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
  category?: ProviderCategory; // 新增：当前选中的分类
}

export function ProviderPresetSelector({
  selectedPresetId,
  groupedPresets,
  categoryKeys,
  presetCategoryLabels,
  onPresetChange,
  category,
}: ProviderPresetSelectorProps) {
  const { t } = useTranslation();

  // 根据分类获取提示文字
  const getCategoryHint = () => {
    switch (category) {
      case "official":
        return t("providerForm.officialHint", {
          defaultValue: "💡 官方供应商使用浏览器登录，无需配置 API Key",
        });
      case "cn_official":
        return t("providerForm.cnOfficialApiKeyHint", {
          defaultValue: "💡 国产官方供应商只需填写 API Key，请求地址已预设",
        });
      case "aggregator":
        return t("providerForm.aggregatorApiKeyHint", {
          defaultValue: "💡 聚合服务供应商只需填写 API Key 即可使用",
        });
      case "third_party":
        return t("providerForm.thirdPartyApiKeyHint", {
          defaultValue: "💡 第三方供应商需要填写 API Key 和请求地址",
        });
      case "custom":
        return t("providerForm.customApiKeyHint", {
          defaultValue: "💡 自定义配置需手动填写所有必要字段",
        });
      default:
        return t("providerPreset.hint", {
          defaultValue: "选择预设后可继续调整下方字段。",
        });
    }
  };

  // 渲染预设按钮的图标
  const renderPresetIcon = (preset: ProviderPreset | CodexProviderPreset) => {
    const iconType = preset.theme?.icon;
    if (!iconType) return null;

    switch (iconType) {
      case "claude":
        return <ClaudeIcon size={14} />;
      case "codex":
        return <CodexIcon size={14} />;
      case "generic":
        return <Zap size={14} />;
      default:
        return null;
    }
  };

  // 获取预设按钮的样式类名
  const getPresetButtonClass = (
    isSelected: boolean,
    preset: ProviderPreset | CodexProviderPreset,
  ) => {
    const baseClass =
      "inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors";

    if (isSelected) {
      // 如果有自定义主题，使用自定义颜色
      if (preset.theme?.backgroundColor) {
        return `${baseClass} text-white`;
      }
      // 默认使用主题蓝色
      return `${baseClass} bg-blue-500 text-white dark:bg-blue-600`;
    }

    return `${baseClass} bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700`;
  };

  // 获取预设按钮的内联样式（用于自定义背景色）
  const getPresetButtonStyle = (
    isSelected: boolean,
    preset: ProviderPreset | CodexProviderPreset,
  ) => {
    if (!isSelected || !preset.theme?.backgroundColor) {
      return undefined;
    }

    return {
      backgroundColor: preset.theme.backgroundColor,
      color: preset.theme.textColor || "#FFFFFF",
    };
  };

  return (
    <div className="space-y-3">
      <FormLabel>{t("providerPreset.label")}</FormLabel>
      <div className="flex flex-wrap gap-2">
        {/* 自定义按钮 */}
        <button
          type="button"
          onClick={() => onPresetChange("custom")}
          className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
            selectedPresetId === "custom"
              ? "bg-blue-500 text-white dark:bg-blue-600"
              : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
          }`}
        >
          {t("providerPreset.custom")}
        </button>

        {/* 预设按钮 */}
        {categoryKeys.map((category) => {
          const entries = groupedPresets[category];
          if (!entries || entries.length === 0) return null;
          return entries.map((entry) => {
            const isSelected = selectedPresetId === entry.id;
            return (
              <button
                key={entry.id}
                type="button"
                onClick={() => onPresetChange(entry.id)}
                className={getPresetButtonClass(isSelected, entry.preset)}
                style={getPresetButtonStyle(isSelected, entry.preset)}
                title={
                  presetCategoryLabels[category] ??
                  t("providerPreset.categoryOther", {
                    defaultValue: "其他",
                  })
                }
              >
                {renderPresetIcon(entry.preset)}
                {entry.preset.name}
              </button>
            );
          });
        })}
      </div>
      <p className="text-xs text-muted-foreground">{getCategoryHint()}</p>
    </div>
  );
}
