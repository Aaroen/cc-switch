import React from "react";
import { useTranslation } from "react-i18next";

interface CodexAuthSectionProps {
  value: string;
  onChange: (value: string) => void;
  onBlur?: () => void;
  error?: string;
}

/**
 * CodexAuthSection - Auth JSON editor section
 */
export const CodexAuthSection: React.FC<CodexAuthSectionProps> = ({
  value,
  onChange,
  onBlur,
  error,
}) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-2">
      <label
        htmlFor="codexAuth"
        className="block text-sm font-medium text-gray-900 dark:text-gray-100"
      >
        {t("codexConfig.authJson")}
      </label>

      <textarea
        id="codexAuth"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        placeholder={t("codexConfig.authJsonPlaceholder")}
        rows={6}
        required
        className="w-full px-3 py-2 border border-border-default  dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20  transition-colors resize-y min-h-[8rem]"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="none"
        spellCheck={false}
        lang="en"
        inputMode="text"
        data-gramm="false"
        data-gramm_editor="false"
        data-enable-grammarly="false"
      />

      {error && (
        <p className="text-xs text-red-500 dark:text-red-400">{error}</p>
      )}

      <p className="text-xs text-gray-500 dark:text-gray-400">
        {t("codexConfig.authJsonHint")}
      </p>
    </div>
  );
};

interface CodexConfigSectionProps {
  value: string;
  onChange: (value: string) => void;
  useCommonConfig: boolean;
  onCommonConfigToggle: (checked: boolean) => void;
  onEditCommonConfig: () => void;
  commonConfigError?: string;
  configError?: string;
}

/**
 * CodexConfigSection - Config TOML editor section
 */
export const CodexConfigSection: React.FC<CodexConfigSectionProps> = ({
  value,
  onChange,
  useCommonConfig,
  onCommonConfigToggle,
  onEditCommonConfig,
  commonConfigError,
  configError,
}) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label
          htmlFor="codexConfig"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          {t("codexConfig.configToml")}
        </label>

        <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={useCommonConfig}
            onChange={(e) => onCommonConfigToggle(e.target.checked)}
            className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-border-default  rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
          />
          {t("codexConfig.writeCommonConfig")}
        </label>
      </div>

      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={onEditCommonConfig}
          className="text-xs text-blue-500 dark:text-blue-400 hover:underline"
        >
          {t("codexConfig.editCommonConfig")}
        </button>
      </div>

      {commonConfigError && (
        <p className="text-xs text-red-500 dark:text-red-400 text-right">
          {commonConfigError}
        </p>
      )}

      <textarea
        id="codexConfig"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder=""
        rows={8}
        className="w-full px-3 py-2 border border-border-default  dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20  transition-colors resize-y min-h-[10rem]"
        autoComplete="off"
        autoCorrect="off"
        autoCapitalize="none"
        spellCheck={false}
        lang="en"
        inputMode="text"
        data-gramm="false"
        data-gramm_editor="false"
        data-enable-grammarly="false"
      />

      {configError && (
        <p className="text-xs text-red-500 dark:text-red-400">{configError}</p>
      )}

      <p className="text-xs text-gray-500 dark:text-gray-400">
        {t("codexConfig.configTomlHint")}
      </p>
    </div>
  );
};
