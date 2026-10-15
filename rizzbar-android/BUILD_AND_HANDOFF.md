# RizzBar — Build & Handoff Guide (for Claude Code Desktop)

**Read this first.** This file tells a Claude Code (or any dev) running on a
local machine exactly what this project is, how to turn it into an installable
Android APK, and what to do when something fails. Hand this whole file to
Claude Code Desktop and say: *"Build this into an APK I can install on my
Android phone, following BUILD_AND_HANDOFF.md. Fix any compile errors you
hit."*

---

## 1. What this project is

RizzBar is a **private, personal Android app** (sideload only — never going on
the Play Store). It is an **AI texting assistant that overlays on top of your
normal keyboard**. It works like the "Casanova AI / Rizz assistant" apps: you're
in any chat (WhatsApp, Tinder, SMS, Telegram, etc.), you tap a floating ✨
button, it reads the conversation on screen, and it suggests replies in a tone
you pick. Tap a suggestion and it types into the message box — you still press
send yourself.

**It is NOT** a rewrite of the desktop MeshUtility app that lives in the rest of
this repo. It is a standalone Android project that happens to live in the
`rizzbar-android/` subfolder. Everything RizzBar needs is inside that folder.

### How it works technically
- **Overlay UI** (a floating ✨ bubble + a suggestion panel) drawn as an
  accessibility overlay, so it floats above your existing keyboard. No custom
  keyboard needed.
- **An Android Accessibility Service** does two things, *only when you tap
  Reply*: (1) reads the visible chat text off the screen (real text via the
  accessibility tree — **no screenshots, no OCR**), labelling each line "Me" or
  "Them" by which side of the screen the bubble is on; (2) inserts the reply you
  pick into the focused text field. It never sends and never reads in the
  background.
- **API layer**: sends the chat + your selected tone/profile to any
  OpenAI-compatible chat-completions endpoint (defaults to OpenRouter) and gets
  back N reply suggestions.
- **Settings app** (the launcher icon): paste API key, choose model, set number
  of suggestions, tones, and per-person profiles.

### Features already implemented (v0.1)
- Floating draggable ✨ bubble + suggestion panel.
- 6 tone **modes**: 😈 Kinky, 😏 Tease, 😂 Funny, 🥰 Sweet, 🧊 Cool, ✍️ Neutral.
- Per-person **profiles**: name, relationship stage, texting style, inside
  jokes, topics to avoid, and a **spice ceiling (1–5)** that overrides the mode
  (so Kinky mode with a new match stays suggestive, not explicit).
- **Configurable suggestion count (1–5, default 3)**, reply length, emoji use,
  context window, default mode — all in Settings, no rebuild needed.
- API key stored in `EncryptedSharedPreferences` (on-device, encrypted).
- Nudge buttons on results: 🔄 regenerate · shorter · spicier · softer.

---

## 2. File map (what each file does)

```
rizzbar-android/
├── settings.gradle.kts            Gradle project definition
├── build.gradle.kts               Top-level Gradle (plugin versions)
├── gradle.properties              Gradle/AndroidX flags
├── app/
│   ├── build.gradle.kts           App module: SDK versions, dependencies
│   └── src/main/
│       ├── AndroidManifest.xml     Declares the activity + accessibility service
│       ├── java/com/redge/rizzbar/
│       │   ├── MainActivity.kt              Settings screen + profile editor
│       │   ├── SettingsStore.kt             Encrypted settings storage
│       │   ├── Mode.kt                       The 6 tones + their prompts
│       │   ├── Profile.kt                    Profile data + JSON store
│       │   ├── PromptBuilder.kt              Builds the system/user prompt
│       │   ├── ApiClient.kt                  Calls the chat-completions endpoint
│       │   ├── RizzAccessibilityService.kt   Reads chat + inserts reply
│       │   └── OverlayController.kt          The floating bubble + panel UI
│       └── res/                    Layouts, strings, theme, launcher icon
└── README.md                       User-facing overview
```

---

## 3. Prerequisites (install on the local machine)

1. **Android Studio** (latest stable) — the easiest path. Download from
   developer.android.com/studio. It bundles the Android SDK and JDK.
   - OR, headless: a **JDK 17** + the **Android command-line tools** + SDK
     packages `platforms;android-35`, `build-tools;35.0.0`, `platform-tools`.
2. An **Android phone** with **Developer options** and **USB debugging** on
   (Settings → About phone → tap Build number 7×, then Settings → Developer
   options → USB debugging). Or just build the APK and copy it to the phone.

---

## 4. Build it — Option A: Android Studio (recommended)

1. Open Android Studio → **File → Open** → select the `rizzbar-android/` folder
   (open that folder specifically, not the repo root).
