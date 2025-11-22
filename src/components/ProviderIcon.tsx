import React, { useMemo } from "react";
import { getIcon, hasIcon } from "@/icons/extracted";
import { cn } from "@/lib/utils";

interface ProviderIconProps {
  icon?: string; // 图标名称
  name: string; // 供应商名称（用于 fallback）
  color?: string; // 自定义颜色 (Deprecated, kept for compatibility but ignored for SVG)
  size?: number | string; // 尺寸
  className?: string;
  showFallback?: boolean; // 是否显示 fallback
}

export const ProviderIcon: React.FC<ProviderIconProps> = ({
  icon,
  name,
  size = 32,
  className,
  showFallback = true,
}) => {
  // 获取图标 SVG
  const iconSvg = useMemo(() => {
    if (icon && hasIcon(icon)) {
      return getIcon(icon);
    }
    return "";
  }, [icon]);

  // 计算尺寸样式
  const sizeStyle = useMemo(() => {
    const sizeValue = typeof size === "number" ? `${size}px` : size;
    return {
      width: sizeValue,
      height: sizeValue,
    };
  }, [size]);

  // 如果有图标，显示图标
  if (iconSvg) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center flex-shrink-0",
          className,
        )}
        style={sizeStyle}
        dangerouslySetInnerHTML={{ __html: iconSvg }}
      />
    );
  }

  // Fallback：显示首字母
  if (showFallback) {
    const initials = name
      .split(" ")
      .map((word) => word[0])
      .join("")
      .toUpperCase()
      .slice(0, 2);
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center flex-shrink-0 rounded-lg",
          "bg-muted text-muted-foreground font-semibold",
          className,
        )}
        style={sizeStyle}
      >
        <span
          style={{
            fontSize: `${typeof size === "number" ? size * 0.4 : 14}px`,
          }}
        >
          {initials}
        </span>
      </span>
    );
  }

  return null;
};
