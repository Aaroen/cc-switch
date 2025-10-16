import { z } from "zod";

const mcpServerSpecSchema = z.object({
  type: z.enum(["stdio", "http"]).optional(),
  command: z.string().trim().min(1, "请输入可执行命令").optional(),
  args: z.array(z.string()).optional(),
  env: z.record(z.string(), z.string()).optional(),
  cwd: z.string().optional(),
  url: z.string().url("请输入有效的 URL").optional(),
  headers: z.record(z.string(), z.string()).optional(),
});

export const mcpServerSchema = z.object({
  id: z.string().min(1, "请输入服务器 ID"),
  name: z.string().optional(),
  description: z.string().optional(),
  tags: z.array(z.string()).optional(),
  homepage: z.string().url().optional(),
  docs: z.string().url().optional(),
  enabled: z.boolean().optional(),
  server: mcpServerSpecSchema,
});

export type McpServerFormData = z.infer<typeof mcpServerSchema>;
