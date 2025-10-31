import { useState, useEffect, useCallback, useRef } from "react";
import {
  updateCommonConfigSnippet,
  hasCommonConfigSnippet,
  validateJsonConfig,
} from "@/utils/providerConfigUtils";

const COMMON_CONFIG_STORAGE_KEY = "cc-switch:common-config-snippet";
const DEFAULT_COMMON_CONFIG_SNIPPET = `{
  "includeCoAuthoredBy": false
}`;

interface UseCommonConfigSnippetProps {
  settingsConfig: string;
  onConfigChange: (config: string) => void;
  initialData?: {
    settingsConfig?: Record<string, unknown>;
  };
}

/**
 * 管理 Claude 通用配置片段
 */
export function useCommonConfigSnippet({
  settingsConfig,
  onConfigChange,
  initialData,
}: UseCommonConfigSnippetProps) {
  const [useCommonConfig, setUseCommonConfig] = useState(false);
  const [commonConfigSnippet, setCommonConfigSnippetState] = useState<string>(
    () => {
      if (typeof window === "undefined") {
        return DEFAULT_COMMON_CONFIG_SNIPPET;
      }
      try {
        const stored = window.localStorage.getItem(COMMON_CONFIG_STORAGE_KEY);
        if (stored && stored.trim()) {
          return stored;
        }
      } catch {
        // ignore localStorage 读取失败
      }
      return DEFAULT_COMMON_CONFIG_SNIPPET;
    },
  );
  const [commonConfigError, setCommonConfigError] = useState("");

  // 用于跟踪是否正在通过通用配置更新
  const isUpdatingFromCommonConfig = useRef(false);

  // 初始化时检查通用配置片段（编辑模式）
  useEffect(() => {
    if (initialData) {
      const configString = JSON.stringify(initialData.settingsConfig, null, 2);
      const hasCommon = hasCommonConfigSnippet(
        configString,
        commonConfigSnippet,
      );
      setUseCommonConfig(hasCommon);
    }
  }, [initialData, commonConfigSnippet]);

  // 同步本地存储的通用配置片段
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      if (commonConfigSnippet.trim()) {
        window.localStorage.setItem(
          COMMON_CONFIG_STORAGE_KEY,
          commonConfigSnippet,
        );
      } else {
        window.localStorage.removeItem(COMMON_CONFIG_STORAGE_KEY);
      }
    } catch {
      // ignore
    }
  }, [commonConfigSnippet]);

  // 处理通用配置开关
  const handleCommonConfigToggle = useCallback(
    (checked: boolean) => {
      const { updatedConfig, error: snippetError } = updateCommonConfigSnippet(
        settingsConfig,
        commonConfigSnippet,
        checked,
      );

      if (snippetError) {
        setCommonConfigError(snippetError);
        setUseCommonConfig(false);
        return;
      }

      setCommonConfigError("");
      setUseCommonConfig(checked);
      // 标记正在通过通用配置更新
      isUpdatingFromCommonConfig.current = true;
      onConfigChange(updatedConfig);
      // 在下一个事件循环中重置标记
      setTimeout(() => {
        isUpdatingFromCommonConfig.current = false;
      }, 0);
    },
    [settingsConfig, commonConfigSnippet, onConfigChange],
  );

  // 处理通用配置片段变化
  const handleCommonConfigSnippetChange = useCallback(
    (value: string) => {
      const previousSnippet = commonConfigSnippet;
      setCommonConfigSnippetState(value);

      if (!value.trim()) {
        setCommonConfigError("");
        if (useCommonConfig) {
          const { updatedConfig } = updateCommonConfigSnippet(
            settingsConfig,
            previousSnippet,
            false,
          );
          onConfigChange(updatedConfig);
          setUseCommonConfig(false);
        }
        return;
      }

      // 验证JSON格式
      const validationError = validateJsonConfig(value, "通用配置片段");
      if (validationError) {
        setCommonConfigError(validationError);
      } else {
        setCommonConfigError("");
      }

      // 若当前启用通用配置且格式正确，需要替换为最新片段
      if (useCommonConfig && !validationError) {
        const removeResult = updateCommonConfigSnippet(
          settingsConfig,
          previousSnippet,
          false,
        );
        if (removeResult.error) {
          setCommonConfigError(removeResult.error);
          return;
        }
        const addResult = updateCommonConfigSnippet(
          removeResult.updatedConfig,
          value,
          true,
        );

        if (addResult.error) {
          setCommonConfigError(addResult.error);
          return;
        }

        // 标记正在通过通用配置更新，避免触发状态检查
        isUpdatingFromCommonConfig.current = true;
        onConfigChange(addResult.updatedConfig);
        // 在下一个事件循环中重置标记
        setTimeout(() => {
          isUpdatingFromCommonConfig.current = false;
        }, 0);
      }
    },
    [commonConfigSnippet, settingsConfig, useCommonConfig, onConfigChange],
  );

  // 当配置变化时检查是否包含通用配置（但避免在通过通用配置更新时检查）
  useEffect(() => {
    if (isUpdatingFromCommonConfig.current) {
      return;
    }
    const hasCommon = hasCommonConfigSnippet(
      settingsConfig,
      commonConfigSnippet,
    );
    setUseCommonConfig(hasCommon);
  }, [settingsConfig, commonConfigSnippet]);

  return {
    useCommonConfig,
    commonConfigSnippet,
    commonConfigError,
    handleCommonConfigToggle,
    handleCommonConfigSnippetChange,
  };
}
