import React, { useEffect, useState } from "react";
import { Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";

interface ClaudeConfigEditorProps {
  value: string;
  onChange: (value: string) => void;
  useCommonConfig: boolean;
  onCommonConfigToggle: (checked: boolean) => void;
  commonConfigSnippet: string;
  onCommonConfigSnippetChange: (value: string) => void;
  commonConfigError: string;
  configError: string;
}

const ClaudeConfigEditor: React.FC<ClaudeConfigEditorProps> = ({
  value,
  onChange,
  useCommonConfig,
  onCommonConfigToggle,
  commonConfigSnippet,
  onCommonConfigSnippetChange,
  commonConfigError,
  configError,
}) => {
  const { t } = useTranslation();
  const [isCommonConfigModalOpen, setIsCommonConfigModalOpen] = useState(false);

  useEffect(() => {
    if (commonConfigError && !isCommonConfigModalOpen) {
      setIsCommonConfigModalOpen(true);
    }
  }, [commonConfigError, isCommonConfigModalOpen]);

  const closeModal = () => {
    setIsCommonConfigModalOpen(false);
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label
          htmlFor="settingsConfig"
          className="block text-sm font-medium text-gray-900 dark:text-gray-100"
        >
          {t("claudeConfig.configLabel")}
        </label>
        <label className="inline-flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400 cursor-pointer">
          <input
            type="checkbox"
            checked={useCommonConfig}
            onChange={(e) => onCommonConfigToggle(e.target.checked)}
            className="w-4 h-4 text-blue-500 bg-white dark:bg-gray-800 border-border-default rounded focus:ring-blue-500 dark:focus:ring-blue-400 focus:ring-2"
          />
          {t("claudeConfig.writeCommonConfig")}
        </label>
      </div>
      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={() => setIsCommonConfigModalOpen(true)}
          className="text-xs text-blue-500 dark:text-blue-400 hover:underline"
        >
          {t("claudeConfig.editCommonConfig")}
        </button>
      </div>
      {commonConfigError && !isCommonConfigModalOpen && (
        <p className="text-xs text-red-500 dark:text-red-400 text-right">
          {commonConfigError}
        </p>
      )}
      <textarea
        id="settingsConfig"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={`{
  "env": {
    "ANTHROPIC_BASE_URL": "https://your-api-endpoint.com",
    "ANTHROPIC_AUTH_TOKEN": "your-api-key-here"
  }
}`}
        rows={12}
        className="w-full px-3 py-2 border border-border-default dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-border-active transition-colors resize-y min-h-[14rem]"
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
        {t("claudeConfig.fullSettingsHint")}
      </p>

      <Dialog
        open={isCommonConfigModalOpen}
        onOpenChange={(open) => !open && closeModal()}
      >
        <DialogContent
          zIndex="nested"
          className="max-w-2xl max-h-[90vh] flex flex-col p-0"
        >
          <DialogHeader className="px-6 pt-6 pb-0">
            <DialogTitle>{t("claudeConfig.editCommonConfigTitle")}</DialogTitle>
          </DialogHeader>

          <div className="flex-1 overflow-auto px-6 py-4 space-y-4">
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {t("claudeConfig.commonConfigHint")}
            </p>
            <textarea
              value={commonConfigSnippet}
              onChange={(e) => onCommonConfigSnippetChange(e.target.value)}
              rows={12}
              className="w-full px-3 py-2 border border-border-default dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-border-active transition-colors resize-y min-h-[14rem]"
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
            {commonConfigError && (
              <p className="text-sm text-red-500 dark:text-red-400">
                {commonConfigError}
              </p>
            )}
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={closeModal}>
              {t("common.cancel")}
            </Button>
            <Button type="button" onClick={closeModal} className="gap-2">
              <Save className="w-4 h-4" />
              {t("common.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
};

export default ClaudeConfigEditor;
