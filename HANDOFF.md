# Handoff — Redge's workspace (for Kimi, or any local agent)

**Written:** 2026-09-11, by Claude Code (cloud session) working in
`Zclaw89/jarvis-overlay`.
**Read this before you answer "did I get a handoff?" again.** Nothing gets
injected into your session automatically — this file *is* the handoff. It is
committed to the repo so it survives between sessions.

If you are running on **DESKTOP-80E3QF7**: the ground truth for code is GitHub,
not whatever is sitting in `Documents`. Pull before you trust a local copy.

---

## 1. The one thing to understand first

There are **four separate projects** living in **two GitHub repos**, and three
of the four share a single repo (`jarvis-overlay`) by *branch*, not by folder.
That is unusual and it is the main thing people get wrong. The repo name
`jarvis-overlay` is historical — `main` is not Jarvis at all.

| Branch / repo | What it actually is | State |
|---|---|---|
| `main` | **MeshUtility** — a shipped Tauri voice-dictation + prompt-enhancer app | Released, v1.0.10 |
| `cursor/windows-jarvis-overlay-ad3d` | **Olé desktop** (was "Jarvis overlay") — glowing orb, Living Dossier, **Zeus mailbox** | PR #1, open, not merged |
| `claude/android-keyboard-ai-suggestions-jfjst6` | **RizzBar** — Android overlay reply assistant, in `rizzbar-android/` | PR #2, open draft, never compiled |
| `Zclaw89/ole-android` (separate repo, private) | **Olé Android** | Empty — README only, 3 lines |

Do not merge these into each other. They are unrelated products that happen to
share a git history root.

---

## 2. Where everything is stored

### On disk (Windows, the desktop machine)

| Path | What |
|---|---|
| `%LOCALAPPDATA%\MeshVoice\meshvoice.sqlite` | MeshUtility history, settings, lifetime stats, pronunciation dictionary |
| `%LOCALAPPDATA%\MeshVoice\models\` | Downloaded Whisper `.bin` + the Parakeet bundle |
| `%LOCALAPPDATA%\com.meshpilot.meshutility\keys\<provider>.bin` | API keys, one file per provider, **encrypted with Windows DPAPI** (`CryptProtectData`) — not plaintext, not in git, and not portable to another machine or user account |
| `%LOCALAPPDATA%\com.meshpilot.meshutility\settings.json` | App settings JSON (separate from the SQLite `settings` table) |
| `Documents\OleMailbox\inbox\` + `outbox\` | **The Zeus mailbox — this is how you get work. See §3.** |
| `Documents\OleDossier\items\` | Living Dossier captures: screenshot + `meta.json` per item |
| `Documents\OleDossier\{ole_containers,source_artifacts,ole_artifact_links}\` | Local JSON "tables" mirroring the Android data model |
| `src-tauri\target\release\bundle\nsis\` | Built Windows installer lands here |

Nothing syncs to a cloud. No telemetry. Everything above is local to that PC.

### In git

- `Zclaw89/jarvis-overlay` — public fork. Upstream is `lazyshrey/MeshUtility`;
  releases for MeshUtility proper come from `MeshPilot-in/MeshUtility`.
- `Zclaw89/ole-android` — private, effectively empty.
- Never committed: `.env*`, `*.local`, `opencode.json`, `.mcp.json`,
  `meshmcp.local.ps1`, `src-tauri/target/`, whisper binaries over size,
  `.commandcode/`, `.meshmemory/`.

### Not verifiable from here (please confirm locally)

- The **Obsidian vault** (`Obsidian - Zeus`) — no reference to it exists
  anywhere in either repo. If notes/specs live there, they are *not* backed by
  git and nothing in the code reads them. Worth confirming what's in it before
  assuming it's the source of truth for anything.
- `Documents\BUILD_AND_HANDOFF.md` — a file by that name exists in the repo at
  `rizzbar-android/BUILD_AND_HANDOFF.md` on the RizzBar branch. The copy in
  Documents is probably that file, dragged out. Check whether it's stale before
  following it.

---

## 3. The Zeus mailbox — how you plug in as the brain

This is the part that matters most to you. Olé desktop has a **Brain** setting
with two modes (Settings → Brain):

- **API key (fast)** — calls a chat provider directly.
- **Zeus mailbox (screenshot + wait)** — writes files to disk and *waits for an
  external agent to answer*. That external agent is you.

"Zeus" is the role name for whatever local agent answers the mailbox. It is not
a service, not a daemon, not something that's running. It's a folder contract.

### The contract

When the user says "analyze this" / "what's on my screen?" in mailbox mode, the
app creates a job:

```
Documents\OleMailbox\
├── inbox\<job-id>\
│   ├── request.txt      what the user asked (plain text)
│   ├── screenshot.png   only if hasScreenshot is true
│   ├── meta.json        { id, createdAt, hasScreenshot }
│   └── done.json        written by the APP after it reads your reply
└── outbox\<job-id>\
    └── reply.txt        ← YOU WRITE THIS
