import { type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import {
  Download,
  Sparkles,
  AlignLeft,
  Maximize,
  Edit2,
  Code,
  Box,
  Bug,
  Mail,
  List,
  FileText,
} from "lucide-react";
import {
  MeshPromptClient,
  MeshPromptProviderError,
  builtInPromptActions,
  buildPromptActionRequest,
  getMeshPromptProvider,
  type MeshPromptProviderId,
} from "../lib";

export type View = "text" | "providers" | "actions" | "history" | "settings";
export type OverlayPhase = "picker" | "processing" | "result";

export interface ProviderSettings {
  provider: MeshPromptProviderId;
  model: string;
  baseUrl?: string;
}

export interface SettingsState {
  provider: ProviderSettings;
  shortcut: string;
  closeToTray: boolean;
  runInTray: boolean;
  launchAtStartup: boolean;
  restoreClipboard: boolean;
  historyEnabled: boolean;
  sensitiveMode: boolean;
  timeoutMs: number;
  maxOutputTokens: number;
  temperature: number;
  defaultActionId: string;
  paused: boolean;
  enhancePromptMode: "auto" | "concise" | "structured" | "detailed";
  voiceEngine: "kokoro" | "openai" | "elevenlabs" | "edge";
  voiceId: string;
  brainMode: "api" | "mailbox";
}

export interface HistoryItem {
  id: string;
  actionId: string;
  actionLabel: string;
  provider: MeshPromptProviderId;
  model: string;
  input: string;
  output: string;
  createdAt: string;
}

export interface AppState {
  settings: SettingsState;
  history: HistoryItem[];
  keyStatus: Record<string, boolean>;
}

export interface ConsoleLine {
  type: "sys" | "net" | "err" | "success";
  text: string;
}

export const fallbackSettings: SettingsState = {
  provider: { provider: "groq", model: "llama-3.1-8b-instant" },
  shortcut: "Ctrl+Shift+Space",
  closeToTray: true,
  runInTray: true,
  launchAtStartup: true,
  restoreClipboard: true,
  historyEnabled: true,
  sensitiveMode: false,
  timeoutMs: 60_000,
  maxOutputTokens: 1_800,
  temperature: 0.35,
  defaultActionId: "enhance-prompt",
  paused: false,
  enhancePromptMode: "auto",
  voiceEngine: "kokoro",
  voiceId: "kokoro-default",
  brainMode: "api",
};

export function ActionIcon({ actionId }: { actionId: string }) {
  switch (actionId) {
    case "enhance-prompt": return <Sparkles size={14} />;
    case "make-concise": return <AlignLeft size={14} />;
    case "expand-details": return <Maximize size={14} />;
    case "rewrite-professionally": return <Edit2 size={14} />;
    case "developer-prompt": return <Code size={14} />;
    case "product-prompt": return <Box size={14} />;
    case "bug-report": return <Bug size={14} />;
    case "email-rewrite": return <Mail size={14} />;
    case "summarize": return <List size={14} />;
    default: return <FileText size={14} />;
  }
}

export function TitleBar({
  status,
  view,
  updateInfo,
  onUpdateClick,
}: {
  status: string;
  view: View;
  updateInfo: { updateAvailable: boolean; version: string } | null;
  onUpdateClick: () => void;
}) {
  const appWindow = getCurrentWindow();
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="title-actions-left" data-tauri-drag-region>
        {status && <div className="status-pill" data-no-drag>{status}</div>}
        {updateInfo?.updateAvailable && (
          <button 
            className="titlebar-update-btn" 
            data-no-drag 
            onClick={onUpdateClick}
            style={{ marginLeft: '8px' }}
            title={`Update Available (${updateInfo.version})`}
            aria-label="Update Available"
          >
            <Download size={11} />
            <span>Update</span>
          </button>
        )}
      </div>
      <div className="title-center" data-tauri-drag-region>
        <span className="title-icon" data-tauri-drag-region>
          <img src="/ole-badge.svg" alt="" data-tauri-drag-region />
        </span>
        <strong className="title-text" data-tauri-drag-region>Olé</strong>
      </div>
      <div className="window-controls-right" data-tauri-drag-region>
        <button className="mac-dot minimize" data-no-drag onClick={(e) => { e.stopPropagation(); void appWindow.minimize(); }} aria-label="Minimize"></button>
        <button className="mac-dot maximize" data-no-drag onClick={(e) => { e.stopPropagation(); void appWindow.toggleMaximize(); }} aria-label="Maximize"></button>
        <button className="mac-dot close" data-no-drag onClick={(e) => { e.stopPropagation(); void appWindow.close(); }} aria-label="Close"></button>
      </div>
    </header>
  );
}

export function NavButton({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: ReactNode; label: string }) {
  return <button className={active ? "nav-button active" : "nav-button"} onClick={onClick}>{icon}<span>{label}</span></button>;
}

export function SettingsCard({ title, icon, children }: { title: string; icon: ReactNode; children: ReactNode }) {
  return (
    <div className="card stack premium-glass-card">
      <div className="settings-title">
        {icon}
        <h3>{title}</h3>
      </div>
      {children}
    </div>
  );
}

export function Toggle({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void | Promise<void> }) {
  return (
    <label 
      className="toggle-row"
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        flexDirection: 'row',
        width: '100%',
        padding: '12px 0',
        borderBottom: '1px solid var(--border)',
        cursor: 'pointer'
      }}
    >
      <span style={{ fontSize: '13px', fontWeight: 500, color: 'var(--text-secondary)' }}>{label}</span>
      <div className="toggle-switch" style={{ flexShrink: 0 }}>
        <input type="checkbox" checked={checked} onChange={(event) => void onChange(event.target.checked)} />
        <span className="toggle-slider"></span>
      </div>
    </label>
  );
}

export function EmptyState({ title, body }: { title: string; body: string }) {
  return <div className="empty-state"><strong>{title}</strong><span>{body}</span></div>;
}

export async function generateWithCurrentProvider(
  settings: SettingsState,
  actionId: string,
  selectedText: string,
  userInstruction = "",
) {
  const providerDef = getMeshPromptProvider(settings.provider.provider);
  const key = providerDef.authMode === "api-key"
    ? await invoke<string | null>("get_provider_key", { provider: providerDef.id })
    : null;

  if (providerDef.authMode === "api-key" && !key) {
    throw new Error(`${providerDef.label} API key is not configured. Add it in AI Providers.`);
  }

  const model = settings.provider.model || providerDef.defaultModel;

  if (!selectedText.trim()) {
    throw new Error("Add text to enhance first.");
  }

  const action = builtInPromptActions.find((item) => item.id === actionId) ?? builtInPromptActions[0];
  const request = buildPromptActionRequest(action, { selectedText, userInstruction, settings });
  const client = new MeshPromptClient({
    provider: providerDef,
    credentials: { apiKey: key ?? undefined, baseUrl: settings.provider.baseUrl },
    timeoutMs: settings.timeoutMs,
    appName: "Olé",
  });
  return client.generate({
    ...request.options,
    model,
    temperature: settings.temperature,
    maxOutputTokens: settings.maxOutputTokens,
    messages: request.messages,
  });
}

export function errorMessage(error: unknown): string {
  if (error instanceof MeshPromptProviderError) {
    return error.message;
  }
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return "Olé action failed.";
}
