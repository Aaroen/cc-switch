import { useState, useEffect, useCallback, useRef } from "react";
import {
  updateTomlCommonConfigSnippet,
  hasTomlCommonConfigSnippet,
} from "@/utils/providerConfigUtils";

const CODEX_COMMON_CONFIG_STORAGE_KEY = "cc-switch:codex-common-config-snippet";
const DEFAULT_CODEX_COMMON_CONFIG_SNIPPET = `# Common Codex config
# Add your common TOML configuration here`;

interface UseCodexCommonConfigProps {
  codexConfig: string;
  onConfigChange: (config: string) => void;
  initialData?: {
    settingsConfig?: Record<string, unknown>;
  };
}

/**
 * 管理 Codex 通用配置片段 (TOML 格式)
 */
export function useCodexCommonConfig({
  codexConfig,
  onConfigChange,
  initialData,
}: UseCodexCommonConfigProps) {
  const [useCommonConfig, setUseCommonConfig] = useState(false);
  const [commonConfigSnippet, setCommonConfigSnippetState] = useState<string>(
    () => {
      if (typeof window === "undefined") {
        return DEFAULT_CODEX_COMMON_CONFIG_SNIPPET;
      }
      try {
        const stored = window.localStorage.getItem(
          CODEX_COMMON_CONFIG_STORAGE_KEY,
        );
        if (stored && stored.trim()) {
          return stored;
        }
      } catch {
        // ignore localStorage 读取失败
      }
      return DEFAULT_CODEX_COMMON_CONFIG_SNIPPET;
    },
  );
  const [commonConfigError, setCommonConfigError] = useState("");

  // 用于跟踪是否正在通过通用配置更新
  const isUpdatingFromCommonConfig = useRef(false);

  // 初始化时检查通用配置片段（编辑模式）
  useEffect(() => {
    if (initialData?.settingsConfig) {
      const config =
        typeof initialData.settingsConfig.config === "string"
          ? initialData.settingsConfig.config
          : "";
      const hasCommon = hasTomlCommonConfigSnippet(config, commonConfigSnippet);
      setUseCommonConfig(hasCommon);
    }
  }, [initialData, commonConfigSnippet]);

  // 同步本地存储的通用配置片段
  useEffect(() => {
    if (typeof window === "undefined") return;
    try {
      if (commonConfigSnippet.trim()) {
        window.localStorage.setItem(
          CODEX_COMMON_CONFIG_STORAGE_KEY,
          commonConfigSnippet,
        );
      } else {
        window.localStorage.removeItem(CODEX_COMMON_CONFIG_STORAGE_KEY);
      }
    } catch {
      // ignore
    }
  }, [commonConfigSnippet]);

  // 处理通用配置开关
  const handleCommonConfigToggle = useCallback(
    (checked: boolean) => {
      const { updatedConfig, error: snippetError } =
        updateTomlCommonConfigSnippet(
          codexConfig,
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
    [codexConfig, commonConfigSnippet, onConfigChange],
  );

  // 处理通用配置片段变化
  const handleCommonConfigSnippetChange = useCallback(
    (value: string) => {
      const previousSnippet = commonConfigSnippet;
      setCommonConfigSnippetState(value);

      if (!value.trim()) {
        setCommonConfigError("");
        if (useCommonConfig) {
          const { updatedConfig } = updateTomlCommonConfigSnippet(
            codexConfig,
            previousSnippet,
            false,
          );
          onConfigChange(updatedConfig);
          setUseCommonConfig(false);
        }
        return;
      }

      // TOML 格式校验较为复杂，暂时不做校验，直接清空错误
      setCommonConfigError("");

      // 若当前启用通用配置，需要替换为最新片段
      if (useCommonConfig) {
        const removeResult = updateTomlCommonConfigSnippet(
          codexConfig,
          previousSnippet,
          false,
        );
        if (removeResult.error) {
          setCommonConfigError(removeResult.error);
          return;
        }
        const addResult = updateTomlCommonConfigSnippet(
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
    [commonConfigSnippet, codexConfig, useCommonConfig, onConfigChange],
  );

  // 当配置变化时检查是否包含通用配置（但避免在通过通用配置更新时检查）
  useEffect(() => {
    if (isUpdatingFromCommonConfig.current) {
      return;
    }
    const hasCommon = hasTomlCommonConfigSnippet(
      codexConfig,
      commonConfigSnippet,
    );
    setUseCommonConfig(hasCommon);
  }, [codexConfig, commonConfigSnippet]);

  return {
    useCommonConfig,
    commonConfigSnippet,
    commonConfigError,
    handleCommonConfigToggle,
    handleCommonConfigSnippetChange,
  };
}
