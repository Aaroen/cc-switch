import React from "react";
import { RefreshCw, AlertCircle, Clock } from "lucide-react";
import { useTranslation } from "react-i18next";
import { type AppId } from "@/lib/api";
import { useUsageQuery } from "@/lib/query/queries";
import { useAutoUsageQuery } from "@/hooks/useAutoUsageQuery";
import { UsageData, Provider } from "../types";

interface UsageFooterProps {
  provider: Provider;
  providerId: string;
  appId: AppId;
  usageEnabled: boolean; // 是否启用了用量查询
  isCurrent: boolean; // 是否为当前激活的供应商
}

const UsageFooter: React.FC<UsageFooterProps> = ({
  provider,
  providerId,
  appId,
  usageEnabled,
  isCurrent,
}) => {
  const { t } = useTranslation();

  // 手动查询（点击刷新按钮时使用）
  const {
    data: manualUsage,
    isFetching: loading,
    refetch,
  } = useUsageQuery(providerId, appId, usageEnabled);

  // 自动查询（仅对当前激活的供应商启用）
  const autoQuery = useAutoUsageQuery(provider, appId, isCurrent && usageEnabled);

  // 优先使用自动查询结果，如果没有则使用手动查询结果
  const usage = autoQuery.result || manualUsage;
  const isAutoQuerying = autoQuery.isQuerying;
  const lastQueriedAt = autoQuery.lastQueriedAt;

  // 只在启用用量查询且有数据时显示
  if (!usageEnabled || !usage) return null;

  // 错误状态
  if (!usage.success) {
    return (
      <div className="mt-3 pt-3 border-t border-border-default ">
        <div className="flex items-center justify-between gap-2 text-xs">
          <div className="flex items-center gap-2 text-red-500 dark:text-red-400">
            <AlertCircle size={14} />
            <span>{usage.error || t("usage.queryFailed")}</span>
          </div>

          {/* 刷新按钮 */}
          <button
            onClick={() => refetch()}
            disabled={loading}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50 flex-shrink-0"
            title={t("usage.refreshUsage")}
          >
            <RefreshCw size={12} className={loading ? "animate-spin" : ""} />
          </button>
        </div>
      </div>
    );
  }

  const usageDataList = usage.data || [];

  // 无数据时不显示
  if (usageDataList.length === 0) return null;

  return (
    <div className="mt-3 pt-3 border-t border-border-default ">
      {/* 标题行：包含刷新按钮和自动查询时间 */}
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs text-gray-500 dark:text-gray-400 font-medium">
          {t("usage.planUsage")}
        </span>
        <div className="flex items-center gap-2">
          {/* 自动查询时间提示 */}
          {lastQueriedAt && (
            <span className="text-[10px] text-gray-400 dark:text-gray-500 flex items-center gap-1">
              <Clock size={10} />
              {formatRelativeTime(lastQueriedAt, t)}
            </span>
          )}
          <button
            onClick={() => refetch()}
            disabled={loading || isAutoQuerying}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50"
            title={t("usage.refreshUsage")}
          >
            <RefreshCw
              size={12}
              className={loading || isAutoQuerying ? "animate-spin" : ""}
            />
          </button>
        </div>
      </div>

      {/* 套餐列表 */}
      <div className="flex flex-col gap-3">
        {usageDataList.map((usageData, index) => (
          <UsagePlanItem key={index} data={usageData} />
        ))}
      </div>
    </div>
  );
};

// 单个套餐数据展示组件
const UsagePlanItem: React.FC<{ data: UsageData }> = ({ data }) => {
  const { t } = useTranslation();
  const {
    planName,
    extra,
    isValid,
    invalidMessage,
    total,
    used,
    remaining,
    unit,
  } = data;

  // 判断套餐是否失效（isValid 为 false 或未定义时视为有效）
  const isExpired = isValid === false;

  return (
    <div className="flex items-center gap-3">
      {/* 标题部分：25% */}
      <div
        className="text-xs text-gray-500 dark:text-gray-400 min-w-0"
        style={{ width: "25%" }}
      >
        {planName ? (
          <span
            className={`font-medium truncate block ${isExpired ? "text-red-500 dark:text-red-400" : ""}`}
            title={planName}
          >
            💰 {planName}
          </span>
        ) : (
          <span className="opacity-50">—</span>
        )}
      </div>

      {/* 扩展字段：30% */}
      <div
        className="text-xs text-gray-500 dark:text-gray-400 min-w-0 flex items-center gap-2"
        style={{ width: "30%" }}
      >
        {extra && (
          <span
            className={`truncate ${isExpired ? "text-red-500 dark:text-red-400" : ""}`}
            title={extra}
          >
            {extra}
          </span>
        )}
        {isExpired && (
          <span className="text-red-500 dark:text-red-400 font-medium text-[10px] px-1.5 py-0.5 bg-red-50 dark:bg-red-900/20 rounded flex-shrink-0">
            {invalidMessage || t("usage.invalid")}
          </span>
        )}
      </div>

      {/* 用量信息：45% */}
      <div
        className="flex items-center justify-end gap-2 text-xs flex-shrink-0"
        style={{ width: "45%" }}
      >
        {/* 总额度 */}
        {total !== undefined && (
          <>
            <span className="text-gray-500 dark:text-gray-400">
              {t("usage.total")}
            </span>
            <span className="tabular-nums text-gray-600 dark:text-gray-400">
              {total === -1 ? "∞" : total.toFixed(2)}
            </span>
            <span className="text-gray-400 dark:text-gray-600">|</span>
          </>
        )}

        {/* 已用额度 */}
        {used !== undefined && (
          <>
            <span className="text-gray-500 dark:text-gray-400">
              {t("usage.used")}
            </span>
            <span className="tabular-nums text-gray-600 dark:text-gray-400">
              {used.toFixed(2)}
            </span>
            <span className="text-gray-400 dark:text-gray-600">|</span>
          </>
        )}

        {/* 剩余额度 - 突出显示 */}
        {remaining !== undefined && (
          <>
            <span className="text-gray-500 dark:text-gray-400">
              {t("usage.remaining")}
            </span>
            <span
              className={`font-semibold tabular-nums ${
                isExpired
                  ? "text-red-500 dark:text-red-400"
                  : remaining < (total || remaining) * 0.1
                    ? "text-orange-500 dark:text-orange-400"
                    : "text-green-600 dark:text-green-400"
              }`}
            >
              {remaining.toFixed(2)}
            </span>
          </>
        )}

        {unit && (
          <span className="text-gray-500 dark:text-gray-400">{unit}</span>
        )}
      </div>
    </div>
  );
};

// 格式化相对时间
function formatRelativeTime(
  timestamp: number,
  t: (key: string, options?: { count?: number }) => string
): string {
  const now = Date.now();
  const diff = Math.floor((now - timestamp) / 1000); // 秒

  if (diff < 60) {
    return t("usage.justNow");
  } else if (diff < 3600) {
    const minutes = Math.floor(diff / 60);
    return t("usage.minutesAgo", { count: minutes });
  } else if (diff < 86400) {
    const hours = Math.floor(diff / 3600);
    return t("usage.hoursAgo", { count: hours });
  } else {
    const days = Math.floor(diff / 86400);
    return t("usage.daysAgo", { count: days });
  }
}

export default UsageFooter;
