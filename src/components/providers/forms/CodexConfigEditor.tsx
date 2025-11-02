import React, { useState, useEffect } from "react";
import { CodexAuthSection, CodexConfigSection } from "./CodexConfigSections";
import { CodexQuickWizardModal } from "./CodexQuickWizardModal";
import { CodexCommonConfigModal } from "./CodexCommonConfigModal";

interface CodexConfigEditorProps {
  authValue: string;

  configValue: string;

  onAuthChange: (value: string) => void;

  onConfigChange: (value: string) => void;

  onAuthBlur?: () => void;

  useCommonConfig: boolean;

  onCommonConfigToggle: (checked: boolean) => void;

  commonConfigSnippet: string;

  onCommonConfigSnippetChange: (value: string) => void;

  commonConfigError: string;

  authError: string;

  configError: string; // config.toml 错误提示

  onWebsiteUrlChange?: (url: string) => void; // 更新网址回调

  isTemplateModalOpen?: boolean; // 模态框状态

  setIsTemplateModalOpen?: (open: boolean) => void; // 设置模态框状态

  onNameChange?: (name: string) => void; // 更新供应商名称回调
}

const CodexConfigEditor: React.FC<CodexConfigEditorProps> = ({
  authValue,
  configValue,
  onAuthChange,
  onConfigChange,
  onAuthBlur,
  useCommonConfig,
  onCommonConfigToggle,
  commonConfigSnippet,
  onCommonConfigSnippetChange,
  commonConfigError,
  authError,
  configError,
  onWebsiteUrlChange,
  onNameChange,
  isTemplateModalOpen: externalTemplateModalOpen,
  setIsTemplateModalOpen: externalSetTemplateModalOpen,
}) => {
  const [isCommonConfigModalOpen, setIsCommonConfigModalOpen] = useState(false);

  // Use internal state or external state
  const [internalTemplateModalOpen, setInternalTemplateModalOpen] =
    useState(false);
  const isTemplateModalOpen =
    externalTemplateModalOpen ?? internalTemplateModalOpen;
  const setIsTemplateModalOpen =
    externalSetTemplateModalOpen ?? setInternalTemplateModalOpen;

  // Auto-open common config modal if there's an error
  useEffect(() => {
    if (commonConfigError && !isCommonConfigModalOpen) {
      setIsCommonConfigModalOpen(true);
    }
  }, [commonConfigError, isCommonConfigModalOpen]);

  const handleQuickWizardApply = (
    auth: string,
    config: string,
    extras: { websiteUrl?: string; displayName?: string },
  ) => {
    onAuthChange(auth);
    onConfigChange(config);

    if (onWebsiteUrlChange && extras.websiteUrl) {
      onWebsiteUrlChange(extras.websiteUrl);
    }

    if (onNameChange && extras.displayName) {
      onNameChange(extras.displayName);
    }
  };

  return (
    <div className="space-y-6">
      {/* Auth JSON Section */}
      <CodexAuthSection
        value={authValue}
        onChange={onAuthChange}
        onBlur={onAuthBlur}
        error={authError}
      />

      {/* Config TOML Section */}
      <CodexConfigSection
        value={configValue}
        onChange={onConfigChange}
        useCommonConfig={useCommonConfig}
        onCommonConfigToggle={onCommonConfigToggle}
        onEditCommonConfig={() => setIsCommonConfigModalOpen(true)}
        commonConfigError={commonConfigError}
        configError={configError}
      />

      {/* Quick Wizard Modal */}
      <CodexQuickWizardModal
        isOpen={isTemplateModalOpen}
        onClose={() => setIsTemplateModalOpen(false)}
        onApply={handleQuickWizardApply}
      />

      {/* Common Config Modal */}
      <CodexCommonConfigModal
        isOpen={isCommonConfigModalOpen}
        onClose={() => setIsCommonConfigModalOpen(false)}
        value={commonConfigSnippet}
        onChange={onCommonConfigSnippetChange}
        error={commonConfigError}
      />
    </div>
  );
};

export default CodexConfigEditor;
