import { useTranslation } from "react-i18next";
import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import type { UseFormReturn } from "react-hook-form";
import type { ProviderFormData } from "@/lib/schemas/provider";

interface BasicFormFieldsProps {
  form: UseFormReturn<ProviderFormData>;
}

export function BasicFormFields({ form }: BasicFormFieldsProps) {
  const { t } = useTranslation();

  return (
    <>
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
              <Input {...field} placeholder="https://" />
            </FormControl>
            <FormMessage />
          </FormItem>
        )}
      />
    </>
  );
}
