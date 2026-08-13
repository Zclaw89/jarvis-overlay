import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { Bot, Send, Settings, X } from "lucide-react";
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

const ORB_SIZE = 76;
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

export function JarvisWidget() {
  const appWindowRef = useRef<ReturnType<typeof getCurrentWindow> | null>(null);
  const idleClickThroughTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [settings, setSettings] = useState<SettingsState>(fallbackSettings);
  const [messages, setMessages] = useState<ChatMessage[]>([
    { role: "assistant", content: "Hi. Ask me anything, or paste text you want help with." },
  ]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("Ready");
  const [pendingMailboxJob, setPendingMailboxJob] = useState<MailboxJob | null>(null);

  const providerLabel = useMemo(() => shortProviderLabel(settings), [settings]);

  useEffect(() => {
    try {
      appWindowRef.current = getCurrentWindow();
    } catch {
      /* outside Tauri */
    }
    void refreshSettings();
  }, []);

  const setClickThrough = useCallback((enabled: boolean) => {
    void invoke("set_widget_click_through", { enabled }).catch(() => {});
  }, []);

  const resize = useCallback(async (open: boolean) => {
    const win = appWindowRef.current;
    if (!win) return;
    const nextSize = open ? new LogicalSize(CHAT_WIDTH, CHAT_HEIGHT) : new LogicalSize(ORB_SIZE, ORB_SIZE);
    await win.setSize(nextSize).catch(() => {});
  }, []);

  const collapse = useCallback(() => {
    setExpanded(false);
    setStatus("Ready");
    void resize(false);
    if (idleClickThroughTimer.current) clearTimeout(idleClickThroughTimer.current);
    idleClickThroughTimer.current = setTimeout(() => setClickThrough(true), 900);
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
    idleClickThroughTimer.current = setTimeout(() => setClickThrough(true), 1200);
    const subs = [
      listen("jarvis://open-chat", expand),
      listen("hotkey-pressed", expand),
    ];
    return () => {
      if (idleClickThroughTimer.current) clearTimeout(idleClickThroughTimer.current);
      setClickThrough(false);
      subs.forEach((sub) => sub.then((off) => off()));
    };
  }, [expand, resize, setClickThrough]);

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
          ? `Sent to Zeus with a screenshot. Waiting for reply in ${job.outboxDir}.`
          : `Sent to Zeus. Waiting for reply in ${job.outboxDir}.`,
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
      setStatus("Zeus replied");
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
        appName: "MeshUtility Jarvis",
      });

      const requestMessages: MeshPromptMessage[] = [
        {
          role: "system",
          content: "You are a concise, friendly desktop assistant for a non-technical Windows user. Do not claim you can control the PC.",
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
    return (
      <div className="jarvis-stage">
        <button className="jarvis-orb" aria-label="Open Jarvis chat" onClick={expand}>
          <span className="jarvis-orb-core"><Bot size={24} /></span>
          <span className="jarvis-orb-ring" />
        </button>
        <JarvisStyles />
      </div>
    );
  }

  return (
    <div className="jarvis-stage">
      <section className="jarvis-chat" onMouseDown={() => setClickThrough(false)}>
        <header className="jarvis-chat-head" data-tauri-drag-region>
          <div data-tauri-drag-region>
            <strong data-tauri-drag-region>Jarvis</strong>
            <span data-tauri-drag-region>{providerLabel}</span>
          </div>
          <div className="jarvis-head-actions">
            <button type="button" onClick={openSettings} title="Open settings">
              <Settings size={15} />
            </button>
            <button type="button" onClick={collapse} title="Close chat">
              <X size={15} />
            </button>
          </div>
        </header>

        <div className="jarvis-messages">
          {messages.map((message, index) => (
            <div key={`${message.role}-${index}`} className={`jarvis-message ${message.role}`}>
              {message.content}
            </div>
          ))}
          {busy && <div className="jarvis-message assistant">Thinking...</div>}
        </div>

        <footer className="jarvis-compose">
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
            placeholder="Ask Jarvis..."
            autoFocus
          />
          <button type="button" disabled={busy || !draft.trim()} onClick={() => void sendMessage()} aria-label="Send">
            <Send size={16} />
          </button>
        </footer>
        <div className="jarvis-status">{status}</div>
      </section>
      <JarvisStyles />
    </div>
  );
}

