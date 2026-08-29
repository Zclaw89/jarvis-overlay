# ✨ RizzBar

Private Android overlay assistant that reads the chat on screen (only when you
tap Reply), sends it to an AI endpoint you configure, and offers reply
suggestions you can insert into the text field — in any chat app. Personal
sideload build; not for the Play Store.

## How it works

- A draggable **✨ bubble** floats on screen while the accessibility service is
  enabled. Tap it to open the panel.
- The panel has **mode chips** (😈 Kinky · 😏 Tease · 😂 Funny · 🥰 Sweet ·
  🧊 Cool · ✍️ Neutral), a **profile selector** (tap 👤 to cycle through your
  profiles), and a **✨ Reply** button.
- Tapping Reply reads the *visible* chat text via the accessibility API (no
  screenshots, no OCR), labels lines Me/Them by bubble side, and asks your
  configured model for N reply options (N is a setting, default 3).
- Tap a suggestion → it's typed into the input field. **It never auto-sends** —
  you always press send yourself.
- Nudge buttons: 🔄 regenerate · shorter · spicier · softer.

Profiles carry a **spice ceiling (1–5)** that overrides whatever mode is
selected, so Kinky mode with a brand-new match stays suggestive, not explicit.

## Build

1. Open the `rizzbar-android/` folder in Android Studio (it will generate the
   Gradle wrapper on sync).
2. Build → Build APK, or run on your phone over USB with developer mode on.
3. Sideload the APK.

## Setup on the phone

1. Open **RizzBar** → paste your **OpenRouter API key** (or any
   OpenAI-compatible endpoint: change Base URL + model).
   - Optional: set a separate, more permissive model just for Kinky mode.
2. Tap **Enable RizzBar in Accessibility settings** and switch the service on.
3. The ✨ bubble appears. Open any chat, tap the bubble, pick a mode/profile,
   tap ✨ Reply.

## Privacy

- API key and profiles are stored in `EncryptedSharedPreferences` on-device.
- Chat text is sent **only** to the endpoint you configured, **only** at the
  moment you tap Reply. Nothing is read continuously, logged, or stored.
- No analytics, no telemetry, no network calls other than the one you trigger.

## Known limitations (v0.1)

- Generic scraping can occasionally mislabel Me/Them in unusual chat layouts.
- Only messages currently visible on screen are included — scroll up first if
  you want more context.
- Some apps with flagged-secure or custom-drawn chat views may expose no text
  to accessibility; those would need a screenshot+OCR fallback (not built).
