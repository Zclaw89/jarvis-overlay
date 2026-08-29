package com.redge.rizzbar

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * All user settings, stored in EncryptedSharedPreferences (hardware-backed keystore).
 * Nothing here ever leaves the device except the API key in the Authorization
 * header of requests to the endpoint the user configured.
 */
object SettingsStore {

    const val DEFAULT_BASE_URL = "https://openrouter.ai/api/v1"
    const val DEFAULT_MODEL = "openai/gpt-4o-mini"

    private lateinit var prefs: SharedPreferences

    fun init(context: Context) {
        if (::prefs.isInitialized) return
        val appContext = context.applicationContext
        prefs = try {
            val masterKey = MasterKey.Builder(appContext)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()
            EncryptedSharedPreferences.create(
                appContext,
                "rizzbar_secure",
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
            )
        } catch (e: Exception) {
            // Keystore can be corrupted after a backup/restore; fall back to plain
            // prefs rather than crashing a private, single-user device app.
            appContext.getSharedPreferences("rizzbar_plain", Context.MODE_PRIVATE)
        }
    }

    var apiKey: String
        get() = prefs.getString("api_key", "") ?: ""
        set(v) = prefs.edit().putString("api_key", v.trim()).apply()

    var baseUrl: String
        get() = prefs.getString("base_url", DEFAULT_BASE_URL) ?: DEFAULT_BASE_URL
        set(v) = prefs.edit().putString("base_url", v.trim().trimEnd('/')).apply()

    var model: String
        get() = prefs.getString("model", DEFAULT_MODEL) ?: DEFAULT_MODEL
        set(v) = prefs.edit().putString("model", v.trim()).apply()

    /** Optional different model used only for Kinky mode (e.g. a permissive model). */
    var spicyModel: String
        get() = prefs.getString("spicy_model", "") ?: ""
        set(v) = prefs.edit().putString("spicy_model", v.trim()).apply()

    /** How many suggestions per Reply tap (1..5). */
    var suggestionCount: Int
        get() = prefs.getInt("suggestion_count", 3).coerceIn(1, 5)
        set(v) = prefs.edit().putInt("suggestion_count", v.coerceIn(1, 5)).apply()

    /** 0 = one-liner, 1 = normal, 2 = longer. */
    var replyLength: Int
        get() = prefs.getInt("reply_length", 0).coerceIn(0, 2)
        set(v) = prefs.edit().putInt("reply_length", v.coerceIn(0, 2)).apply()

    /** 0 = none, 1 = light, 2 = match their energy. */
    var emojiUse: Int
        get() = prefs.getInt("emoji_use", 2).coerceIn(0, 2)
        set(v) = prefs.edit().putInt("emoji_use", v.coerceIn(0, 2)).apply()

    /** 0 = last ~10 messages, 1 = everything visible on screen. */
    var contextAmount: Int
        get() = prefs.getInt("context_amount", 1).coerceIn(0, 1)
        set(v) = prefs.edit().putInt("context_amount", v.coerceIn(0, 1)).apply()

    var defaultMode: Mode
        get() = Mode.fromName(prefs.getString("default_mode", Mode.NEUTRAL.name))
        set(v) = prefs.edit().putString("default_mode", v.name).apply()

    var selectedProfileId: String
        get() = prefs.getString("selected_profile", "") ?: ""
        set(v) = prefs.edit().putString("selected_profile", v).apply()

    var profilesJson: String
        get() = prefs.getString("profiles", "[]") ?: "[]"
        set(v) = prefs.edit().putString("profiles", v).apply()

    // Remembered overlay bubble position.
    var bubbleX: Int
        get() = prefs.getInt("bubble_x", 0)
        set(v) = prefs.edit().putInt("bubble_x", v).apply()

    var bubbleY: Int
        get() = prefs.getInt("bubble_y", 400)
        set(v) = prefs.edit().putInt("bubble_y", v).apply()
}
