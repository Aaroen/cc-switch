import { useMemo } from "react";
import { FolderSearch, Undo2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { useTranslation } from "react-i18next";
import type { AppType } from "@/lib/api";
import type { ResolvedDirectories } from "@/hooks/useSettings";

interface DirectorySettingsProps {
  appConfigDir?: string;
  resolvedDirs: ResolvedDirectories;
  onAppConfigChange: (value?: string) => void;
  onBrowseAppConfig: () => Promise<void>;
  onResetAppConfig: () => Promise<void>;
  claudeDir?: string;
  codexDir?: string;
  onDirectoryChange: (app: AppType, value?: string) => void;
  onBrowseDirectory: (app: AppType) => Promise<void>;
  onResetDirectory: (app: AppType) => Promise<void>;
}

export function DirectorySettings({
  appConfigDir,
  resolvedDirs,
  onAppConfigChange,
  onBrowseAppConfig,
  onResetAppConfig,
  claudeDir,
  codexDir,
  onDirectoryChange,
  onBrowseDirectory,
  onResetDirectory,
}: DirectorySettingsProps) {
  const { t } = useTranslation();

  return (
    <section className="space-y-4">
      <header className="space-y-1">
        <h3 className="text-sm font-medium">
          {t("settings.configDirectoryOverride")}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.configDirectoryDescription")}
        </p>
      </header>

      <DirectoryInput
        label={t("settings.appConfigDir")}
        description={t("settings.appConfigDirDescription")}
        value={appConfigDir}
        resolvedValue={resolvedDirs.appConfig}
        placeholder={t("settings.browsePlaceholderApp")}
        onChange={onAppConfigChange}
        onBrowse={onBrowseAppConfig}
        onReset={onResetAppConfig}
      />

      <DirectoryInput
        label={t("settings.claudeConfigDir")}
        description={t("settings.claudeConfigDirDescription", {
          defaultValue: "覆盖 Claude 配置目录 (settings.json)。",
        })}
        value={claudeDir}
        resolvedValue={resolvedDirs.claude}
        placeholder={t("settings.browsePlaceholderClaude")}
        onChange={(val) => onDirectoryChange("claude", val)}
        onBrowse={() => onBrowseDirectory("claude")}
        onReset={() => onResetDirectory("claude")}
      />

      <DirectoryInput
        label={t("settings.codexConfigDir")}
        description={t("settings.codexConfigDirDescription", {
          defaultValue: "覆盖 Codex 配置目录。",
        })}
        value={codexDir}
        resolvedValue={resolvedDirs.codex}
        placeholder={t("settings.browsePlaceholderCodex")}
        onChange={(val) => onDirectoryChange("codex", val)}
        onBrowse={() => onBrowseDirectory("codex")}
        onReset={() => onResetDirectory("codex")}
      />
    </section>
  );
}

interface DirectoryInputProps {
  label: string;
  description?: string;
  value?: string;
  resolvedValue: string;
  placeholder?: string;
  onChange: (value?: string) => void;
  onBrowse: () => Promise<void>;
  onReset: () => Promise<void>;
}

function DirectoryInput({
  label,
  description,
  value,
  resolvedValue,
  placeholder,
  onChange,
  onBrowse,
  onReset,
}: DirectoryInputProps) {
  const { t } = useTranslation();
  const displayValue = useMemo(() => value ?? resolvedValue ?? "", [value, resolvedValue]);

  return (
    <div className="space-y-1.5">
      <div className="space-y-1">
        <p className="text-xs font-medium text-foreground">{label}</p>
        {description ? (
          <p className="text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <div className="flex items-center gap-2">
        <Input
          value={displayValue}
          placeholder={placeholder}
          className="font-mono text-xs"
          onChange={(event) => onChange(event.target.value)}
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={onBrowse}
          title={t("settings.browseDirectory")}
        >
          <FolderSearch className="h-4 w-4" />
        </Button>
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={onReset}
          title={t("settings.resetDefault")}
        >
          <Undo2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
