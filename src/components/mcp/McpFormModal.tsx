import React, { useMemo, useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  Save,
  Plus,
  AlertCircle,
  ChevronDown,
  ChevronUp,
  AlertTriangle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { mcpApi, type AppId } from "@/lib/api";
import { McpServer, McpServerSpec } from "@/types";
import { mcpPresets, getMcpPresetWithDescription } from "@/config/mcpPresets";
import McpWizardModal from "./McpWizardModal";
import {
  extractErrorMessage,
  translateMcpBackendError,
} from "@/utils/errorUtils";
import {
  tomlToMcpServer,
  extractIdFromToml,
  mcpServerToToml,
} from "@/utils/tomlUtils";
import { normalizeTomlText } from "@/utils/textNormalization";
import { useMcpValidation } from "./useMcpValidation";

interface McpFormModalProps {
  appId: AppId;
  editingId?: string;
  initialData?: McpServer;
  onSave: (
    id: string,
    server: McpServer,
    options?: { syncOtherSide?: boolean },
  ) => Promise<void>;
  onClose: () => void;
  existingIds?: string[];
}

/**
 * MCP 表单模态框组件（简化版）
 * Claude: 使用 JSON 格式
 * Codex: 使用 TOML 格式
 */
const McpFormModal: React.FC<McpFormModalProps> = ({
  appId,
  editingId,
  initialData,
  onSave,
  onClose,
  existingIds = [],
}) => {
  const { t } = useTranslation();
  const { formatTomlError, validateTomlConfig, validateJsonConfig } =
    useMcpValidation();

  const [formId, setFormId] = useState(
    () => editingId || initialData?.id || "",
  );
  const [formName, setFormName] = useState(initialData?.name || "");
  const [formDescription, setFormDescription] = useState(
    initialData?.description || "",
  );
  const [formHomepage, setFormHomepage] = useState(initialData?.homepage || "");
  const [formDocs, setFormDocs] = useState(initialData?.docs || "");
  const [formTags, setFormTags] = useState(initialData?.tags?.join(", ") || "");

  // 编辑模式下禁止修改 ID
  const isEditing = !!editingId;

  // 判断是否在编辑模式下有附加信息
  const hasAdditionalInfo = !!(
    initialData?.description ||
    initialData?.tags?.length ||
    initialData?.homepage ||
    initialData?.docs
  );

  // 附加信息展开状态（编辑模式下有值时默认展开）
  const [showMetadata, setShowMetadata] = useState(
    isEditing ? hasAdditionalInfo : false,
  );

  // 根据 appId 决定初始格式
  const [formConfig, setFormConfig] = useState(() => {
    const spec = initialData?.server;
    if (!spec) return "";
    if (appId === "codex") {
      return mcpServerToToml(spec);
    }
    return JSON.stringify(spec, null, 2);
  });

  const [configError, setConfigError] = useState("");
  const [saving, setSaving] = useState(false);
  const [isWizardOpen, setIsWizardOpen] = useState(false);
  const [idError, setIdError] = useState("");
  const [syncOtherSide, setSyncOtherSide] = useState(false);
  const [otherSideHasConflict, setOtherSideHasConflict] = useState(false);

  // 判断是否使用 TOML 格式
  const useToml = appId === "codex";
  const syncTargetLabel =
    appId === "claude" ? t("apps.codex") : t("apps.claude");
  const otherAppType: AppId = appId === "claude" ? "codex" : "claude";
  const syncCheckboxId = useMemo(() => `sync-other-side-${appId}`, [appId]);

  // 检测另一侧是否有同名 MCP
  useEffect(() => {
    const checkOtherSide = async () => {
      const currentId = formId.trim();
      if (!currentId) {
        setOtherSideHasConflict(false);
        return;
      }

      try {
        const otherConfig = await mcpApi.getConfig(otherAppType);
        const hasConflict = Object.keys(otherConfig.servers || {}).includes(
          currentId,
        );
        setOtherSideHasConflict(hasConflict);
      } catch (error) {
        console.error("检查另一侧 MCP 配置失败:", error);
        setOtherSideHasConflict(false);
      }
    };

    checkOtherSide();
  }, [formId, otherAppType]);

  const wizardInitialSpec = useMemo(() => {
    const fallback = initialData?.server;
    if (!formConfig.trim()) {
      return fallback;
    }

    if (useToml) {
      try {
        return tomlToMcpServer(formConfig);
      } catch {
        return fallback;
      }
    }

    try {
      const parsed = JSON.parse(formConfig);
      if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
        return parsed as McpServerSpec;
      }
      return fallback;
    } catch {
      return fallback;
    }
  }, [formConfig, initialData, useToml]);

  // 预设选择状态（仅新增模式显示；-1 表示自定义）
  const [selectedPreset, setSelectedPreset] = useState<number | null>(
    isEditing ? null : -1,
  );

  const handleIdChange = (value: string) => {
    setFormId(value);
    if (!isEditing) {
      const exists = existingIds.includes(value.trim());
      setIdError(exists ? t("mcp.error.idExists") : "");
    }
  };

  const ensureUniqueId = (base: string): string => {
    let candidate = base.trim();
    if (!candidate) candidate = "mcp-server";
    if (!existingIds.includes(candidate)) return candidate;
    let i = 1;
    while (existingIds.includes(`${candidate}-${i}`)) i++;
    return `${candidate}-${i}`;
  };

  // 应用预设（写入表单但不落库）
  const applyPreset = (index: number) => {
    if (index < 0 || index >= mcpPresets.length) return;
    const preset = mcpPresets[index];
    const presetWithDesc = getMcpPresetWithDescription(preset, t);

    const id = ensureUniqueId(presetWithDesc.id);
    setFormId(id);
    setFormName(presetWithDesc.name || presetWithDesc.id);
    setFormDescription(presetWithDesc.description || "");
    setFormHomepage(presetWithDesc.homepage || "");
    setFormDocs(presetWithDesc.docs || "");
    setFormTags(presetWithDesc.tags?.join(", ") || "");

    // 根据格式转换配置
    if (useToml) {
      const toml = mcpServerToToml(presetWithDesc.server);
      setFormConfig(toml);
      setConfigError(validateTomlConfig(toml));
    } else {
      const json = JSON.stringify(presetWithDesc.server, null, 2);
      setFormConfig(json);
      setConfigError(validateJsonConfig(json));
    }
    setSelectedPreset(index);
  };

  // 切回自定义
  const applyCustom = () => {
    setSelectedPreset(-1);
    // 恢复到空白模板
    setFormId("");
    setFormName("");
    setFormDescription("");
    setFormHomepage("");
    setFormDocs("");
    setFormTags("");
    setFormConfig("");
    setConfigError("");
  };

  const handleConfigChange = (value: string) => {
    // 若为 TOML 模式，先做引号归一化，避免中文输入法导致的格式错误
    const nextValue = useToml ? normalizeTomlText(value) : value;
    setFormConfig(nextValue);

    if (useToml) {
      // TOML validation (use hook's complete validation)
      const err = validateTomlConfig(nextValue);
      if (err) {
        setConfigError(err);
        return;
      }

      // Try to extract ID (if user hasn't filled it yet)
      if (nextValue.trim() && !formId.trim()) {
        const extractedId = extractIdFromToml(nextValue);
        if (extractedId) {
          setFormId(extractedId);
        }
      }
    } else {
      // JSON validation (use hook's complete validation)
      const err = validateJsonConfig(value);
      if (err) {
        setConfigError(err);
        return;
      }
    }

    setConfigError("");
  };

  const handleWizardApply = (title: string, json: string) => {
    setFormId(title);
    if (!formName.trim()) {
      setFormName(title);
    }
    // Wizard returns JSON, convert based on format if needed
    if (useToml) {
      try {
        const server = JSON.parse(json) as McpServerSpec;
        const toml = mcpServerToToml(server);
        setFormConfig(toml);
        setConfigError(validateTomlConfig(toml));
      } catch (e: any) {
        setConfigError(t("mcp.error.jsonInvalid"));
      }
    } else {
      setFormConfig(json);
      setConfigError(validateJsonConfig(json));
    }
  };

  const handleSubmit = async () => {
    const trimmedId = formId.trim();
    if (!trimmedId) {
      toast.error(t("mcp.error.idRequired"), { duration: 3000 });
      return;
    }

    // 新增模式：阻止提交重名 ID
    if (!isEditing && existingIds.includes(trimmedId)) {
      setIdError(t("mcp.error.idExists"));
      return;
    }

    // Validate configuration format
    let serverSpec: McpServerSpec;

    if (useToml) {
      // TOML mode
      const tomlError = validateTomlConfig(formConfig);
      setConfigError(tomlError);
      if (tomlError) {
        toast.error(t("mcp.error.tomlInvalid"), { duration: 3000 });
        return;
      }

      if (!formConfig.trim()) {
        // Empty configuration
        serverSpec = {
          type: "stdio",
          command: "",
          args: [],
        };
      } else {
        try {
          serverSpec = tomlToMcpServer(formConfig);
        } catch (e: any) {
          const msg = e?.message || String(e);
          setConfigError(formatTomlError(msg));
          toast.error(t("mcp.error.tomlInvalid"), { duration: 4000 });
          return;
        }
      }
    } else {
      // JSON mode
      const jsonError = validateJsonConfig(formConfig);
      setConfigError(jsonError);
      if (jsonError) {
        toast.error(t("mcp.error.jsonInvalid"), { duration: 3000 });
        return;
      }

      if (!formConfig.trim()) {
        // Empty configuration
        serverSpec = {
          type: "stdio",
          command: "",
          args: [],
        };
      } else {
        try {
          serverSpec = JSON.parse(formConfig) as McpServerSpec;
        } catch (e: any) {
          setConfigError(t("mcp.error.jsonInvalid"));
          toast.error(t("mcp.error.jsonInvalid"), { duration: 4000 });
          return;
        }
      }
    }

    // 前置必填校验
    if (serverSpec?.type === "stdio" && !serverSpec?.command?.trim()) {
      toast.error(t("mcp.error.commandRequired"), { duration: 3000 });
      return;
    }
    if (serverSpec?.type === "http" && !serverSpec?.url?.trim()) {
      toast.error(t("mcp.wizard.urlRequired"), { duration: 3000 });
      return;
    }

    setSaving(true);
    try {
      const entry: McpServer = {
        ...(initialData ? { ...initialData } : {}),
        id: trimmedId,
        server: serverSpec,
      };

      // 修复：新增 MCP 时默认启用（enabled=true）
      // 编辑模式下保留原有的 enabled 状态
      if (initialData?.enabled !== undefined) {
        entry.enabled = initialData.enabled;
      } else {
        // 新增模式：默认启用
        entry.enabled = true;
      }

      const nameTrimmed = (formName || trimmedId).trim();
      entry.name = nameTrimmed || trimmedId;

      const descriptionTrimmed = formDescription.trim();
      if (descriptionTrimmed) {
        entry.description = descriptionTrimmed;
      } else {
        delete entry.description;
      }

      const homepageTrimmed = formHomepage.trim();
      if (homepageTrimmed) {
        entry.homepage = homepageTrimmed;
      } else {
        delete entry.homepage;
      }

      const docsTrimmed = formDocs.trim();
      if (docsTrimmed) {
        entry.docs = docsTrimmed;
      } else {
        delete entry.docs;
      }

      const parsedTags = formTags
        .split(",")
        .map((tag) => tag.trim())
        .filter((tag) => tag.length > 0);
      if (parsedTags.length > 0) {
        entry.tags = parsedTags;
      } else {
        delete entry.tags;
      }

      // 显式等待父组件保存流程
      await onSave(trimmedId, entry, { syncOtherSide });
    } catch (error: any) {
      const detail = extractErrorMessage(error);
      const mapped = translateMcpBackendError(detail, t);
      const msg = mapped || detail || t("mcp.error.saveFailed");
      toast.error(msg, { duration: mapped || detail ? 6000 : 4000 });
    } finally {
      setSaving(false);
    }
  };

  const getFormTitle = () => {
    if (appId === "claude") {
      return isEditing ? t("mcp.editClaudeServer") : t("mcp.addClaudeServer");
    } else {
      return isEditing ? t("mcp.editCodexServer") : t("mcp.addCodexServer");
    }
  };

  return (
    <>
      <Dialog open={true} onOpenChange={(open) => !open && onClose()}>
        <DialogContent className="max-w-3xl max-h-[90vh] flex flex-col">
          <DialogHeader>
            <DialogTitle>{getFormTitle()}</DialogTitle>
          </DialogHeader>

          {/* Content - Scrollable */}
          <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
            {/* 预设选择（仅新增时展示） */}
            {!isEditing && (
              <div>
                <label className="block text-sm font-medium text-gray-900 dark:text-gray-100 mb-3">
                  {t("mcp.presets.title")}
                </label>
                <div className="flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={applyCustom}
                    className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                      selectedPreset === -1
                        ? "bg-emerald-500 text-white dark:bg-emerald-600"
                        : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                    }`}
                  >
                    {t("presetSelector.custom")}
                  </button>
                  {mcpPresets.map((preset, idx) => {
                    const descriptionKey = `mcp.presets.${preset.id}.description`;
                    return (
                      <button
                        key={preset.id}
                        type="button"
                        onClick={() => applyPreset(idx)}
                        className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                          selectedPreset === idx
                            ? "bg-emerald-500 text-white dark:bg-emerald-600"
                            : "bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-700"
                        }`}
                        title={t(descriptionKey)}
                      >
                        {preset.id}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
            {/* ID (标题) */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  {t("mcp.form.title")} <span className="text-red-500">*</span>
                </label>
                {!isEditing && idError && (
                  <span className="text-xs text-red-500 dark:text-red-400">
                    {idError}
                  </span>
                )}
              </div>
              <Input
                type="text"
                placeholder={t("mcp.form.titlePlaceholder")}
                value={formId}
                onChange={(e) => handleIdChange(e.target.value)}
                disabled={isEditing}
              />
            </div>

            {/* Name */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                {t("mcp.form.name")}
              </label>
              <Input
                type="text"
                placeholder={t("mcp.form.namePlaceholder")}
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
              />
            </div>

            {/* 可折叠的附加信息按钮 */}
            <div>
              <button
                type="button"
                onClick={() => setShowMetadata(!showMetadata)}
                className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-gray-100 transition-colors"
              >
                {showMetadata ? (
                  <ChevronUp size={16} />
                ) : (
                  <ChevronDown size={16} />
                )}
                {t("mcp.form.additionalInfo")}
              </button>
            </div>

            {/* 附加信息区域（可折叠） */}
            {showMetadata && (
              <>
                {/* Description (描述) */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    {t("mcp.form.description")}
                  </label>
                  <Input
                    type="text"
                    placeholder={t("mcp.form.descriptionPlaceholder")}
                    value={formDescription}
                    onChange={(e) => setFormDescription(e.target.value)}
                  />
                </div>

                {/* Tags */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    {t("mcp.form.tags")}
                  </label>
                  <Input
                    type="text"
                    placeholder={t("mcp.form.tagsPlaceholder")}
                    value={formTags}
                    onChange={(e) => setFormTags(e.target.value)}
                  />
                </div>

                {/* Homepage */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    {t("mcp.form.homepage")}
                  </label>
                  <Input
                    type="text"
                    placeholder={t("mcp.form.homepagePlaceholder")}
                    value={formHomepage}
                    onChange={(e) => setFormHomepage(e.target.value)}
                  />
                </div>

                {/* Docs */}
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    {t("mcp.form.docs")}
                  </label>
                  <Input
                    type="text"
                    placeholder={t("mcp.form.docsPlaceholder")}
                    value={formDocs}
                    onChange={(e) => setFormDocs(e.target.value)}
                  />
                </div>
              </>
            )}

            {/* 配置输入框（根据格式显示 JSON 或 TOML） */}
            <div>
              <div className="flex items-center justify-between mb-2">
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  {useToml
                    ? t("mcp.form.tomlConfig")
                    : t("mcp.form.jsonConfig")}
                </label>
                {(isEditing || selectedPreset === -1) && (
                  <button
                    type="button"
                    onClick={() => setIsWizardOpen(true)}
                    className="text-sm text-blue-500 dark:text-blue-400 hover:text-blue-600 dark:hover:text-blue-300 transition-colors"
                  >
                    {t("mcp.form.useWizard")}
                  </button>
                )}
              </div>
              <Textarea
                className="h-48 resize-none font-mono text-xs"
                placeholder={
                  useToml
                    ? t("mcp.form.tomlPlaceholder")
                    : t("mcp.form.jsonPlaceholder")
                }
                value={formConfig}
                onChange={(e) => handleConfigChange(e.target.value)}
              />
              {configError && (
                <div className="flex items-center gap-2 mt-2 text-red-500 dark:text-red-400 text-sm">
                  <AlertCircle size={16} />
                  <span>{configError}</span>
                </div>
              )}
            </div>
          </div>

          {/* Footer */}
          <DialogFooter className="flex-col sm:flex-row sm:justify-between gap-3 pt-4">
            {/* 双端同步选项 */}
            <div className="flex items-center gap-3">
              <div className="flex items-center gap-2">
                <input
                  id={syncCheckboxId}
                  type="checkbox"
                  className="h-4 w-4 rounded border-border-default text-emerald-600 focus:ring-emerald-500  dark:bg-gray-800"
                  checked={syncOtherSide}
                  onChange={(event) => setSyncOtherSide(event.target.checked)}
                />
                <label
                  htmlFor={syncCheckboxId}
                  className="text-sm text-gray-700 dark:text-gray-300 cursor-pointer select-none"
                  title={t("mcp.form.syncOtherSideHint", {
                    target: syncTargetLabel,
                  })}
                >
                  {t("mcp.form.syncOtherSide", { target: syncTargetLabel })}
                </label>
              </div>
              {syncOtherSide && otherSideHasConflict && (
                <div className="flex items-center gap-1.5 text-amber-600 dark:text-amber-400">
                  <AlertTriangle size={14} />
                  <span className="text-xs font-medium">
                    {t("mcp.form.willOverwriteWarning", {
                      target: syncTargetLabel,
                    })}
                  </span>
                </div>
              )}
            </div>

            {/* 操作按钮 */}
            <div className="flex items-center gap-3">
              <Button type="button" variant="ghost" onClick={onClose}>
                {t("common.cancel")}
              </Button>
              <Button
                type="button"
                onClick={handleSubmit}
                disabled={saving || (!isEditing && !!idError)}
                variant="mcp"
              >
                {isEditing ? <Save size={16} /> : <Plus size={16} />}
                {saving
                  ? t("common.saving")
                  : isEditing
                    ? t("common.save")
                    : t("common.add")}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Wizard Modal */}
      <McpWizardModal
        isOpen={isWizardOpen}
        onClose={() => setIsWizardOpen(false)}
        onApply={handleWizardApply}
        initialTitle={formId}
        initialServer={wizardInitialSpec}
      />
    </>
  );
};

export default McpFormModal;
