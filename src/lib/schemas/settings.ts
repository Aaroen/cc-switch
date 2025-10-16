import { z } from "zod";

const directorySchema = z
  .string()
  .trim()
  .min(1, "路径不能为空")
  .optional()
  .or(z.literal(""));

export const settingsSchema = z.object({
  showInTray: z.boolean(),
  minimizeToTrayOnClose: z.boolean(),
  enableClaudePluginIntegration: z.boolean().optional(),
  claudeConfigDir: directorySchema.nullable().optional(),
  codexConfigDir: directorySchema.nullable().optional(),
  language: z.enum(["en", "zh"]).optional(),
  customEndpointsClaude: z.record(z.string(), z.unknown()).optional(),
  customEndpointsCodex: z.record(z.string(), z.unknown()).optional(),
});

export type SettingsFormData = z.infer<typeof settingsSchema>;
