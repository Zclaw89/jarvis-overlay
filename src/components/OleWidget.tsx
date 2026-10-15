import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { Send, Settings, X } from "lucide-react";
import { getMeshPromptProvider, MeshPromptClient } from "../lib";
import { fallbackSettings, type AppState, type SettingsState } from "./PromptCommon";
import type { MeshPromptMessage } from "../lib/types";

type ChatMessage = {
  role: "user" | "assistant";
  content: string;
};

type MailboxJob = {
  id: string;
  inboxDir: string;
  outboxDir: string;
  screenshotPath?: string | null;
};

type DossierItem = {
  id: string;
  createdAt: string;
  note: string;
  screenshotPath: string;
  screenshotHash: string;
  sourceTitle?: string | null;
  analysis?: string | null;
};

type OleMode = "OLE" | "SCREEN_RECORD" | "VOICE_RECORD" | "CUSTOM";

const PLATE_WIDTH = 84;
const PLATE_HEIGHT = 124;
const CHAT_WIDTH = 360;
const CHAT_HEIGHT = 500;
const SCREEN_REQUEST_PATTERN = /\b(analy[sz]e this|look at this|what'?s on my screen|what is on my screen|my screen|this screen|screenshot)\b/i;

function shortProviderLabel(settings: SettingsState) {
  try {
    const provider = getMeshPromptProvider(settings.provider.provider);
    return `${provider.label} / ${settings.provider.model || provider.defaultModel}`;
  } catch {
    return "No AI provider selected";
  }
}

export function OleWidget() {
  const appWindowRef = useRef<ReturnType<typeof getCurrentWindow> | null>(null);
  const idleClickThroughTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const longPressTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tapTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const longPressDropRef = useRef(false);
  const dragRef = useRef<{ active: boolean; moved: boolean; lastY: number; startY: number; lockTriggered: boolean }>({ active: false, moved: false, lastY: 0, startY: 0, lockTriggered: false });
  const [expanded, setExpanded] = useState(false);
  const [settings, setSettings] = useState<SettingsState>(fallbackSettings);
  const [messages, setMessages] = useState<ChatMessage[]>([
    { role: "assistant", content: "Hi. Drop an Olé from the badge, or ask a quick question here." },
  ]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Ready");
  const [pendingMailboxJob, setPendingMailboxJob] = useState<MailboxJob | null>(null);
  const [dockSide, setDockSide] = useState<"left" | "right">("right");
  const [dockY, setDockY] = useState(50);
  const [transparency, setTransparency] = useState(100);
  const [mode, setMode] = useState<OleMode>("OLE");
  const [modeActive, setModeActive] = useState(false);
  const [lockRemoteItem, setLockRemoteItem] = useState<DossierItem | null>(null);
  const [lockInstruction, setLockInstruction] = useState("");

  const providerLabel = useMemo(() => shortProviderLabel(settings), [settings]);

  useEffect(() => {
    try {
      appWindowRef.current = getCurrentWindow();
    } catch {
      /* outside Tauri */
    }
    void refreshSettings();
    void refreshBubbleSettings();
  }, []);

  const setClickThrough = useCallback((enabled: boolean) => {
    void invoke("set_widget_click_through", { enabled }).catch(() => {});
  }, []);

  const resize = useCallback(async (open: boolean) => {
    const win = appWindowRef.current;
    if (!win) return;
    const nextSize = open ? new LogicalSize(CHAT_WIDTH, CHAT_HEIGHT) : new LogicalSize(PLATE_WIDTH, PLATE_HEIGHT);
    await win.setSize(nextSize).catch(() => {});
    if (!open) {
      await invoke("dock_ole_widget", { side: dockSide, vertical: dockY }).catch(() => {});
    }
  }, [dockSide, dockY]);

  const collapse = useCallback(() => {
    setExpanded(false);
    setLockRemoteItem(null);
    setStatus("Ready");
    void resize(false);
    if (idleClickThroughTimer.current) clearTimeout(idleClickThroughTimer.current);
    if (tapTimer.current) clearTimeout(tapTimer.current);
    setClickThrough(false);
  }, [resize, setClickThrough]);

  const expand = useCallback(() => {
    if (idleClickThroughTimer.current) clearTimeout(idleClickThroughTimer.current);
    setClickThrough(false);
    setExpanded(true);
    void resize(true);
  }, [resize, setClickThrough]);

  useEffect(() => {
    setClickThrough(false);
    void resize(false);
    const subs = [
      listen("ole://open-chat", expand),
      listen("hotkey-pressed", expand),
      listen("ole-bubble-settings-changed", () => {
        void refreshBubbleSettings();
      }),
    ];
    return () => {
      if (idleClickThroughTimer.current) clearTimeout(idleClickThroughTimer.current);
      setClickThrough(false);
      subs.forEach((sub) => sub.then((off) => off()));
    };
  }, [expand, resize, setClickThrough]);

  async function refreshBubbleSettings() {
    const [side, y, alpha] = await Promise.all([
      invoke<string | null>("get_setting", { key: "ole_dock_side" }).catch(() => null),
      invoke<string | null>("get_setting", { key: "ole_dock_y" }).catch(() => null),
      invoke<string | null>("get_setting", { key: "ole_bubble_transparency" }).catch(() => null),
    ]);
    const nextSide = side === "left" || side === "right" ? side : "right";
    const nextY = y ? Math.max(0, Math.min(100, Number(y))) : 50;
    const nextTransparency = alpha ? Math.max(40, Math.min(100, Number(alpha))) : 100;
    setDockSide(nextSide);
    setDockY(nextY);
    setTransparency(nextTransparency);
    if (!expanded) {
      await resize(false);
      await invoke("dock_ole_widget", { side: nextSide, vertical: nextY }).catch(() => {});
    }
  }

  async function refreshSettings() {
    try {
      const state = await invoke<AppState>("get_app_state");
      setSettings(state.settings);
      return state.settings;
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error));
      return settings;
    }
  }

  async function openSettings() {
    setClickThrough(false);
    await invoke("show_main_window").catch(() => {});
    await invoke("hide_widget").catch(() => {});
  }

  useEffect(() => {
    if (!pendingMailboxJob) return;
    const timer = setInterval(() => {
      void pollMailboxReply(pendingMailboxJob);
    }, 3500);
    void pollMailboxReply(pendingMailboxJob);
    return () => clearInterval(timer);
  }, [pendingMailboxJob]);

  async function getBrainMode(latestSettings: SettingsState) {
    const stored = await invoke<string | null>("get_setting", { key: "brain_mode" }).catch(() => null);
    if (stored === "mailbox" || stored === "api") return stored;
    return latestSettings.brainMode ?? "api";
  }

  function shouldAttachScreenshot(text: string) {
    return SCREEN_REQUEST_PATTERN.test(text);
  }

  async function sendToMailbox(text: string, includeScreenshot: boolean) {
    const job = await invoke<MailboxJob>("create_mailbox_job", {
      request: text,
      includeScreenshot,
    });
    setPendingMailboxJob(job);
    setMessages((current) => [
      ...current,
      {
        role: "assistant",
        content: includeScreenshot
          ? `Sent with a screenshot. Waiting for reply in ${job.outboxDir}.`
          : `Sent. Waiting for reply in ${job.outboxDir}.`,
      },
    ]);
    setStatus("Waiting for Zeus...");
  }

  async function pollMailboxReply(job: MailboxJob) {
    try {
      const reply = await invoke<string | null>("read_mailbox_reply", { jobId: job.id });
      if (!reply?.trim()) return;
      setPendingMailboxJob(null);
      setBusy(false);
      setStatus("Reply ready");
      setMessages((current) => [...current, { role: "assistant", content: reply }]);
      const latestSettings = await refreshSettings();
      await speakReply(reply, latestSettings);
    } catch (error) {
      setPendingMailboxJob(null);
      setBusy(false);
      setStatus("Mailbox error");
      setMessages((current) => [...current, { role: "assistant", content: error instanceof Error ? error.message : String(error) }]);
    }
  }

  async function speakReply(text: string, latestSettings: SettingsState) {
    let engine = latestSettings.voiceEngine;
    if (engine === "openai" || engine === "elevenlabs") {
      const provider = engine === "openai" ? "openai" : "elevenlabs";
      const key = await invoke<string | null>("get_provider_key", { provider }).catch(() => null);
      if (!key) engine = "kokoro";
    }
    await invoke("test_voice", {
      request: {
        engine,
        voiceId: latestSettings.voiceId,
        text,
      },
    }).catch(() => {});
  }

  async function dropAnOle(note = "Drop an Olé") {
    setClickThrough(false);
    setBusy(true);
    setStatus("Dropping an Olé...");
    try {
      const item = await invoke<DossierItem>("create_dossier_item", { note });
      setMessages((current) => [...current, { role: "assistant", content: `Saved to Living Dossier: ${item.screenshotHash.slice(0, 12)}` }]);
      setStatus("Capture saved");
    } catch (error) {
      setMessages((current) => [...current, { role: "assistant", content: error instanceof Error ? error.message : String(error) }]);
      setStatus("Drop failed");
    } finally {
      setBusy(false);
    }
  }

  function cycleMode(direction = 1) {
    const modes: OleMode[] = ["OLE", "SCREEN_RECORD", "VOICE_RECORD", "CUSTOM"];
    const index = modes.indexOf(mode);
    setMode(modes[(index + direction + modes.length) % modes.length]);
    setModeActive(false);
  }

  async function handleBadgeAction() {
    if (mode === "OLE") {
      await dropAnOle();
      return;
    }
    const stopping = modeActive;
    setModeActive(!stopping);
    setStatus(stopping ? `${mode} saved` : `${mode} started`);
    setMessages((current) => [
      ...current,
      {
        role: "assistant",
        content: stopping
          ? `${mode} stopped. Full media capture is a V1 placeholder; no background recorder was started.`
          : `${mode} started as a V1 placeholder. Tap again to stop.`,
      },
    ]);
  }

  async function lockRemoteCapture() {
    if (tapTimer.current) clearTimeout(tapTimer.current);
    if (longPressTimer.current) clearTimeout(longPressTimer.current);
    setClickThrough(false);
    setBusy(true);
    setStatus(mode === "VOICE_RECORD" && modeActive ? "Waiting for VOICE_RECORD to stop..." : "Lock capture...");
    try {
      if (mode === "VOICE_RECORD" && modeActive) {
        setMessages((current) => [...current, { role: "assistant", content: "VOICE_RECORD is active. Finish that recording before speaking into the lock remote." }]);
        return;
      }
      await invoke("flash_ole_screen").catch(() => {});
      const item = await invoke<DossierItem>("create_dossier_item", { note: "Lock remote capture" });
      setLockRemoteItem(item);
      setLockInstruction("");
      setExpanded(false);
      await appWindowRef.current?.setSize(new LogicalSize(300, 220)).catch(() => {});
      setStatus("Lock remote ready");
    } catch (error) {
      setMessages((current) => [...current, { role: "assistant", content: error instanceof Error ? error.message : String(error) }]);
      setStatus("Lock capture failed");
    } finally {
      setBusy(false);
    }
  }

  async function saveLockInstruction() {
    if (!lockRemoteItem) return;
    const text = lockInstruction.trim() || "No instruction added.";
    await invoke("attach_dossier_analysis", {
      id: lockRemoteItem.id,
      analysis: `Lock remote instruction:\n${text}`,
    }).catch(() => {});
    setMessages((current) => [...current, { role: "assistant", content: `Saved lock instruction for ${lockRemoteItem.screenshotHash.slice(0, 12)}.` }]);
    setLockRemoteItem(null);
    setLockInstruction("");
    void resize(false);
  }

  async function sendMessage() {
    const text = draft.trim();
    if (!text || busy) return;
    setDraft("");
    setBusy(true);
    setStatus("Thinking...");
    const nextMessages: ChatMessage[] = [...messages, { role: "user", content: text }];
    setMessages(nextMessages);

    try {
      const latestSettings = await refreshSettings();
      const useMailbox = await getBrainMode(latestSettings);
      const includeScreenshot = shouldAttachScreenshot(text);
      if (useMailbox === "mailbox" || includeScreenshot) {
        await sendToMailbox(text, includeScreenshot);
        return;
      }

      const provider = getMeshPromptProvider(latestSettings.provider.provider);
      const apiKey = provider.authMode === "api-key"
        ? await invoke<string | null>("get_provider_key", { provider: provider.id })
        : null;

      if (provider.authMode === "api-key" && !apiKey) {
        throw new Error(`Paste your ${provider.label} API key in Settings > AI Providers first.`);
      }

      const client = new MeshPromptClient({
        provider,
        credentials: { apiKey: apiKey ?? undefined, baseUrl: latestSettings.provider.baseUrl },
        timeoutMs: latestSettings.timeoutMs,
        appName: "Olé",
      });

      const requestMessages: MeshPromptMessage[] = [
        {
          role: "system",
          content: "You are Olé, a concise, friendly desktop assistant for a non-technical Windows user. Do not claim you can control the PC.",
        },
        ...nextMessages.slice(-8),
      ];

      const response = await client.generate({
        model: latestSettings.provider.model || provider.defaultModel,
        messages: requestMessages,
        temperature: Math.min(latestSettings.temperature ?? 0.35, 0.8),
        maxOutputTokens: Math.min(latestSettings.maxOutputTokens ?? 900, 900),
      });

      setMessages((current) => [...current, { role: "assistant", content: response.content }]);
      setStatus("Ready");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setMessages((current) => [...current, { role: "assistant", content: message }]);
      setStatus("Needs setup");
    } finally {
      setBusy(false);
    }
  }

  if (!expanded) {
    if (lockRemoteItem) {
      return (
        <div className="ole-stage">
          <section className="ole-lock-remote">
            <strong>Lock remote</strong>
            <span>{lockRemoteItem.screenshotHash.slice(0, 12)}</span>
            <textarea
              value={lockInstruction}
              onChange={(event) => setLockInstruction(event.target.value)}
              placeholder="Speak or type what Olé should do..."
              autoFocus
            />
            <div>
              <button onClick={() => void saveLockInstruction()}>Save</button>
              <button onClick={() => { setLockRemoteItem(null); void resize(false); }}>Close</button>
            </div>
          </section>
          <OleStyles />
        </div>
      );
    }

    return (
      <div className="ole-stage">
        <button
          className="ole-badge"
          aria-label="Drop an Olé"
          style={{ opacity: transparency / 100 }}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            dragRef.current = { active: true, moved: false, lastY: event.screenY, startY: event.screenY, lockTriggered: false };
            longPressDropRef.current = false;
            longPressTimer.current = setTimeout(() => {
              longPressDropRef.current = true;
              void handleBadgeAction();
            }, 550);
          }}
          onPointerMove={(event) => {
            if (!dragRef.current.active) return;
            const delta = event.screenY - dragRef.current.lastY;
            const total = event.screenY - dragRef.current.startY;
            if (total < -42 && !dragRef.current.lockTriggered) {
              dragRef.current = { ...dragRef.current, lockTriggered: true };
              longPressDropRef.current = true;
              if (longPressTimer.current) clearTimeout(longPressTimer.current);
              void lockRemoteCapture();
              return;
            }
            if (Math.abs(delta) < 2) return;
            dragRef.current = { ...dragRef.current, active: true, moved: true, lastY: event.screenY };
            if (longPressTimer.current) clearTimeout(longPressTimer.current);
            setDockY((current) => {
              const next = Math.max(0, Math.min(100, current + delta / 6));
              void invoke("dock_ole_widget", { side: dockSide, vertical: next }).catch(() => {});
              return next;
            });
          }}
          onPointerUp={(event) => {
            event.currentTarget.releasePointerCapture(event.pointerId);
            if (longPressTimer.current) clearTimeout(longPressTimer.current);
            if (dragRef.current.moved) {
              void invoke("set_setting", { key: "ole_dock_y", value: String(Math.round(dockY)) }).catch(() => {});
            }
            window.setTimeout(() => {
              dragRef.current = { active: false, moved: false, lastY: 0, startY: 0, lockTriggered: false };
            }, 0);
          }}
          onPointerCancel={() => {
            if (longPressTimer.current) clearTimeout(longPressTimer.current);
            dragRef.current = { active: false, moved: false, lastY: 0, startY: 0, lockTriggered: false };
          }}
          onDoubleClick={() => void lockRemoteCapture()}
          onClick={() => {
            if (longPressDropRef.current || dragRef.current.moved) return;
            if (tapTimer.current) clearTimeout(tapTimer.current);
            tapTimer.current = setTimeout(() => void handleBadgeAction(), 230);
          }}
          onWheel={(event) => {
            event.preventDefault();
            cycleMode(event.deltaY >= 0 ? 1 : -1);
          }}
        >
          <span className="ole-badge-core"><img src="/ole-badge.svg" alt="" /></span>
          <span className="ole-mode-label">{modeActive ? `${mode}●` : mode}</span>
          <span className="ole-badge-ring" />
        </button>
        <OleStyles />
      </div>
    );
  }

  return (
    <div className="ole-stage">
      <section className="ole-chat" onMouseDown={() => setClickThrough(false)}>
        <header className="ole-chat-head" data-tauri-drag-region>
          <div data-tauri-drag-region>
            <strong data-tauri-drag-region>Olé</strong>
            <span data-tauri-drag-region>{providerLabel}</span>
          </div>
          <div className="ole-head-actions">
            <button type="button" onClick={openSettings} title="Open settings">
              <Settings size={15} />
            </button>
            <button type="button" onClick={collapse} title="Close chat">
              <X size={15} />
            </button>
          </div>
        </header>

        <div className="ole-messages">
          {messages.map((message, index) => (
            <div key={`${message.role}-${index}`} className={`ole-message ${message.role}`}>
              {message.content}
            </div>
          ))}
          {busy && <div className="ole-message assistant">Thinking...</div>}
        </div>

        <footer className="ole-compose">
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                void sendMessage();
              }
              if (event.key === "Escape") collapse();
            }}
            placeholder="Ask Olé..."
            autoFocus
          />
          <button type="button" disabled={busy || !draft.trim()} onClick={() => void sendMessage()} aria-label="Send">
            <Send size={16} />
          </button>
        </footer>
        <div className="ole-status">{status}</div>
      </section>
      <OleStyles />
    </div>
  );
}

