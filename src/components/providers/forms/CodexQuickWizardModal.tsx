import React, { useState, useRef } from "react";
import { X, Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import { isLinux } from "@/lib/platform";
import {
  generateThirdPartyAuth,
  generateThirdPartyConfig,
} from "@/config/codexProviderPresets";

interface CodexQuickWizardModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApply: (auth: string, config: string, extras: {
    websiteUrl?: string;
    displayName?: string;
  }) => void;
}

/**
 * CodexQuickWizardModal - Codex quick configuration wizard
 * Helps users quickly generate auth.json and config.toml
 */
export const CodexQuickWizardModal: React.FC<CodexQuickWizardModalProps> = ({
  isOpen,
  onClose,
  onApply,
}) => {
  const { t } = useTranslation();

  const [templateApiKey, setTemplateApiKey] = useState("");
  const [templateProviderName, setTemplateProviderName] = useState("");
  const [templateBaseUrl, setTemplateBaseUrl] = useState("");
  const [templateWebsiteUrl, setTemplateWebsiteUrl] = useState("");
  const [templateModelName, setTemplateModelName] = useState("gpt-5-codex");
  const [templateDisplayName, setTemplateDisplayName] = useState("");

  const apiKeyInputRef = useRef<HTMLInputElement>(null);
  const baseUrlInputRef = useRef<HTMLInputElement>(null);
  const modelNameInputRef = useRef<HTMLInputElement>(null);
  const displayNameInputRef = useRef<HTMLInputElement>(null);

  const resetForm = () => {
    setTemplateApiKey("");
    setTemplateProviderName("");
    setTemplateBaseUrl("");
    setTemplateWebsiteUrl("");
    setTemplateModelName("gpt-5-codex");
    setTemplateDisplayName("");
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  const applyTemplate = () => {
    const requiredInputs = [
      displayNameInputRef.current,
      apiKeyInputRef.current,
      baseUrlInputRef.current,
      modelNameInputRef.current,
    ];

    for (const input of requiredInputs) {
      if (input && !input.checkValidity()) {
        input.reportValidity();
        input.focus();
        return;
      }
    }

    const trimmedKey = templateApiKey.trim();
    const trimmedBaseUrl = templateBaseUrl.trim();
    const trimmedModel = templateModelName.trim();

    const auth = generateThirdPartyAuth(trimmedKey);
    const config = generateThirdPartyConfig(
      templateProviderName || "custom",
      trimmedBaseUrl,
      trimmedModel,
    );

    onApply(
      JSON.stringify(auth, null, 2),
      config,
      {
        websiteUrl: templateWebsiteUrl.trim(),
        displayName: templateDisplayName.trim(),
      }
    );

    resetForm();
    onClose();
  };

  const handleInputKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      e.preventDefault();
      e.stopPropagation();
      applyTemplate();
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) {
          handleClose();
        }
      }}
    >
      <div
        className={`absolute inset-0 bg-black/50 dark:bg-black/70${
          isLinux() ? "" : " backdrop-blur-sm"
        }`}
      />

      <div className="relative mx-4 flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-xl bg-white shadow-lg dark:bg-gray-900">
        <div className="flex h-full min-h-0 flex-col" role="form">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-gray-200 p-6 dark:border-gray-800">
            <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
              {t("codexConfig.quickWizard")}
            </h2>
            <button
              type="button"
              onClick={handleClose}
              className="rounded-md p-1 text-gray-500 transition-colors hover:bg-gray-100 hover:text-gray-900 dark:text-gray-400 dark:hover:bg-gray-800 dark:hover:text-gray-100"
              aria-label={t("common.close")}
            >
              <X size={18} />
            </button>
          </div>

          {/* Content */}
          <div className="flex-1 min-h-0 space-y-4 overflow-auto p-6">
            <div className="rounded-lg border border-blue-200 bg-blue-50 p-3 dark:border-blue-800 dark:bg-blue-900/20">
              <p className="text-sm text-blue-800 dark:text-blue-200">
                {t("codexConfig.wizardHint")}
              </p>
            </div>

            <div className="space-y-4">
              {/* API Key */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.apiKeyLabel")}
                </label>
                <input
                  type="text"
                  value={templateApiKey}
                  ref={apiKeyInputRef}
                  onChange={(e) => setTemplateApiKey(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  pattern=".*\S.*"
                  title={t("common.enterValidValue")}
                  placeholder={t("codexConfig.apiKeyPlaceholder")}
                  required
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm font-mono text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
              </div>

              {/* Display Name */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.supplierNameLabel")}
                </label>
                <input
                  type="text"
                  value={templateDisplayName}
                  ref={displayNameInputRef}
                  onChange={(e) => setTemplateDisplayName(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  placeholder={t("codexConfig.supplierNamePlaceholder")}
                  required
                  pattern=".*\S.*"
                  title={t("common.enterValidValue")}
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {t("codexConfig.supplierNameHint")}
                </p>
              </div>

              {/* Provider Name */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.supplierCodeLabel")}
                </label>
                <input
                  type="text"
                  value={templateProviderName}
                  onChange={(e) => setTemplateProviderName(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  placeholder={t("codexConfig.supplierCodePlaceholder")}
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {t("codexConfig.supplierCodeHint")}
                </p>
              </div>

              {/* Base URL */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.apiUrlLabel")}
                </label>
                <input
                  type="url"
                  value={templateBaseUrl}
                  ref={baseUrlInputRef}
                  onChange={(e) => setTemplateBaseUrl(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  placeholder={t("codexConfig.apiUrlPlaceholder")}
                  required
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
              </div>

              {/* Website URL */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.websiteLabel")}
                </label>
                <input
                  type="url"
                  value={templateWebsiteUrl}
                  onChange={(e) => setTemplateWebsiteUrl(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  placeholder={t("codexConfig.websitePlaceholder")}
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
                <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                  {t("codexConfig.websiteHint")}
                </p>
              </div>

              {/* Model Name */}
              <div>
                <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.modelNameLabel")}
                </label>
                <input
                  type="text"
                  value={templateModelName}
                  ref={modelNameInputRef}
                  onChange={(e) => setTemplateModelName(e.target.value)}
                  onKeyDown={handleInputKeyDown}
                  pattern=".*\S.*"
                  title={t("common.enterValidValue")}
                  placeholder={t("codexConfig.modelNamePlaceholder")}
                  required
                  className="w-full rounded-lg border border-gray-200 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100"
                />
              </div>
            </div>

            {/* Preview */}
            {(templateApiKey || templateProviderName || templateBaseUrl) && (
              <div className="space-y-2 border-t border-gray-200 pt-4 dark:border-gray-700">
                <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
                  {t("codexConfig.configPreview")}
                </h3>
                <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                  <div>
                    <label className="mb-1 block text-xs font-medium text-gray-500 dark:text-gray-400">
                      auth.json
                    </label>
                    <pre className="overflow-x-auto rounded-lg bg-gray-50 p-3 text-xs font-mono text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                      {JSON.stringify(
                        generateThirdPartyAuth(templateApiKey),
                        null,
                        2,
                      )}
                    </pre>
                  </div>
                  <div>
                    <label className="mb-1 block text-xs font-medium text-gray-500 dark:text-gray-400">
                      config.toml
                    </label>
                    <pre className="whitespace-pre-wrap rounded-lg bg-gray-50 p-3 text-xs font-mono text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                      {templateProviderName && templateBaseUrl
                        ? generateThirdPartyConfig(
                            templateProviderName,
                            templateBaseUrl,
                            templateModelName,
                          )
                        : ""}
                    </pre>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-3 border-t border-gray-200 bg-gray-100 p-6 dark:border-gray-800 dark:bg-gray-800">
            <button
              type="button"
              onClick={handleClose}
              className="rounded-lg px-4 py-2 text-sm font-medium text-gray-500 transition-colors hover:bg-white hover:text-gray-900 dark:text-gray-400 dark:hover:bg-gray-700 dark:hover:text-gray-100"
            >
              {t("common.cancel")}
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                applyTemplate();
              }}
              className="flex items-center gap-2 rounded-lg bg-blue-500 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-blue-600 dark:bg-blue-600 dark:hover:bg-blue-700"
            >
              <Save className="h-4 w-4" />
              {t("codexConfig.applyConfig")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
