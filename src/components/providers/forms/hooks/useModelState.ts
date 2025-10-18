import { useState, useCallback } from "react";

interface UseModelStateProps {
  settingsConfig: string;
  onConfigChange: (config: string) => void;
}

/**
 * 管理模型选择状态
 * 支持 ANTHROPIC_MODEL 和 ANTHROPIC_SMALL_FAST_MODEL
 */
export function useModelState({
  settingsConfig,
  onConfigChange,
}: UseModelStateProps) {
  const [claudeModel, setClaudeModel] = useState("");
  const [claudeSmallFastModel, setClaudeSmallFastModel] = useState("");

  const handleModelChange = useCallback(
    (
      field: "ANTHROPIC_MODEL" | "ANTHROPIC_SMALL_FAST_MODEL",
      value: string,
    ) => {
      if (field === "ANTHROPIC_MODEL") {
        setClaudeModel(value);
      } else {
        setClaudeSmallFastModel(value);
      }

      try {
        const currentConfig = settingsConfig
          ? JSON.parse(settingsConfig)
          : { env: {} };
        if (!currentConfig.env) currentConfig.env = {};

        if (value.trim()) {
          currentConfig.env[field] = value.trim();
        } else {
          delete currentConfig.env[field];
        }

        onConfigChange(JSON.stringify(currentConfig, null, 2));
      } catch (err) {
        // 如果 JSON 解析失败，不做处理
        console.error("Failed to update model config:", err);
      }
    },
    [settingsConfig, onConfigChange],
  );

  return {
    claudeModel,
    setClaudeModel,
    claudeSmallFastModel,
    setClaudeSmallFastModel,
    handleModelChange,
  };
}
