import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { UpdateProvider } from "./contexts/UpdateContext";
import "./index.css";
// 导入国际化配置
import "./i18n";
import { QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "@/components/theme-provider";
import { queryClient } from "@/lib/query";
import { Toaster } from "@/components/ui/sonner";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { exit } from "@tauri-apps/plugin-process";

// 根据平台添加 body class，便于平台特定样式
try {
  const ua = navigator.userAgent || "";
  const plat = (navigator.platform || "").toLowerCase();
  const isMac = /mac/i.test(ua) || plat.includes("mac");
  if (isMac) {
    document.body.classList.add("is-mac");
  }
} catch {
  // 忽略平台检测失败
}

// 监听后端的配置加载错误事件：仅提醒用户并在确认后退出，不修改任何配置文件
try {
  void listen("configLoadError", async (evt) => {
    const payload = evt.payload as { path?: string; error?: string } | null;
    const path = payload?.path ?? "~/.cc-switch/config.json";
    const detail = payload?.error ?? "Unknown error";

    await message(
      `无法读取配置文件：\n${path}\n\n错误详情：\n${detail}\n\n请手动检查 JSON 是否有效，或从同目录的备份文件（如 config.json.bak）恢复。`,
      { title: "配置加载失败", kind: "error" },
    );

    const shouldExit = await confirm("现在退出应用以进行修复？", {
      title: "退出确认",
      okLabel: "退出应用",
      cancelLabel: "取消",
    });

    if (shouldExit) {
      await exit(1);
    }
  });
} catch (e) {
  // 忽略事件订阅异常（例如在非 Tauri 环境下）
  console.error("订阅 configLoadError 事件失败", e);
}

async function bootstrap() {
  // 启动早期主动查询后端初始化错误，避免事件竞态
  try {
    const initError = (await invoke("get_init_error")) as
      | { path?: string; error?: string }
      | null;
    if (initError && (initError.path || initError.error)) {
      const path = initError.path ?? "~/.cc-switch/config.json";
      const detail = initError.error ?? "Unknown error";
      await message(
        `无法读取配置文件：\n${path}\n\n错误详情：\n${detail}\n\n请手动检查 JSON 是否有效，或从同目录的备份文件（如 config.json.bak）恢复。`,
        { title: "配置加载失败", kind: "error" },
      );
      const shouldExit = await confirm("现在退出应用以进行修复？", {
        title: "退出确认",
        okLabel: "退出应用",
        cancelLabel: "取消",
      });
      if (shouldExit) {
        await exit(1);
        return; // 退出流程
      }
    }
  } catch (e) {
    // 忽略拉取错误，继续渲染
    console.error("拉取初始化错误失败", e);
  }

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider defaultTheme="system" storageKey="cc-switch-theme">
          <UpdateProvider>
            <App />
            <Toaster />
          </UpdateProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

void bootstrap();
