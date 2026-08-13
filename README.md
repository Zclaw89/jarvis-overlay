# MeshUtility Jarvis v1

Small Windows desktop orb built on this MeshUtility Tauri 2 + React app. It stays on top, opens a compact Jarvis chat from the orb/tray/hotkey, and keeps API keys in the app's secure key store.

## Run in dev

```bash
npm install
npm run tauri dev
```

## Paste API keys

Open **Settings > API Keys** for chat keys such as OpenAI, Anthropic, Gemini, Groq, or local Ollama URL.

Open **Settings > Voice & Hotkey** for voice. **Local (Kokoro)** is the default and uses this PC. On first speak, the Windows app downloads and starts its Kokoro helper automatically. No Python install is required.

If the Kokoro helper is still downloading, Jarvis uses the PC's Windows voice once and tries Kokoro again next time. OpenAI and ElevenLabs only run after you select them and save a key.

## Build the Windows app

Run this on the Windows PC that will build the `.exe`:

```bash
npm run tauri build
```

The NSIS installer is written under `src-tauri/target/release/bundle/nsis/`. This Linux VM does not produce or sign a Windows installer.

## Not done in v1

- Wake word
- Login/accounts
- Computer-use agent actions
- Signed Windows installer

## License

Apache-2.0. See `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md`.
