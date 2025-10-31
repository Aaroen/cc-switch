import { useState, useEffect, useCallback } from "react";

interface UseKimiModelSelectorProps {
  initialData?: {
    settingsConfig?: Record<string, unknown>;
  };
  settingsConfig: string;
  onConfigChange: (config: string) => void;
  selectedPresetId: string | null;
  presetName?: string;
}

/**
 * 管理 Kimi 模型选择器的状态和逻辑
 */
export function useKimiModelSelector({
  initialData,
  settingsConfig,
  onConfigChange,
  selectedPresetId,
  presetName = "",
}: UseKimiModelSelectorProps) {
  const [kimiAnthropicModel, setKimiAnthropicModel] = useState("");
  const [kimiAnthropicSmallFastModel, setKimiAnthropicSmallFastModel] =
    useState("");

  // 判断是否显示 Kimi 模型选择器
  const shouldShowKimiSelector =
    selectedPresetId !== null &&
    selectedPresetId !== "custom" &&
    presetName.includes("Kimi");

  // 判断是否正在编辑 Kimi 供应商
  const isEditingKimi = Boolean(
    initialData &&
      settingsConfig.includes("api.moonshot.cn") &&
      settingsConfig.includes("ANTHROPIC_MODEL"),
  );

  const shouldShow = shouldShowKimiSelector || isEditingKimi;

  // 初始化 Kimi 模型选择（编辑模式）
  useEffect(() => {
    if (
      initialData?.settingsConfig &&
      typeof initialData.settingsConfig === "object"
    ) {
      const config = initialData.settingsConfig as {
        env?: Record<string, unknown>;
      };
      if (config.env) {
        const model =
          typeof config.env.ANTHROPIC_MODEL === "string"
            ? config.env.ANTHROPIC_MODEL
            : "";
        const smallFastModel =
          typeof config.env.ANTHROPIC_SMALL_FAST_MODEL === "string"
            ? config.env.ANTHROPIC_SMALL_FAST_MODEL
            : "";
        setKimiAnthropicModel(model);
        setKimiAnthropicSmallFastModel(smallFastModel);
      }
    }
  }, [initialData]);

  // 处理 Kimi 模型变化
  const handleKimiModelChange = useCallback(
    (
      field: "ANTHROPIC_MODEL" | "ANTHROPIC_SMALL_FAST_MODEL",
      value: string,
    ) => {
      if (field === "ANTHROPIC_MODEL") {
        setKimiAnthropicModel(value);
      } else {
        setKimiAnthropicSmallFastModel(value);
      }

      // 更新配置 JSON
      try {
        const currentConfig = JSON.parse(settingsConfig || "{}");
        if (!currentConfig.env) currentConfig.env = {};
        currentConfig.env[field] = value;

        const updatedConfigString = JSON.stringify(currentConfig, null, 2);
        onConfigChange(updatedConfigString);
      } catch (err) {
        console.error("更新 Kimi 模型配置失败:", err);
      }
    },
    [settingsConfig, onConfigChange],
  );

  // 当选择 Kimi 预设时，同步模型值
  useEffect(() => {
    if (shouldShowKimiSelector && settingsConfig) {
      try {
        const config = JSON.parse(settingsConfig);
        if (config.env) {
          const model = config.env.ANTHROPIC_MODEL || "";
          const smallFastModel = config.env.ANTHROPIC_SMALL_FAST_MODEL || "";
          setKimiAnthropicModel(model);
          setKimiAnthropicSmallFastModel(smallFastModel);
        }
      } catch {
        // ignore
      }
    }
  }, [shouldShowKimiSelector, settingsConfig]);

  return {
    shouldShow,
    kimiAnthropicModel,
    kimiAnthropicSmallFastModel,
    handleKimiModelChange,
  };
}
