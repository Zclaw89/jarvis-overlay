import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff, KeyRound, CheckCircle2 } from "lucide-react";
import {
  getMeshPromptProvider,
  meshPromptProviders,
  MeshPromptClient,
  type MeshPromptProviderId,
} from "../lib";
import { type AppState, type ProviderSettings } from "./PromptCommon";

export function ProviderView(props: {
  state: AppState;
  apiKeyDraft: string;
  setApiKeyDraft: (value: string) => void;
  saveKey: () => Promise<void>;
  clearKey: () => Promise<void>;
  saveProvider: (settings: ProviderSettings) => Promise<void>;
  setStatus: (value: string) => void;
}) {
  const { state, apiKeyDraft, setApiKeyDraft, saveKey, clearKey, saveProvider } = props;
  const [testing, setTesting] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [saved, setSaved] = useState(false);
  const [testResult, setTestResult] = useState<{ success: boolean; latency?: number; error?: string } | null>(null);
  const [originalKey, setOriginalKey] = useState("");

  async function handleSaveKeyClick() {
    await saveKey();
    setOriginalKey(apiKeyDraft);
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  }
  
  const provider = getMeshPromptProvider(state.settings.provider.provider);
  const hasSavedKey = Boolean(state.keyStatus[provider.id]);

  useEffect(() => {
    let active = true;
    async function loadKey() {
      try {
        const key = await invoke<string | null>("get_provider_key", { provider: provider.id });
        if (active) {
          const resolvedKey = key ?? "";
          setApiKeyDraft(resolvedKey);
          setOriginalKey(resolvedKey);
          // Reset test connection indicator on provider swap
          setTestResult(null);
        }
      } catch (err) {
        console.error(err);
      }
    }
    loadKey();
    return () => {
      active = false;
    };
  }, [provider.id, setApiKeyDraft]);

  async function testConnection() {
    setTesting(true);
    setTestResult(null);
    
    try {
      const key = apiKeyDraft.trim() || (await invoke<string | null>("get_provider_key", { provider: provider.id }));
      if (provider.authMode === "api-key" && !key) {
        throw new Error(`API Key is required for ${provider.label}. Connection aborted.`);
      }
      
      const resolvedModel = state.settings.provider.model || provider.defaultModel;
      
      const startTime = Date.now();
      const client = new MeshPromptClient({
        provider,
        credentials: { apiKey: key ?? undefined, baseUrl: state.settings.provider.baseUrl },
        timeoutMs: Math.min(state.settings.timeoutMs, 20_000),
        appName: "Olé",
      });

      await client.generate({
        model: resolvedModel,
        messages: [{ role: "user", content: "Reply brief only: Online" }],
        maxOutputTokens: 20,
        temperature: 0,
      });
      
      const latency = Date.now() - startTime;
      
      setTestResult({
        success: true,
        latency,
      });
    } catch (error: any) {
      const message = error?.message || String(error);
      let shortError = message;
      const msg = message.toLowerCase();
      if (msg.includes("fetch") || msg.includes("network")) shortError = "Network error";
      else if (msg.includes("401") || msg.includes("api key") || msg.includes("unauthorized")) shortError = "Invalid API key";
      else if (msg.includes("404")) shortError = "Model invalid or endpoint unreachable";
      
      setTestResult({
        success: false,
        error: shortError,
      });
    } finally {
      setTesting(false);
    }
  }

  function getProviderIcon(id: string) {
    let src = "/ole-badge.svg";
    switch (id) {
      case "xai":
        src = "/xai_logo.svg";
        break;
      case "openai":
        src = "/openai_logo.svg";
        break;
      case "anthropic":
        src = "/anthropic_logo.svg";
        break;
      case "gemini":
        src = "/gemini_logo.svg";
        break;
      case "groq":
        src = "/groq_logo.svg";
        break;
      case "openrouter":
        src = "/openrouter_logo.svg";
        break;
      case "ollama":
        src = "/ollama_logo.svg";
        break;
    }
    return (
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '28px',
          height: '28px',
          borderRadius: '6px',
          background: '#ffffff', // Clean white background makes all dark/brand logos pop
          border: '1px solid rgba(255, 255, 255, 0.15)',
          padding: '4px',
          flexShrink: 0,
          boxShadow: '0 2px 8px rgba(0, 0, 0, 0.2)',
        }}
      >
        <img
          src={src}
          alt=""
          style={{
            width: '100%',
            height: '100%',
            objectFit: 'contain',
          }}
        />
      </div>
    );
  }

  return (
    <div className="stack" style={{ maxWidth: '800px', margin: '0 auto', gap: '24px', padding: '16px 24px' }}>
      <div style={{ borderBottom: '1px solid var(--border)', paddingBottom: '20px', marginBottom: '8px' }}>
        <h2 style={{ fontSize: '18px', fontWeight: 600, color: 'var(--text-primary)', margin: '4px 0 0 0', display: 'flex', alignItems: 'center', gap: '10px' }}>
          <KeyRound size={18} style={{ color: 'var(--accent)' }} /> API keys
        </h2>
      </div>

      {/* 1. Dropdown Selector */}
      <div className="stack" style={{ gap: '8px' }}>
        <label style={{ fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600, color: 'var(--text-secondary)' }}>Choose chat provider</label>
        <select
          value={provider.id}
          onChange={(event) => {
            const selectedId = event.target.value as MeshPromptProviderId;
            const selectedProv = getMeshPromptProvider(selectedId);
            void saveProvider({ provider: selectedId, model: selectedProv.defaultModel });
          }}
          style={{
            background: 'var(--card)',
            border: '1px solid var(--border)',
            borderRadius: '8px',
            padding: '0 36px 0 12px',
            height: '38px',
            color: 'var(--text-primary)',
            cursor: 'pointer',
            width: '100%',
          }}
        >
          {meshPromptProviders.map((item) => (
              <option key={item.id} value={item.id} style={{ background: 'var(--surface)', color: 'var(--text-primary)' }}>
              {item.label} ({item.authMode === "api-key" ? "needs an API key" : "runs on this PC"})
            </option>
          ))}
        </select>
      </div>

      {/* Divider */}
      <div style={{ borderBottom: '1px solid var(--border)' }} />

      {/* 2. Provider Parameters Area */}
      <div className="stack" style={{ gap: '20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
            {getProviderIcon(provider.id)}
            <h4 style={{ fontSize: '12px', textTransform: 'uppercase', letterSpacing: '0.08em', fontWeight: 700, color: 'var(--text-primary)', margin: 0 }}>
              {provider.label}
            </h4>
          </div>
          {hasSavedKey && <span className="success-chip" style={{ fontSize: '11px', color: '#10b981', display: 'flex', alignItems: 'center', gap: '4px' }}><CheckCircle2 size={12} /> Key Secured</span>}
        </div>

        <div className="stack" style={{ gap: '8px' }}>
          <label style={{ fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600, color: 'var(--text-secondary)' }}>Chat model</label>
          <select
            value={state.settings.provider.model}
            onChange={(event) => void saveProvider({ ...state.settings.provider, model: event.target.value })}
            style={{
              background: 'var(--card)',
              border: '1px solid var(--border)',
              borderRadius: '8px',
              padding: '0 36px 0 12px',
              height: '38px',
              color: 'var(--text-primary)',
              cursor: 'pointer',
              width: '100%',
            }}
          >
            {provider.models.map((model) => (
              <option key={model.id} value={model.id} style={{ background: 'var(--surface)', color: 'var(--text-primary)' }}>
                {model.label} ({model.id})
              </option>
            ))}
          </select>
        </div>

        {provider.supportsCustomBaseUrl && (
          <div className="advanced-toggle stack" style={{ gap: '8px' }}>
            <button className="link-button" onClick={() => setShowAdvanced(!showAdvanced)} style={{ alignSelf: 'flex-start', padding: 0, border: 'none', background: 'none', color: 'var(--accent)', cursor: 'pointer', fontSize: '12px' }}>
              {showAdvanced ? "Hide custom URL" : "Use a custom URL"}
            </button>
            {showAdvanced && (
              <div className="advanced-panel stack" style={{ gap: '8px', width: '100%' }}>
                <label style={{ fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600, color: 'var(--text-secondary)' }}>Custom URL</label>
                <div className="row-input" style={{ display: 'flex', gap: '8px', width: '100%' }}>
                  <input
                    value={state.settings.provider.baseUrl ?? ""}
                    onChange={(event) => void saveProvider({ ...state.settings.provider, baseUrl: event.target.value })}
                    placeholder={provider.endpoint.baseUrl}
                    style={{
                      flex: 1,
                      background: 'var(--card)',
                      border: '1px solid var(--border)',
                      borderRadius: '8px',
                      padding: '0 12px',
                      height: '38px',
                      color: 'var(--text-primary)',
                    }}
                  />
                  <button 
                    onClick={() => void saveProvider({ ...state.settings.provider, baseUrl: "" })}
                    style={{
                      background: 'var(--muted)',
                      border: '1px solid var(--border)',
                      borderRadius: '8px',
                      padding: '0 16px',
                      cursor: 'pointer',
                      height: '38px',
                      color: 'var(--text-primary)',
                    }}
                  >
                    Reset
                  </button>
                </div>
              </div>
            )}
          </div>
        )}

        {provider.authMode === "api-key" && (
          <div className="stack" style={{ gap: '8px' }}>
            <label style={{ fontSize: '11px', textTransform: 'uppercase', letterSpacing: '0.05em', fontWeight: 600, color: 'var(--text-secondary)' }}>API key</label>
            <div className="row-input" style={{ position: 'relative', display: 'flex', width: '100%' }}>
              <input
                type={showKey ? "text" : "password"}
                value={apiKeyDraft}
                onChange={(event) => setApiKeyDraft(event.target.value)}
                placeholder={hasSavedKey ? "Saved in this computer's secure store" : `Paste ${provider.label} API key`}
                style={{
                  flex: 1,
                  background: 'var(--card)',
                  border: '1px solid var(--border)',
                  borderRadius: '8px',
                  padding: '0 40px 0 12px',
                  height: '38px',
                  color: 'var(--text-primary)',
                  fontFamily: apiKeyDraft ? "inherit" : "'JetBrains Mono', monospace",
                }}
              />
              <button 
                className="link-button" 
                onClick={() => setShowKey(!showKey)}
                style={{ 
                  position: 'absolute', 
                  right: '12px', 
                  top: '50%', 
                  transform: 'translateY(-50%)', 
                  border: 'none', 
                  background: 'transparent',
                  height: 'auto',
                  textDecoration: 'none',
                  color: 'var(--text-secondary)',
                  cursor: 'pointer',
                }}
              >
                {showKey ? <EyeOff size={16} /> : <Eye size={16} />}
              </button>
            </div>
            <p style={{ fontSize: '11px', color: 'var(--text-muted)', margin: 0 }}>Saved locally in the operating system's protected storage on Windows.</p>
          </div>
        )}

        <div style={{ display: 'flex', flexDirection: 'column', gap: '12px', marginTop: '8px' }}>
          {provider.authMode === "api-key" && (
            <button 
              className={saved ? "btn-premium-save-success" : "btn-premium-save"} 
              disabled={!saved && (!apiKeyDraft.trim() || apiKeyDraft === originalKey)} 
              onClick={handleSaveKeyClick}
              style={{ width: '100%', height: '42px', borderRadius: '10px' }}
            >
              {saved ? "Saved" : "Save settings"}
            </button>
          )}

          <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
            <button 
              className="btn-premium-test" 
              onClick={() => void testConnection()} 
              disabled={testing || (provider.authMode === "api-key" && !hasSavedKey && !apiKeyDraft.trim())}
              style={{ flex: 1, height: '38px', borderRadius: '8px', cursor: 'pointer' }}
            >
              {testing ? "Testing..." : "Test chat"}
            </button>
            {provider.authMode === "api-key" && hasSavedKey && (
              <button
                className="btn-premium-clear"
                onClick={() => void clearKey()}
                style={{ flex: 0, height: '38px', borderRadius: '8px', cursor: 'pointer', whiteSpace: 'nowrap', padding: '0 14px' }}
              >
                Remove key
              </button>
            )}

            {testResult && (
              <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexShrink: 0 }}>
                <div 
                  style={{
                    width: '10px',
                    height: '10px',
                    borderRadius: '50%',
                    background: testResult.success ? '#10b981' : '#ef4444',
                    boxShadow: testResult.success ? '0 0 8px rgba(16, 185, 129, 0.6)' : '0 0 8px rgba(239, 68, 68, 0.6)',
                  }}
                />
                <span style={{ fontSize: '13px', fontWeight: 500, color: testResult.success ? 'var(--text-primary)' : '#ef4444' }}>
                  {testResult.success ? `Connected (${testResult.latency}ms)` : 'Connection Failed'}
                </span>
                {!testResult.success && testResult.error && (
                  <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                    ({testResult.error})
                  </span>
                )}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
