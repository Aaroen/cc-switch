import { useEffect, useMemo } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { useTheme } from "@/components/theme-provider";
import JsonEditor from "@/components/JsonEditor";
import {
  providerSchema,
  type ProviderFormData,
} from "@/lib/schemas/provider";

interface ProviderFormProps {
  submitLabel: string;
  onSubmit: (values: ProviderFormData) => void;
  onCancel: () => void;
  initialData?: {
    name?: string;
    websiteUrl?: string;
    settingsConfig?: Record<string, unknown>;
  };
}

const DEFAULT_CONFIG_PLACEHOLDER = `{
  "env": {},
  "config": {}
}`;

export function ProviderForm({
  submitLabel,
  onSubmit,
  onCancel,
  initialData,
}: ProviderFormProps) {
  const { t } = useTranslation();
  const { theme } = useTheme();

  const defaultValues: ProviderFormData = useMemo(
    () => ({
      name: initialData?.name ?? "",
      websiteUrl: initialData?.websiteUrl ?? "",
      settingsConfig: initialData?.settingsConfig
        ? JSON.stringify(initialData.settingsConfig, null, 2)
        : DEFAULT_CONFIG_PLACEHOLDER,
    }),
    [initialData],
  );

  const form = useForm<ProviderFormData>({
    resolver: zodResolver(providerSchema),
    defaultValues,
    mode: "onSubmit",
  });

  useEffect(() => {
    form.reset(defaultValues);
  }, [defaultValues, form]);

  const isDarkMode = useMemo(() => {
    if (theme === "dark") return true;
    if (theme === "light") return false;
    return typeof window !== "undefined"
      ? window.document.documentElement.classList.contains("dark")
      : false;
  }, [theme]);

  const handleSubmit = (values: ProviderFormData) => {
    onSubmit({
      ...values,
      websiteUrl: values.websiteUrl?.trim() ?? "",
      settingsConfig: values.settingsConfig.trim(),
    });
  };

  return (
    <Form {...form}>
      <form
        onSubmit={form.handleSubmit(handleSubmit)}
        className="space-y-6"
      >
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.name", { defaultValue: "供应商名称" })}
              </FormLabel>
              <FormControl>
                <Input
                  {...field}
                  placeholder={t("provider.namePlaceholder", {
                    defaultValue: "例如：Claude 官方",
                  })}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="websiteUrl"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.websiteUrl", { defaultValue: "官网链接" })}
              </FormLabel>
              <FormControl>
                <Input
                  {...field}
                  placeholder="https://"
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name="settingsConfig"
          render={({ field }) => (
            <FormItem>
              <FormLabel>
                {t("provider.configJson", { defaultValue: "配置 JSON" })}
              </FormLabel>
              <FormControl>
                <div className="rounded-md border">
                  <JsonEditor
                    value={field.value}
                    onChange={field.onChange}
                    placeholder={DEFAULT_CONFIG_PLACEHOLDER}
                    darkMode={isDarkMode}
                    rows={14}
                    showValidation
                  />
                </div>
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <div className="flex justify-end gap-2">
          <Button variant="outline" type="button" onClick={onCancel}>
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button type="submit">{submitLabel}</Button>
        </div>
      </form>
    </Form>
  );
}

export type ProviderFormValues = ProviderFormData;