```

`<job-id>` looks like `ole-20260911143022123-4821` (timestamp + PID).

**Your job:** watch `Documents\OleMailbox\inbox\`, find job folders with no
matching reply in `outbox\<same-id>\`, read `request.txt` (and the screenshot),
then write your answer to `outbox\<job-id>\reply.txt`.

Details that will bite you if you miss them:

- The app polls for **`reply.txt` first, then `reply.md`** — either works,
  nothing else does. `response.txt`, `out.txt`, `answer.md` are ignored forever.
- The outbox job folder **already exists** when the job is created. An empty
  outbox folder does not mean "already handled" — check for the reply file
  itself.
- You never write `done.json`. The app writes it into the *inbox* folder after
  it has successfully read your reply. Presence of `done.json` = that job is
  closed; skip it.
- The reply is read as **plain UTF-8 text** and shown to the user. No JSON
  envelope, no wrapper. Just write the answer.
- The app blocks/waits while the job is open, so latency is felt directly.

There is no queue, no lock file, and no retry. If two agents both write
`reply.txt`, last writer wins.

---

## 4. Project details

### MeshUtility (`main`) — the only shipped thing

Tauri 2 + React 19 + Rust. Tray-resident Windows app. Two features: voice
dictation (local whisper.cpp / sherpa-onnx Parakeet, or Groq Whisper cloud) and
a prompt enhancer overlay (Groq, OpenAI, Anthropic, Gemini, Mistral, or any
OpenAI-compatible endpoint).

- Frontend `src/` (Zustand store in `src/store/appStore.ts`), backend
  `src-tauri/src/` split into `main.rs`, `audio.rs`, `engine.rs`,
  `transcription.rs`, `injection.rs`, `clipboard.rs`, `db.rs`.
- Three windows: `main`, `widget` (the floating pill), `overlay`.
- Hotkeys: `Alt+Space` record, `Ctrl+Shift+Space` enhancer overlay.
- GPU: DirectML for Parakeet ONNX, Vulkan for whisper.cpp — the release
  workflow installs the Vulkan SDK on the Windows runner.
- Ships on tag push (`v*`) via `.github/workflows/release.yml`, Windows only.
- Bundle id `com.meshpilot.meshutility`, deep-link schemes `meshvoice://`,
  `meshprompt://`.

Dev: `npm install && npm run tauri dev` (dev server on **5170**).
Release: `npm run tauri build`.

Version bumps must be changed in **three** places or the updater misreports:
`package.json`, `src-tauri/tauri.conf.json`, and `latest-version.json`
(which also carries the changelog string and the release download URL).

### Olé desktop (`cursor/windows-jarvis-overlay-ad3d`, PR #1)

Started as "Jarvis overlay v1" (glowing always-on-top orb, compact chat,
Kokoro/OpenAI/ElevenLabs voice) and was then **pivoted to the Olé dossier**
model. Commit order tells the story: `Build Jarvis overlay v1` → `Add Zeus
mailbox brain loop` → `Pivot desktop app to Olé dossier` → `Lock Olé Android
model names` → `Add Olé lock remote gesture`.

