# Olé desktop

Windows desktop twin of the Android Olé app.

## Run in dev

```bash
npm install
npm run tauri dev
```

## Build the Windows app

Run this on the Windows PC that will build the `.exe`:

```bash
npm run tauri build
```

The NSIS installer is named Olé and is written under `src-tauri/target/release/bundle/nsis/`. This Linux VM does not produce or sign a Windows installer.

## Settings

- **API Keys**: paste OpenAI or another supported chat key for fast analysis.
- **Voice & Hotkey**: Local (Kokoro) is the default. First speak downloads the Kokoro helper automatically; no Python install is required. OpenAI and ElevenLabs only run after you select them and save a key.
- **Floating badge**: choose left/right dock, vertical position, and transparency.

## Drop an Olé

Click or long-press the docked badge to take a normal screenshot and save a local Living Dossier item under `Documents\OleDossier\items`.

If an API key is present, Olé adds a short structured analysis to the item. If not, the capture is saved and analysis waits for a key.

Windows does not have Android's share sheet. On desktop, use Drop an Olé; Android remains the share door.

## Not in v1

Wake word, login/accounts, computer-use clicking, signed installer, always-on screen recorder, phone DRM metadata, ping-pong bounce, full patent graph/DAG orchestration.

## License

Apache-2.0. See `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md`.
