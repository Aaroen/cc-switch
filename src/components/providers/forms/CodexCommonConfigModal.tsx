import React, { useEffect } from "react";
import { X, Save } from "lucide-react";
import { useTranslation } from "react-i18next";
import { isLinux } from "@/lib/platform";

interface CodexCommonConfigModalProps {
  isOpen: boolean;
  onClose: () => void;
  value: string;
  onChange: (value: string) => void;
  error?: string;
}

/**
 * CodexCommonConfigModal - Common Codex configuration editor modal
 * Allows editing of common TOML configuration shared across providers
 */
export const CodexCommonConfigModal: React.FC<CodexCommonConfigModalProps> = ({
  isOpen,
  onClose,
  value,
  onChange,
  error,
}) => {
  const { t } = useTranslation();

  // Support ESC key to close modal
  useEffect(() => {
    if (!isOpen) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* Backdrop */}
      <div
        className={`absolute inset-0 bg-black/50 dark:bg-black/70${
          isLinux() ? "" : " backdrop-blur-sm"
        }`}
      />

      {/* Modal */}
      <div className="relative bg-white dark:bg-gray-900 rounded-xl shadow-lg max-w-2xl w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-200 dark:border-gray-800">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">
            {t("codexConfig.editCommonConfigTitle")}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="p-1 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-md transition-colors"
            aria-label={t("common.close")}
          >
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-6 space-y-4">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {t("codexConfig.commonConfigHint")}
          </p>

          <textarea
            value={value}
            onChange={(e) => onChange(e.target.value)}
            placeholder={`# Common Codex config

# Add your common TOML configuration here`}
            rows={12}
            className="w-full px-3 py-2 border border-gray-200 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-100 rounded-lg text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500/20 dark:focus:ring-blue-400/20 focus:border-blue-500 dark:focus:border-blue-400 transition-colors resize-y"
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
            <p className="text-sm text-red-500 dark:text-red-400">{error}</p>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 p-6 border-t border-gray-200 dark:border-gray-800 bg-gray-100 dark:bg-gray-800">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-100 hover:bg-white dark:hover:bg-gray-700 rounded-lg transition-colors"
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 bg-blue-500 dark:bg-blue-600 text-white rounded-lg hover:bg-blue-600 dark:hover:bg-blue-700 transition-colors text-sm font-medium flex items-center gap-2"
          >
            <Save className="w-4 h-4" />
            {t("common.save")}
          </button>
        </div>
      </div>
    </div>
  );
};
