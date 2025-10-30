import type { CustomEndpoint, ProviderMeta } from "@/types";

/**
 * 合并供应商元数据中的自定义端点。
 * - 当 customEndpoints 为空对象或 null 时，移除自定义端点但保留其它元数据。
 * - 当 customEndpoints 存在时，覆盖原有自定义端点。
 * - 若结果为空对象则返回 undefined，避免写入空 meta。
 */
export function mergeProviderMeta(
  initialMeta: ProviderMeta | undefined,
  customEndpoints: Record<string, CustomEndpoint> | null | undefined,
): ProviderMeta | undefined {
  const hasCustomEndpoints =
    !!customEndpoints && Object.keys(customEndpoints).length > 0;

  if (hasCustomEndpoints) {
    return {
      ...(initialMeta ? { ...initialMeta } : {}),
      custom_endpoints: customEndpoints!,
    };
  }

  if (!initialMeta) {
    return undefined;
  }

  if ("custom_endpoints" in initialMeta) {
    const { custom_endpoints, ...rest } = initialMeta;
    return Object.keys(rest).length > 0 ? rest : undefined;
  }

  return { ...initialMeta };
}
