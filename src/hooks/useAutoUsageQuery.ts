import { useEffect, useRef, useState } from "react";
import { usageApi, type AppId } from "@/lib/api";
import type { Provider, UsageResult } from "@/types";

export interface AutoQueryState {
  result: UsageResult | null;
  lastQueriedAt: number | null;
  isQuerying: boolean;
  error: string | null;
}

/**
 * 自动用量查询 Hook
 * @param provider 供应商对象
 * @param appId 应用 ID（claude 或 codex）
 * @param enabled 是否启用（通常只对当前激活的供应商启用）
 * @returns 自动查询状态
 */
export function useAutoUsageQuery(
  provider: Provider,
  appId: AppId,
  enabled: boolean
): AutoQueryState {
  const [state, setState] = useState<AutoQueryState>({
    result: null,
    lastQueriedAt: null,
    isQuerying: false,
    error: null,
  });

  const timerRef = useRef<NodeJS.Timeout | null>(null);
  const isMountedRef = useRef(true);

  // 跟踪组件挂载状态
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    // 清理旧定时器
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }

    // 重置状态（切换供应商或禁用时）
    if (!enabled) {
      setState({
        result: null,
        lastQueriedAt: null,
        isQuerying: false,
        error: null,
      });
      return;
    }

    // 检查是否启用自动查询
    const usageScript = provider.meta?.usage_script;
    if (!usageScript?.enabled) {
      return;
    }

    const interval = usageScript.autoQueryInterval || 0;
    if (interval === 0) {
      return; // 间隔为 0，不启用自动查询
    }

    // 限制最小间隔为 1 分钟，避免过于频繁
    const actualInterval = Math.max(interval, 1);

    // 执行查询的函数
    const executeQuery = async () => {
      if (!isMountedRef.current) return;

      setState((prev) => ({ ...prev, isQuerying: true, error: null }));

      try {
        const result = await usageApi.query(provider.id, appId);

        if (isMountedRef.current) {
          setState({
            result,
            lastQueriedAt: Date.now(),
            isQuerying: false,
            error: result.success ? null : result.error || "Unknown error",
          });
        }
      } catch (error: any) {
        if (isMountedRef.current) {
          setState((prev) => ({
            ...prev,
            isQuerying: false,
            error: error?.message || "Query failed",
          }));
        }
        console.error("[AutoQuery] Failed:", error);
      }
    };

    // 立即执行一次查询
    executeQuery();

    // 设置定时器（间隔单位：分钟）
    timerRef.current = setInterval(executeQuery, actualInterval * 60 * 1000);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [provider.id, provider.meta?.usage_script, appId, enabled]);

  return state;
}