function OleStyles() {
  return (
    <style>{`
      *, *::before, *::after { box-sizing: border-box; }
      html, body, #root {
        width: 100%;
        height: 100%;
        margin: 0;
        overflow: hidden;
        background: transparent !important;
        font-family: 'Noto Sans', 'Segoe UI', sans-serif;
      }
      .ole-stage {
        width: 100vw;
        height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
      }
      .ole-badge {
        position: relative;
        width: 78px;
        height: 112px;
        border: 1px solid rgba(255, 229, 168, 0.34);
        border-radius: 999px;
        padding: 0;
        background: rgba(7, 7, 7, 0.68);
        color: #ecfeff;
        cursor: grab;
        box-shadow: 0 0 24px rgba(248, 208, 112, 0.38), inset 0 0 18px rgba(255,255,255,0.12);
        -webkit-app-region: no-drag;
      }
      .ole-badge:active { cursor: grabbing; }
      .ole-badge-core {
        position: absolute;
        inset: 9px;
        display: grid;
        place-items: center;
        border-radius: 50%;
        background: transparent;
        backdrop-filter: blur(8px);
      }
      .ole-badge-core img {
        width: 62px;
        height: 62px;
        display: block;
        border-radius: 50%;
        filter: drop-shadow(0 0 12px rgba(248, 208, 112, 0.42));
      }
      .ole-mode-label {
        position: absolute;
        left: 0;
        right: 0;
        bottom: 15px;
        color: #ffe9a8;
        font-size: 10px;
        font-weight: 800;
        letter-spacing: 0.08em;
        text-align: center;
        text-shadow: 0 1px 4px rgba(0,0,0,0.82);
      }
      .ole-badge-ring {
        position: absolute;
        inset: 5px;
        border-radius: 999px;
        border: 1px solid rgba(248, 208, 112, 0.32);
        animation: olePulse 2.3s ease-in-out infinite;
      }
      .ole-lock-remote {
        width: 288px;
        height: 208px;
        display: flex;
        flex-direction: column;
        gap: 8px;
        padding: 14px;
        border-radius: 22px;
        border: 1px solid rgba(255, 229, 168, 0.28);
        background: rgba(9, 9, 9, 0.88);
        color: #ffe9a8;
        box-shadow: 0 14px 50px rgba(0,0,0,0.45);
      }
      .ole-lock-remote span {
        color: rgba(255, 233, 168, 0.68);
        font-size: 11px;
      }
      .ole-lock-remote textarea {
        flex: 1;
        min-height: 76px;
        resize: none;
        border-radius: 12px;
        border: 1px solid rgba(255, 229, 168, 0.2);
        background: rgba(0,0,0,0.34);
        color: #fff7dd;
        padding: 10px;
        outline: none;
      }
      .ole-lock-remote div {
        display: flex;
        gap: 8px;
        justify-content: flex-end;
      }
      .ole-lock-remote button {
        border: 1px solid rgba(255, 229, 168, 0.24);
        border-radius: 10px;
        background: rgba(255, 229, 168, 0.1);
        color: #ffe9a8;
        padding: 6px 10px;
        cursor: pointer;
      }
      .ole-chat {
        width: 348px;
        height: 488px;
        display: flex;
        flex-direction: column;
        overflow: hidden;
        border: 1px solid rgba(125, 211, 252, 0.22);
        border-radius: 26px;
        background: linear-gradient(145deg, rgba(9, 14, 29, 0.94), rgba(12, 19, 38, 0.86));
        color: #e5f6ff;
        box-shadow: 0 16px 80px rgba(0, 0, 0, 0.62), 0 0 48px rgba(56, 189, 248, 0.18);
        backdrop-filter: blur(22px) saturate(150%);
      }
      .ole-chat-head {
        height: 62px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 14px 10px 18px;
        border-bottom: 1px solid rgba(125, 211, 252, 0.14);
        cursor: move;
      }
      .ole-chat-head strong {
        display: block;
        font-size: 15px;
        letter-spacing: 0.02em;
      }
      .ole-chat-head span {
        display: block;
        max-width: 220px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: rgba(229, 246, 255, 0.62);
        font-size: 11px;
      }
      .ole-head-actions {
        display: flex;
        gap: 6px;
      }
      .ole-head-actions button,
      .ole-compose button {
        border: 1px solid rgba(125, 211, 252, 0.2);
        background: rgba(125, 211, 252, 0.08);
        color: #e5f6ff;
        border-radius: 12px;
        cursor: pointer;
      }
      .ole-head-actions button {
        width: 30px;
        height: 30px;
        display: grid;
        place-items: center;
      }
      .ole-messages {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 10px;
        padding: 14px;
        overflow-y: auto;
      }
      .ole-message {
        max-width: 86%;
        padding: 10px 12px;
        border-radius: 16px;
        font-size: 13px;
        line-height: 1.45;
        white-space: pre-wrap;
      }
      .ole-message.assistant {
        align-self: flex-start;
        background: rgba(125, 211, 252, 0.1);
        border: 1px solid rgba(125, 211, 252, 0.14);
      }
      .ole-message.user {
        align-self: flex-end;
        background: rgba(59, 130, 246, 0.28);
        border: 1px solid rgba(147, 197, 253, 0.24);
      }
      .ole-compose {
        display: flex;
        gap: 8px;
        padding: 12px;
        border-top: 1px solid rgba(125, 211, 252, 0.14);
      }
      .ole-compose textarea {
        flex: 1;
        min-height: 46px;
        max-height: 94px;
        resize: none;
        border: 1px solid rgba(125, 211, 252, 0.18);
        border-radius: 14px;
        background: rgba(2, 6, 23, 0.56);
        color: #e5f6ff;
        padding: 10px 12px;
        outline: none;
      }
      .ole-compose button {
        width: 46px;
        height: 46px;
      }
      .ole-compose button:disabled {
        opacity: 0.45;
        cursor: default;
      }
      .ole-status {
        height: 22px;
        padding: 0 16px 10px;
        color: rgba(229, 246, 255, 0.54);
        font-size: 11px;
      }
      @keyframes olePulse {
        0%, 100% { transform: scale(0.94); opacity: 0.72; }
        50% { transform: scale(1.08); opacity: 0.18; }
      }
    `}</style>
  );
}
