import { z } from "zod";

export const providerSchema = z.object({
  name: z.string().min(1, "请填写供应商名称"),
  websiteUrl: z
    .string()
    .url("请输入有效的网址")
    .optional()
    .or(z.literal("")),
  settingsConfig: z
    .string()
    .min(1, "请填写配置内容")
    .refine((value) => {
      try {
        JSON.parse(value);
        return true;
      } catch {
        return false;
      }
    }, "配置 JSON 格式错误"),
});

export type ProviderFormData = z.infer<typeof providerSchema>;
