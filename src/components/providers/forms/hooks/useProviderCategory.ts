import { useState, useEffect } from "react";
import type { ProviderCategory } from "@/types";
import type { AppType } from "@/lib/api";
import { providerPresets } from "@/config/providerPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";

interface UseProviderCategoryProps {
  appType: AppType;
  selectedPresetId: string | null;
  isEditMode: boolean;
}

/**
 * 管理供应商类别状态
 * 根据选择的预设自动更新类别
 */
export function useProviderCategory({
  appType,
  selectedPresetId,
  isEditMode,
}: UseProviderCategoryProps) {
  const [category, setCategory] = useState<ProviderCategory | undefined>(
    undefined,
  );

  useEffect(() => {
    // 编辑模式不自动设置类别
    if (isEditMode) return;

    if (selectedPresetId === "custom") {
      setCategory("custom");
      return;
    }

    if (!selectedPresetId) return;

    // 从预设 ID 提取索引
    const match = selectedPresetId.match(/^(claude|codex)-(\d+)$/);
    if (!match) return;

    const [, type, indexStr] = match;
    const index = parseInt(indexStr, 10);

    if (type === "codex" && appType === "codex") {
      const preset = codexProviderPresets[index];
      if (preset) {
        setCategory(
          preset.category || (preset.isOfficial ? "official" : undefined),
        );
      }
    } else if (type === "claude" && appType === "claude") {
      const preset = providerPresets[index];
      if (preset) {
        setCategory(
          preset.category || (preset.isOfficial ? "official" : undefined),
        );
      }
    }
  }, [appType, selectedPresetId, isEditMode]);

  return { category, setCategory };
}
