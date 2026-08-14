import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  Copy,
  History,
  KeyRound,
  RotateCcw,
  Search,
  Settings,
  Sparkles,
  Wand2,
  X,
} from "lucide-react";
import {
  builtInPromptActions,
  getMeshPromptProvider,
} from "../lib";
import {
  fallbackSettings,
  generateWithCurrentProvider,
  errorMessage,
  TitleBar,
  NavButton,
  ActionIcon,
  type View,
  type OverlayPhase,
  type SettingsState,
  type AppState,
} from "./PromptCommon";

import { ProviderView } from "./ProviderView";
import { ActionsView } from "./ActionsView";
import { HistoryView } from "./HistoryView";
import { SettingsView } from "./SettingsView";

export default function App() {
  return <MainApp />;
}

export function MainApp({
  embed = false,
  activeView,
  hideSidebar = false,
}: {
  embed?: boolean;
  activeView?: "text" | "providers" | "actions" | "history" | "settings";
  hideSidebar?: boolean;
}) {
  const [state, setState] = useState<AppState>({ settings: fallbackSettings, history: [], keyStatus: {} });
  const [view, setView] = useState<View>("text");

  useEffect(() => {
    if (activeView) {
      setView(activeView);
    }
  }, [activeView]);

  const [selectedText, setSelectedText] = useState("");
  const [output, setOutput] = useState("");
  const [actionId, setActionId] = useState(fallbackSettings.defaultActionId);
  const [instruction, setInstruction] = useState("");
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [status, setStatus] = useState("Ready");
  const [busy, setBusy] = useState(false);

  const [updateInfo, setUpdateInfo] = useState<{
    updateAvailable: boolean;
    version: string;
    changelog: string;
    downloadUrl: string;
  } | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [updating, setUpdating] = useState(false);
  const [updateError, setUpdateError] = useState("");

  useEffect(() => {
    void refreshState();
    void checkUpdates();
    const captureError = listen<string>("meshprompt://capture-error", (event) => {
      setStatus(event.payload ?? "No selected text found.");
      setView("text");
    });
    const openView = listen<View>("meshprompt://open-view", (event) => {
      if (event.payload) {
        setView(event.payload);
      }
    });
    return () => {
      void captureError.then((off) => off());
      void openView.then((off) => off());
    };
  }, []);

  async function checkUpdates() {
    try {
      const result = await invoke<any>("check_for_updates");
      if (result && result.updateAvailable) {
        setUpdateInfo(result);
      }
    } catch (e) {
      console.warn("Failed to check for updates:", e);
    }
  }

  async function triggerUpdate() {
    if (!updateInfo) return;
    setUpdating(true);
    setUpdateError("");
    try {
      await invoke("install_update", { downloadUrl: updateInfo.downloadUrl });
    } catch (e) {
      setUpdating(false);
      setUpdateError(errorMessage(e));
    }
  }

  useEffect(() => {
    if (view === "history") {
      void refreshState();
    }
  }, [view]);

  async function refreshState() {
    try {
      const next = await invoke<AppState>("get_app_state");
      setState(next);
      setActionId(next.settings.defaultActionId);
    } catch (error) {
      setStatus(errorMessage(error));
    }
  }

  async function saveSettings(patch: Partial<SettingsState>) {
    const settings = { ...state.settings, ...patch };
    setState((current) => ({ ...current, settings }));
    await invoke("save_settings", { settings });
    setStatus("Settings saved");
  }

  async function runAction(sourceText = selectedText, sourceInstruction = instruction) {
    if (!sourceText.trim()) {
      setStatus("Add text to enhance first.");
      return;
    }
    const action = builtInPromptActions.find((item) => item.id === actionId) ?? builtInPromptActions[0];
    setBusy(true);
    setStatus("Enhancing text...");
    try {
      const response = await generateWithCurrentProvider(state.settings, action.id, sourceText, sourceInstruction);
      setOutput(response.content);
      setStatus("Output ready");
      if (state.settings.historyEnabled && !state.settings.sensitiveMode) {
        await invoke("add_history", {
          item: {
            id: crypto.randomUUID(),
            actionId: action.id,
            actionLabel: action.label,
            provider: response.provider,
            model: response.model,
            input: sourceText,
            output: response.content,
            createdAt: new Date().toISOString(),
          },
        });
        await refreshState();
      }
    } catch (error) {
      setStatus(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  async function captureText() {
    setStatus("Capturing selected text...");
    try {
      const text = await invoke<string>("capture_selected_text");
      setSelectedText(text);
      setStatus("Text captured");
    } catch (error) {
      setStatus(errorMessage(error));
    }
  }

  async function copyOutput() {
    await invoke("copy_text", { text: output });
    setStatus("Copied");
  }

  async function replaceOutput() {
    try {
      await invoke("replace_selected_text", { text: output });
      setStatus("Replaced selected text");
    } catch (error) {
      await invoke("copy_text", { text: output });
      setStatus(`Replace failed. Copied instead. ${errorMessage(error)}`);
    }
  }

  async function saveKey() {
    const providerId = state.settings.provider.provider;
    const providerDef = getMeshPromptProvider(providerId);
    const label = providerDef.label;

    const trimmedKey = apiKeyDraft.trim().replace(/[\r\n]+/g, "");
    if (!trimmedKey) {
      setStatus(`Add ${providerDef.label} API key first.`);
      return;
    }

    try {
      await invoke("save_provider_key", { provider: providerId, apiKey: trimmedKey });
      await refreshState();
      setStatus(`${label} key saved locally.`);
    } catch (error) {
      setStatus(errorMessage(error));
    }
  }

  async function clearKey() {
    const providerId = state.settings.provider.provider;
    const providerDef = getMeshPromptProvider(providerId);
    const label = providerDef.label;

    try {
      await invoke("delete_provider_key", { provider: providerId });
      if (typeof window !== "undefined") {
        localStorage.removeItem(`meshprompt_key_${providerId}`);
      }
      setApiKeyDraft("");
      await refreshState();
      setStatus(`${label} key cleared.`);
    } catch (error) {
      setStatus(errorMessage(error));
    }
  }

  const activeAction = builtInPromptActions.find((action) => action.id === actionId) ?? builtInPromptActions[0];

  const renderUpdateModal = () => {
    if (!showUpdateModal || !updateInfo) return null;
    return (
      <div className="modal-overlay" onClick={() => !updating && setShowUpdateModal(false)}>
        <div className="modal-content card stack" onClick={(e) => e.stopPropagation()} style={{ width: '420px', padding: '24px' }}>
          <div className="card-heading" style={{ marginBottom: '16px' }}>
            <div>
              <span className="eyebrow" style={{ color: 'var(--text-accent)' }}>New Version Released</span>
              <h3>Olé {updateInfo.version}</h3>
            </div>
          </div>
          
          <div className="stack" style={{ gap: '12px', flex: 1, minHeight: 0 }}>
            <label>Release Notes</label>
            <div className="update-changelog">
              {updateInfo.changelog || "No release notes provided."}
            </div>
            
            {updateError && <div className="error-message" style={{ color: 'var(--error)', fontSize: '12px' }}>{updateError}</div>}
            
            {updating ? (
              <div className="stack" style={{ gap: '8px', alignItems: 'center', margin: '16px 0' }}>
                <div className="spinner" />
                <strong>Downloading & Installing...</strong>
                <span style={{ fontSize: '11px', color: 'var(--text-secondary)' }}>The application will automatically restart once finished.</span>
              </div>
            ) : (
              <div className="button-row" style={{ marginTop: '16px' }}>
                <button className="primary" onClick={() => void triggerUpdate()}>Update Now</button>
                <button onClick={() => setShowUpdateModal(false)}>Later</button>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  };

  if (embed) {
    return (
      <div className="meshprompt-theme app-body" style={{ flex: 1, display: 'flex', minHeight: 0, overflow: 'hidden', height: '100%', width: '100%' }}>
        {!hideSidebar && (
          <aside className="sidebar">
            <div className="brand-block">
              <div className="brand-mark">
                <img src="/ole-badge.svg" alt="" />
              </div>
              <div>
                <h1 className="brand-name">Olé</h1>
              </div>
            </div>
            <nav>
              <NavButton active={view === "text"} onClick={() => setView("text")} icon={<Wand2 size={15} />} label="Enhancer" />
              <NavButton active={view === "providers"} onClick={() => setView("providers")} icon={<KeyRound size={15} />} label="Test Provider" />
              <NavButton active={view === "actions"} onClick={() => setView("actions")} icon={<Sparkles size={15} />} label="Prompt Actions" />
              <NavButton active={view === "history"} onClick={() => setView("history")} icon={<History size={15} />} label="Action History" />
              <NavButton active={view === "settings"} onClick={() => setView("settings")} icon={<Settings size={15} />} label="Settings" />
            </nav>
            <div className="sidebar-footer">
              <span>{state.settings.paused ? "Shortcuts paused" : "Tray utility active"}</span>
              <strong>{state.settings.shortcut}</strong>
            </div>
          </aside>
        )}
        <main className="main-panel">
          <section className="content-scroll">
            {view === "text" && (
              <div className="stack compact-grid" style={{ gap: '24px' }}>
                <div className="stack" style={{ gap: '16px' }}>
                  <div className="card-heading">
                    <div>
                      <span className="eyebrow">Active Pipeline</span>
                      <h3>{activeAction.label}</h3>
                    </div>
                  </div>
                  
                  <div className="stack" style={{ gap: '8px' }}>
                    <label>Transformation Type</label>
                    <select value={actionId} onChange={(event) => setActionId(event.target.value)}>
                      {builtInPromptActions.map((action) => (
                        <option key={action.id} value={action.id}>{action.label}</option>
                      ))}
                    </select>
                  </div>
                  
                  <div className="stack" style={{ gap: '8px', flex: 1, minHeight: 0 }}>
                    <label>Original Prompt / Text</label>
                    <textarea 
                      value={selectedText} 
                      onChange={(event) => setSelectedText(event.target.value)} 
                      placeholder="Select text anywhere on your device and press Ctrl+Shift+Space, or paste/type text here to improve it." 
                      style={{ flex: 1 }}
                    />
                  </div>

                  <div className="stack" style={{ gap: '8px' }}>
                    <label>Contextual Guidelines / Focus (Optional)</label>
                    <textarea 
                      className="short" 
                      value={instruction} 
                      onChange={(event) => setInstruction(event.target.value)} 
                      placeholder="e.g. rewrite as a clear system instruction, keep it highly technical, use bullet points, etc." 
                    />
                  </div>

                  <div className="button-row">
                    <button className="primary" disabled={busy} onClick={() => void runAction()} style={{ cursor: 'pointer' }}>{busy ? "Enhancing..." : "Enhance Prompt"}</button>
                    <button disabled={!output} onClick={() => setOutput("")} style={{ cursor: 'pointer' }}>Clear</button>
                  </div>
                </div>
                
                {output && (
                  <>
                    <hr style={{ border: 'none', borderTop: '1px solid var(--border)', margin: '8px 0' }} />

                    <div className="stack" style={{ gap: '16px' }}>
                      <div className="card-heading">
                        <div>
                          <span className="eyebrow">Output Stream ({state.settings.provider.provider} / {state.settings.provider.model})</span>
                          <h3>Enhanced Prompt</h3>
                        </div>
                      </div>
                      
                      <textarea 
                        value={output} 
                        onChange={(event) => setOutput(event.target.value)} 
                        placeholder="Improved prompt text will appear here." 
                        style={{ flex: 1 }}
                      />
                      
                      <div className="button-row">
                        <button className="primary" disabled={!output} onClick={() => void replaceOutput()} style={{ cursor: 'pointer' }}>Replace Selection</button>
                        <button disabled={!output} onClick={() => void copyOutput()} style={{ cursor: 'pointer' }}><Copy size={14} /> Copy</button>
                        <button disabled={!selectedText || busy} onClick={() => void runAction()} style={{ cursor: 'pointer' }}><RotateCcw size={14} /> Retry</button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            )}

            {view === "actions" && <ActionsView settings={state.settings} saveSettings={saveSettings} />}
            
            {view === "providers" && (
              <ProviderView
                state={state}
                apiKeyDraft={apiKeyDraft}
                setApiKeyDraft={setApiKeyDraft}
                saveKey={saveKey}
                clearKey={clearKey}
                saveProvider={(provider) => saveSettings({ ...state.settings, provider })}
                setStatus={setStatus}
              />
            )}

            {view === "history" && (
              <HistoryView
                items={state.history}
                historyEnabled={state.settings.historyEnabled}
                sensitiveMode={state.settings.sensitiveMode}
                onUse={(item) => {
                  setSelectedText(item.input);
                  setOutput(item.output);
                  setActionId(item.actionId);
                  setView("text");
                }}
                onClear={async () => {
                  await invoke("clear_history");
                  await refreshState();
                  setStatus("History cleared");
                }}
              />
            )}

            {view === "settings" && <SettingsView settings={state.settings} saveSettings={saveSettings} />}
          </section>
        </main>
        {renderUpdateModal()}
      </div>
    );
  }

  return (
    <div className="app-shell flex-col">
      <TitleBar status={status} view={view} updateInfo={updateInfo} onUpdateClick={() => setShowUpdateModal(true)} />
      <div className="app-body">
        <aside className="sidebar">
          <div className="brand-block">
            <div className="brand-mark">
              <img src="/ole-badge.svg" alt="" />
            </div>
            <div>
              <h1 className="brand-name">Olé</h1>
            </div>
          </div>
          <nav>
            <NavButton active={view === "text"} onClick={() => setView("text")} icon={<Wand2 size={15} />} label="Enhancer" />
            <NavButton active={view === "providers"} onClick={() => setView("providers")} icon={<KeyRound size={15} />} label="Test Provider" />
            <NavButton active={view === "actions"} onClick={() => setView("actions")} icon={<Sparkles size={15} />} label="Prompt Actions" />
            <NavButton active={view === "history"} onClick={() => setView("history")} icon={<History size={15} />} label="Action History" />
            <NavButton active={view === "settings"} onClick={() => setView("settings")} icon={<Settings size={15} />} label="Settings" />
          </nav>
          <div className="sidebar-footer">
            <span>{state.settings.paused ? "Shortcuts paused" : "Tray utility active"}</span>
            <strong>{state.settings.shortcut}</strong>
          </div>
        </aside>
        <main className="main-panel">
          <section className="content-scroll">
            {view === "text" && (
              <div className="stack compact-grid" style={{ gap: '24px' }}>
                <div className="stack" style={{ gap: '16px' }}>
                  <div className="card-heading">
                    <div>
                      <span className="eyebrow">Active Pipeline</span>
                      <h3>{activeAction.label}</h3>
                    </div>
                  </div>
                  
                  <div className="stack" style={{ gap: '8px' }}>
                    <label>Transformation Type</label>
                    <select value={actionId} onChange={(event) => setActionId(event.target.value)}>
                      {builtInPromptActions.map((action) => (
                        <option key={action.id} value={action.id}>{action.label}</option>
                      ))}
                    </select>
                  </div>
                  
                  <div className="stack" style={{ gap: '8px', flex: 1, minHeight: 0 }}>
                    <label>Original Prompt / Text</label>
                    <textarea 
                      value={selectedText} 
                      onChange={(event) => setSelectedText(event.target.value)} 
                      placeholder="Select text anywhere on your device and press Ctrl+Shift+Space, or paste/type text here to improve it." 
                      style={{ flex: 1 }}
                    />
                  </div>

                  <div className="stack" style={{ gap: '8px' }}>
                    <label>Contextual Guidelines / Focus (Optional)</label>
                    <textarea 
                      className="short" 
                      value={instruction} 
                      onChange={(event) => setInstruction(event.target.value)} 
                      placeholder="e.g. rewrite as a clear system instruction, keep it highly technical, use bullet points, etc." 
                    />
                  </div>

                  <div className="button-row">
                    <button className="primary" disabled={busy} onClick={() => void runAction()} style={{ cursor: 'pointer' }}>{busy ? "Enhancing..." : "Enhance Prompt"}</button>
                    <button disabled={!output} onClick={() => setOutput("")} style={{ cursor: 'pointer' }}>Clear</button>
                  </div>
                </div>
                
                {output && (
                  <>
                    <hr style={{ border: 'none', borderTop: '1px solid var(--border)', margin: '8px 0' }} />

                    <div className="stack" style={{ gap: '16px' }}>
                      <div className="card-heading">
                        <div>
                          <span className="eyebrow">Output Stream ({state.settings.provider.provider} / {state.settings.provider.model})</span>
                          <h3>Enhanced Prompt</h3>
                        </div>
                      </div>
                      
                      <textarea 
                        value={output} 
                        onChange={(event) => setOutput(event.target.value)} 
                        placeholder="Improved prompt text will appear here." 
                        style={{ flex: 1 }}
                      />
                      
                      <div className="button-row">
                        <button className="primary" disabled={!output} onClick={() => void replaceOutput()} style={{ cursor: 'pointer' }}>Replace Selection</button>
                        <button disabled={!output} onClick={() => void copyOutput()} style={{ cursor: 'pointer' }}><Copy size={14} /> Copy</button>
                        <button disabled={!selectedText || busy} onClick={() => void runAction()} style={{ cursor: 'pointer' }}><RotateCcw size={14} /> Retry</button>
                      </div>
                    </div>
                  </>
                )}
              </div>
            )}

            {view === "actions" && <ActionsView settings={state.settings} saveSettings={saveSettings} />}
            
            {view === "providers" && (
              <ProviderView
                state={state}
                apiKeyDraft={apiKeyDraft}
                setApiKeyDraft={setApiKeyDraft}
                saveKey={saveKey}
                clearKey={clearKey}
                saveProvider={(provider) => saveSettings({ ...state.settings, provider })}
                setStatus={setStatus}
              />
            )}

            {view === "history" && (
              <HistoryView
                items={state.history}
                historyEnabled={state.settings.historyEnabled}
                sensitiveMode={state.settings.sensitiveMode}
                onUse={(item) => {
                  setSelectedText(item.input);
                  setOutput(item.output);
                  setActionId(item.actionId);
                  setView("text");
                }}
                onClear={async () => {
                  await invoke("clear_history");
                  await refreshState();
                  setStatus("History cleared");
                }}
              />
            )}

            {view === "settings" && <SettingsView settings={state.settings} saveSettings={saveSettings} />}
          </section>
        </main>
      </div>

      {renderUpdateModal()}
    </div>
  );
}

export function OverlayApp() {
  const [selectedText, setSelectedText] = useState("");
  const [output, setOutput] = useState("");
  const [query, setQuery] = useState("");
  const [actionId, setActionId] = useState("enhance-prompt");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [settings, setSettings] = useState<SettingsState>(fallbackSettings);
  const [phase, setPhase] = useState<OverlayPhase>("picker");
  const [status, setStatus] = useState("Select an action");

  const actions = useMemo(() => {
    const normalized = query.toLowerCase().trim();
    return builtInPromptActions.filter((action) =>
      `${action.label} ${action.description} ${action.category}`.toLowerCase().includes(normalized),
    );
  }, [query]);

  const displayedActions = useMemo(() => {
    return actions.filter((a) =>
      ["enhance-prompt", "make-concise", "expand-details", "rewrite-professionally", "developer-prompt", "product-prompt"].includes(a.id)
    );
  }, [actions]);

  const activeAction = displayedActions[selectedIndex] ?? builtInPromptActions.find((action) => action.id === actionId) ?? builtInPromptActions[0];

  useEffect(() => {
    document.body.style.background = "transparent";
    void invoke<AppState>("get_app_state").then((state) => {
      setSettings(state.settings);
      setActionId(state.settings.defaultActionId);
    });
    void invoke<string>("get_captured_text").then((text) => {
      if (text) {
        setSelectedText(text);
        setOutput("");
        setPhase("picker");
        setStatus("Text captured");
        void invoke("resize_overlay", { width: 480, height: 180 }).catch(() => {});
      }
    });
    const captured = listen<string>("meshprompt://captured-text", (event) => {
      void invoke<AppState>("get_app_state").then((state) => {
        setSettings(state.settings);
        if (!actionId) setActionId(state.settings.defaultActionId);
      });
      setSelectedText(event.payload ?? "");
      setOutput("");
      setPhase("picker");
      setStatus("Text captured");
      void invoke("resize_overlay", { width: 480, height: 180 }).catch(() => {});
    });
    const captureError = listen<string>("meshprompt://capture-error", (event) => {
      setStatus(event.payload ?? "No selected text found. Paste text below.");
      setPhase("picker");
      void invoke("resize_overlay", { width: 480, height: 180 }).catch(() => {});
    });
    return () => {
      void captured.then((off) => off());
      void captureError.then((off) => off());
    };
  }, []);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        void invoke("hide_overlay");
      }
      if (phase === "picker" && displayedActions.length > 0) {
        if (event.key === "ArrowRight") {
          event.preventDefault();
          setSelectedIndex((index) => (index + 1) % displayedActions.length);
        }
        if (event.key === "ArrowLeft") {
          event.preventDefault();
          setSelectedIndex((index) => (index - 1 + displayedActions.length) % displayedActions.length);
        }
        if (event.key === "ArrowDown") {
          event.preventDefault();
          setSelectedIndex((index) => {
            const nextIndex = index + 3;
            return nextIndex < displayedActions.length ? nextIndex : index;
          });
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          setSelectedIndex((index) => {
            const nextIndex = index - 3;
            return nextIndex >= 0 ? nextIndex : index;
          });
        }
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c" && output) {
        event.preventDefault();
        void copyOverlayOutput();
      }
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter" && output) {
        event.preventDefault();
        void replaceOverlayOutput();
      } else if (event.key === "Enter" && phase !== "processing") {
        event.preventDefault();
        void runOverlayAction();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [displayedActions, output, phase, selectedText, settings, activeAction]);

  async function runOverlayAction(nextActionId?: string) {
    const action = nextActionId
      ? builtInPromptActions.find((item) => item.id === nextActionId) ?? activeAction
      : activeAction;
    setActionId(action.id);
    if (!selectedText.trim()) {
      setStatus("Add text to enhance first.");
      return;
    }
    setPhase("processing");
    setStatus(action.label);
    void invoke("resize_overlay", { width: 480, height: 180 }).catch(() => {});
    try {
      const response = await generateWithCurrentProvider(settings, action.id, selectedText);
      
      if (settings.historyEnabled && !settings.sensitiveMode) {
        await invoke("add_history", {
          item: {
            id: crypto.randomUUID(),
            actionId: action.id,
            actionLabel: action.label,
            provider: response.provider,
            model: response.model,
            input: selectedText,
            output: response.content,
            createdAt: new Date().toISOString(),
          },
        });
      }

      try {
        await invoke("hide_overlay");
        await invoke("replace_selected_text", { text: response.content });
      } catch (error) {
        await invoke("show_overlay");
        await invoke("copy_text", { text: response.content });
        setOutput(response.content);
        setPhase("result");
        setStatus(`Replace failed. Copied instead. ${errorMessage(error)}`);
        void invoke("resize_overlay", { width: 480, height: 380 }).catch(() => {});
      }
    } catch (error: any) {
      setPhase("picker");
      let shortError = errorMessage(error);
      const msg = shortError.toLowerCase();
      if (msg.includes("fetch") || msg.includes("network")) shortError = "Network error. Check connection.";
      else if (msg.includes("401") || msg.includes("api key") || msg.includes("unauthorized")) shortError = "Invalid API key.";
      else if (msg.includes("404")) shortError = "Invalid model or endpoint unreachable.";
      setStatus(shortError);
    }
  }

  async function copyOverlayOutput() {
    await invoke("copy_text", { text: output });
    setStatus("Copied");
  }

  async function replaceOverlayOutput() {
    try {
      await invoke("replace_selected_text", { text: output });
      await invoke("hide_overlay");
    } catch (error) {
      await invoke("copy_text", { text: output });
      setStatus(`Replace failed. Copied instead. ${errorMessage(error)}`);
    }
  }

  const isErrorStatus = status && (
    status.toLowerCase().includes("fail") || 
    status.toLowerCase().includes("error") || 
    status.toLowerCase().includes("timed out") || 
    status.toLowerCase().includes("no selected text") ||
    status.toLowerCase().includes("invalid")
  );

  const displaySubtitle = phase === "result" 
    ? "Result" 
    : ((status && status !== "Select an action" && status !== "Output ready") 
        ? status 
        : "Improve selected text");

  return (
    <div className="overlay-shell meshprompt-theme">
      <div className="overlay-card">
        <div className="overlay-top" data-tauri-drag-region>
          <div data-tauri-drag-region>
            <span className="eyebrow" data-tauri-drag-region>OLÉ</span>
            <h2 
              data-tauri-drag-region 
              style={{ 
                color: isErrorStatus ? "var(--error)" : "var(--text-primary)",
                transition: "color 0.15s ease"
              }}
            >
              {displaySubtitle}
            </h2>
          </div>
          <button className="overlay-close-btn" data-no-drag onClick={() => invoke("hide_overlay")} aria-label="Close">
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
              <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
            </svg>
          </button>
        </div>

        {phase !== "result" && (
          <div className="overlay-search-container">
            <Search size={14} className="muted" />
            <input
              autoFocus
              placeholder="Paste text here to enhance..."
              value={selectedText}
              onChange={(e) => setSelectedText(e.target.value)}
              className="overlay-search-input"
            />
          </div>
        )}

        {phase === "picker" && (
          <div className="compact-action-list">
            {displayedActions.map((action, index) => (
              <button
                key={action.id}
                className={index === selectedIndex ? "compact-action active" : "compact-action"}
                title={action.description}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => {
                  setSelectedIndex(index);
                  void runOverlayAction(action.id);
                }}
              >
                <ActionIcon actionId={action.id} />
                <strong>{action.id === "rewrite-professionally" ? "Rewrite" : action.id === "developer-prompt" ? "Developer" : action.id === "product-prompt" ? "Product" : action.label}</strong>
              </button>
            ))}
          </div>
        )}

        {phase === "processing" && (
          <div className="processing-state">
            <div className="spinner" />
            <strong>{status}</strong>
            <span>Generating with {settings.provider.provider} / {settings.provider.model}</span>
            <button onClick={() => setPhase("picker")} style={{ cursor: 'pointer' }}>Cancel</button>
          </div>
        )}

        {phase === "result" && (
          <>
            <div className="result-preview">{output}</div>
            <div className="overlay-actions">
              <button className="primary" onClick={() => void copyOverlayOutput()} style={{ cursor: 'pointer' }}>Copy</button>
              <button onClick={() => void replaceOverlayOutput()} style={{ cursor: 'pointer' }}>Replace</button>
              <button onClick={() => void runOverlayAction()} style={{ cursor: 'pointer' }}>Retry</button>
              <button onClick={() => invoke("hide_overlay")} style={{ cursor: 'pointer' }}>Close</button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