- Docked badge; mouse-wheel over it switches mode: `OLE`, `SCREEN_RECORD`,
  `VOICE_RECORD`, `CUSTOM`.
- Click/long-press in OLE mode = screenshot → saved as a Living Dossier item.
- Drag a file onto Living Dossier → copied into Olé storage with a sha256 and
  `UNVERIFIED` source metadata.
- Voice defaults to **local Kokoro**, which self-downloads on first speak (no
  Python needed). Paid voices only activate once selected *and* keyed;
  otherwise it falls back to local.
- **AI analysis/report links are stubbed in v1.** Captures are saved first and
  no paid API is test-called automatically.
- Explicitly out of scope for v1: wake word, accounts/login, computer-use
  clicking, signed installer, always-on screen recorder, phone DRM metadata,
  ping-pong bounce, full patent graph/DAG orchestration.

### RizzBar (`claude/android-keyboard-ai-suggestions-jfjst6`, PR #2 draft)

Private sideload-only Android app in `rizzbar-android/`. Floating ✨ bubble over
any chat app; reads visible chat via the **Accessibility API — no screenshots,
no OCR**; labels lines Me/Them by bubble side; asks an OpenAI-compatible
endpoint (OpenRouter default) for N reply suggestions; taps insert into the
input field and **never auto-send**.

- 6 tone modes; per-person profiles with a **spice ceiling (1–5)** that
  overrides the mode.
- Key + profiles in `EncryptedSharedPreferences`, with a plain-prefs fallback if
  the keystore throws.
- Pins: AGP 8.5.2, Kotlin 2.0.20, compileSdk/targetSdk 35, minSdk 26.
- **The Gradle wrapper is deliberately not committed.** Android Studio generates
  it on first open; on CLI you need a system `gradle` once to run
  `gradle wrapper --gradle-version 8.9`, plus a `local.properties` with
  `sdk.dir=`.
- **This has never been compiled.** It was written without a local build, so
  expect a real error on first `assembleDebug`. `rizzbar-android/BUILD_AND_HANDOFF.md`
  has a troubleshooting section written for exactly this.

### Olé Android (`Zclaw89/ole-android`)

Private repo containing one file: a README saying *"Living OLE dossier. Overlay
is the capture door."* That's all that exists. The desktop branch was
deliberately aligned to Android's model names (`ole_containers`,
`source_artifacts`, `ole_artifact_links`) in anticipation of it, but no Android
code has been written.

---

## 5. Git conventions

- Feature work goes on a branch, never straight to `main`.
- Push with `git push -u origin <branch>`; open a **draft** PR.
- Branches in flight are listed in §1 — check whether a PR is already open
  before starting a new branch for the same thing.
- Both open PRs target `main` from base `982b138`, so both will need a merge
  from `main` if `main` moves.

---

## 6. Open threads / what's actually unfinished

1. **PR #1 (Olé desktop) and PR #2 (RizzBar) have both sat unmerged since
   mid/late August.** Neither is reviewed. Decide: merge, or let them live as
   long-running branches.
2. **RizzBar has never been built.** Highest-value next action if the user wants
   it on their phone: open `rizzbar-android/` in Android Studio, build, fix the
   first compile errors.
3. **Olé desktop's AI analysis is a stub.** The mailbox path (§3) is the working
   route; the direct-API analysis path is not wired up.
4. **Olé Android doesn't exist yet** beyond a README.
5. The three projects sharing one repo will get more painful as they diverge.
   Splitting RizzBar and Olé into their own repos is worth considering.

---

## 7. Honest limits of this document

Written from the two GitHub repos only. I could not see the desktop's local
filesystem, the Obsidian vault, any local uncommitted work, or anything in
`Documents` — so if the machine has work that was never pushed, it is not
described here and I don't know it exists. The Windows paths in §2 are read out
of the Rust source (`dirs::data_local_dir()`, `dirs::document_dir()`), so they
are what the code *will* use, not paths I confirmed on the machine.
