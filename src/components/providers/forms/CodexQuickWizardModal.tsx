import React, { useState, useRef } from "react";
import { Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  generateThirdPartyAuth,
  generateThirdPartyConfig,
} from "@/config/codexProviderPresets";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface CodexQuickWizardModalProps {
  isOpen: boolean;
  onClose: () => void;
  onApply: (
    auth: string,
    config: string,
    extras: {
      websiteUrl?: string;
      displayName?: string;
    },
  ) => void;
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

    onApply(JSON.stringify(auth, null, 2), config, {
      websiteUrl: templateWebsiteUrl.trim(),
      displayName: templateDisplayName.trim(),
    });

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

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && handleClose()}>
      <DialogContent
        zIndex="nested"
        className="max-w-2xl max-h-[90vh] flex flex-col p-0"
      >
        <DialogHeader className="px-6 pt-6 pb-0">
          <DialogTitle>{t("codexConfig.quickWizard")}</DialogTitle>
        </DialogHeader>

        <div className="flex-1 min-h-0 space-y-4 overflow-auto px-6 py-4">
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
              <Input
                type="text"
                value={templateApiKey}
                ref={apiKeyInputRef}
                onChange={(e) => setTemplateApiKey(e.target.value)}
                onKeyDown={handleInputKeyDown}
                pattern=".*\S.*"
                title={t("common.enterValidValue")}
                placeholder={t("codexConfig.apiKeyPlaceholder")}
                required
                className="font-mono"
              />
            </div>

            {/* Display Name */}
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                {t("codexConfig.supplierNameLabel")}
              </label>
              <Input
                type="text"
                value={templateDisplayName}
                ref={displayNameInputRef}
                onChange={(e) => setTemplateDisplayName(e.target.value)}
                onKeyDown={handleInputKeyDown}
                placeholder={t("codexConfig.supplierNamePlaceholder")}
                required
                pattern=".*\S.*"
                title={t("common.enterValidValue")}
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
              <Input
                type="text"
                value={templateProviderName}
                onChange={(e) => setTemplateProviderName(e.target.value)}
                onKeyDown={handleInputKeyDown}
                placeholder={t("codexConfig.supplierCodePlaceholder")}
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
              <Input
                type="url"
                value={templateBaseUrl}
                ref={baseUrlInputRef}
                onChange={(e) => setTemplateBaseUrl(e.target.value)}
                onKeyDown={handleInputKeyDown}
                placeholder={t("codexConfig.apiUrlPlaceholder")}
                required
                className="font-mono"
              />
            </div>

            {/* Website URL */}
            <div>
              <label className="mb-1 block text-sm font-medium text-gray-900 dark:text-gray-100">
                {t("codexConfig.websiteLabel")}
              </label>
              <Input
                type="url"
                value={templateWebsiteUrl}
                onChange={(e) => setTemplateWebsiteUrl(e.target.value)}
                onKeyDown={handleInputKeyDown}
                placeholder={t("codexConfig.websitePlaceholder")}
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
              <Input
                type="text"
                value={templateModelName}
                ref={modelNameInputRef}
                onChange={(e) => setTemplateModelName(e.target.value)}
                onKeyDown={handleInputKeyDown}
                pattern=".*\S.*"
                title={t("common.enterValidValue")}
                placeholder={t("codexConfig.modelNamePlaceholder")}
                required
              />
            </div>
          </div>

          {/* Preview */}
          {(templateApiKey || templateProviderName || templateBaseUrl) && (
            <div className="space-y-2 border-t border-border-default pt-4 ">
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

        <DialogFooter>
          <Button type="button" variant="outline" onClick={handleClose}>
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              applyTemplate();
            }}
            className="gap-2"
          >
            <Save className="h-4 w-4" />
            {t("codexConfig.applyConfig")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