2. Let it sync Gradle. On first sync it will **download the Gradle wrapper and
   dependencies** and may prompt to install the matching SDK — accept.
3. **Build → Build Bundle(s) / APK(s) → Build APK(s)**.
4. When it finishes, click **locate** — the file is
   `app/build/outputs/apk/debug/app-debug.apk`.
5. Install it: either **Run ▶** with the phone plugged in, or copy that `.apk`
   to the phone and tap it (allow "install from unknown sources").

---

## 5. Build it — Option B: command line (headless)

From inside `rizzbar-android/`:

```bash
# One-time: generate the Gradle wrapper if it's not present
#   (Android Studio does this automatically; on CLI you need Gradle installed,
#    or copy a gradle/ wrapper folder from another Android project.)
gradle wrapper --gradle-version 8.9

# Point Gradle at your SDK (create local.properties):
echo "sdk.dir=/path/to/Android/Sdk" > local.properties

# Build the debug APK:
./gradlew assembleDebug

# Result:
#   app/build/outputs/apk/debug/app-debug.apk

# Install to a connected phone:
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

> Note: the Gradle **wrapper** (`gradlew`, `gradle/wrapper/`) is intentionally
> not committed. Android Studio generates it on first open. On pure CLI you need
> a system `gradle` once to run `gradle wrapper`, after which `./gradlew` works.

---

## 6. Set it up on the phone (after install)

1. Open **RizzBar**.
2. Paste your **OpenRouter API key** (get one at openrouter.ai → Keys).
   - Base URL defaults to `https://openrouter.ai/api/v1`, model to
     `openai/gpt-4o-mini`. To use OpenAI/Anthropic/local instead, change Base
     URL + model.
   - **For Kinky mode**: mainstream models tone down or refuse explicit content.
     Set the optional **"Kinky-mode model override"** to a permissive model
     (e.g. an uncensored model on OpenRouter) if that matters to you.
3. Set **Suggestions per reply** (default 3) and the other behavior options.
4. Add one or more **Profiles** (one per person), set each one's spice ceiling.
5. Tap **Enable RizzBar in Accessibility settings** → turn the service ON.
6. Go to any chat. Tap the floating **✨** bubble → pick a mode + profile →
   tap **✨ Reply** → tap a suggestion to drop it in the box → press send.

---

## 7. If the build fails — how to fix

This is v0.1 written without a local compile, so a small error is possible.
General approach for Claude Code Desktop:

- **Read the exact Gradle/Kotlin error** and fix the named file. The code is
  plain Kotlin + Android Views (no Compose), using AndroidX, Material, OkHttp,
  kotlinx-coroutines, and androidx.security-crypto — all standard.
- **Dependency version not found** → bump to the nearest available stable
  version in `app/build.gradle.kts`. Current pins:
  - AGP `8.5.2`, Kotlin `2.0.20`, compileSdk/targetSdk `35`, minSdk `26`.
  - If AGP/Kotlin/SDK mismatch, align them to what the installed Android Studio
    ships (Studio will usually offer an "AGP Upgrade Assistant").
- **`security-crypto` alpha issue** → it's `1.1.0-alpha06`; if unavailable, use
  `1.0.0`. `SettingsStore` already has a fallback to plain prefs if the keystore
  throws, so encryption problems won't crash the app.
- **Overlay not showing** → the accessibility service must be enabled (step 5);
  the bubble is created in `RizzAccessibilityService.onServiceConnected()`.
- **"Couldn't find the text field"** when inserting → tap into the chat's input
  box once so it's focused, then pick the suggestion.
- **Me/Them reversed or messy in some app** → tuning lives in
  `RizzAccessibilityService.scrapeChat()` (it splits by screen center-x).

After fixing, rebuild and reinstall the APK.

---

## 8. Where to take it next (planned)

- **v0.2**: edit each mode's prompt text inside the app; nicer results UI;
  swipeable suggestion cards.
- **v0.3**: auto-detect which profile applies from the app + contact on screen.
- **v0.4 (optional)**: a real custom keyboard (IME) with the bar built in, for
  people who'd rather replace their keyboard than use an overlay.
- Possible fallback: **screenshot + on-device OCR** for apps that expose no
  accessibility text (e.g. FLAG_SECURE screens like Snapchat).

---

## 9. Privacy summary (state this to the user)

- Everything is local. The API key and profiles live in encrypted storage on the
  phone.
- Chat text is transmitted **only** to the endpoint you configured, **only** at
  the moment you tap Reply. Nothing is logged, stored, or sent anywhere else.
- No analytics, no telemetry, no background screen reading.
- Because it's a private sideload, there's no Play Store review limiting the
  Kinky mode — but the *AI provider's* content policy still applies to whatever
  model you point it at. That's why the per-mode model override exists.
