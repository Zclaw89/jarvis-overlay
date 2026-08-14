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
- **Floating badge**: choose rail left/right/auto, vertical position, and transparency. Wheel over the badge to switch modes: Olé, REC, MIC, CAM.

## Drop an Olé

Click or long-press the docked badge in Olé mode to take a normal screenshot and save a local Living Dossier item under `Documents\OleDossier\items`.

The desktop data model mirrors Android names as plain local JSON: `ole_containers`, `source_artifacts`, and `ole_artifact_links`. Source artifacts store file path, sha256, capturedAt, mime, and metadataJson.

Drag a text/image/video file onto Living Dossier to copy it into Olé storage with sha256 and UNVERIFIED source metadata. Windows does not have Android's share sheet; Android remains the share door.

AI analysis/report links are stubbed in V1. API keys stay available for fast chat, but captures are saved first and no paid API is test-called automatically.

## Not in v1

Wake word, login/accounts, computer-use clicking, signed installer, always-on screen recorder, phone DRM metadata, ping-pong bounce, full patent graph/DAG orchestration.

## License

Apache-2.0. See `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md`.