function JarvisStyles() {
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
      .jarvis-stage {
        width: 100vw;
        height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: transparent;
      }
      .jarvis-orb {
        position: relative;
        width: 68px;
        height: 68px;
        border: 0;
        border-radius: 50%;
        padding: 0;
        background: radial-gradient(circle at 34% 28%, #f4fbff 0 7%, #7dd3fc 8% 20%, #2563eb 38%, #111827 68%, #050816 100%);
        color: #ecfeff;
        cursor: pointer;
        box-shadow: 0 0 24px rgba(56, 189, 248, 0.82), 0 0 72px rgba(37, 99, 235, 0.42), inset 0 0 18px rgba(255,255,255,0.22);
        -webkit-app-region: no-drag;
      }
      .jarvis-orb-core {
        position: absolute;
        inset: 13px;
        display: grid;
        place-items: center;
        border-radius: 50%;
        background: rgba(8, 13, 28, 0.58);
        backdrop-filter: blur(8px);
      }
      .jarvis-orb-ring {
        position: absolute;
        inset: -5px;
        border-radius: 50%;
        border: 1px solid rgba(125, 211, 252, 0.72);
        animation: jarvisPulse 2.3s ease-in-out infinite;
      }
      .jarvis-chat {
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
      .jarvis-chat-head {
        height: 62px;
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 14px 10px 18px;
        border-bottom: 1px solid rgba(125, 211, 252, 0.14);
        cursor: move;
      }
      .jarvis-chat-head strong {
        display: block;
        font-size: 15px;
        letter-spacing: 0.02em;
      }
      .jarvis-chat-head span {
        display: block;
        max-width: 220px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        color: rgba(229, 246, 255, 0.62);
        font-size: 11px;
      }
      .jarvis-head-actions {
        display: flex;
        gap: 6px;
      }
      .jarvis-head-actions button,
      .jarvis-compose button {
        border: 1px solid rgba(125, 211, 252, 0.2);
        background: rgba(125, 211, 252, 0.08);
        color: #e5f6ff;
        border-radius: 12px;
        cursor: pointer;
      }
      .jarvis-head-actions button {
        width: 30px;
        height: 30px;
        display: grid;
        place-items: center;
      }
      .jarvis-messages {
        flex: 1;
        display: flex;
        flex-direction: column;
        gap: 10px;
        padding: 14px;
        overflow-y: auto;
      }
      .jarvis-message {
        max-width: 86%;
        padding: 10px 12px;
        border-radius: 16px;
        font-size: 13px;
        line-height: 1.45;
        white-space: pre-wrap;
      }
      .jarvis-message.assistant {
        align-self: flex-start;
        background: rgba(125, 211, 252, 0.1);
        border: 1px solid rgba(125, 211, 252, 0.14);
      }
      .jarvis-message.user {
        align-self: flex-end;
        background: rgba(59, 130, 246, 0.28);
        border: 1px solid rgba(147, 197, 253, 0.24);
      }
      .jarvis-compose {
        display: flex;
        gap: 8px;
        padding: 12px;
        border-top: 1px solid rgba(125, 211, 252, 0.14);
      }
      .jarvis-compose textarea {
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
      .jarvis-compose button {
        width: 46px;
        height: 46px;
      }
      .jarvis-compose button:disabled {
        opacity: 0.45;
        cursor: default;
      }
      .jarvis-status {
        height: 22px;
        padding: 0 16px 10px;
        color: rgba(229, 246, 255, 0.54);
        font-size: 11px;
      }
      @keyframes jarvisPulse {
        0%, 100% { transform: scale(0.94); opacity: 0.72; }
        50% { transform: scale(1.08); opacity: 0.18; }
      }
    `}</style>
  );
}
