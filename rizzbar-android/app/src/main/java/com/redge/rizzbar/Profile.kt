package com.redge.rizzbar

import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

/**
 * Who you're talking to. The active profile is baked into every prompt, so
 * suggestions respect the relationship stage and this person's spice ceiling
 * regardless of which mode chip is selected.
 */
data class Profile(
    val id: String = UUID.randomUUID().toString(),
    var name: String = "",
    var stage: String = "",          // e.g. "new match", "dating 2 months", "my girlfriend"
    var notes: String = "",          // inside jokes, running bits, things she's into
    var avoid: String = "",          // topics to stay away from
    var spiceCap: Int = 2,           // 1..5 ceiling, overrides mode
    var style: String = "",          // her texting style: short/long, emoji-heavy, dry...
) {
    fun toJson(): JSONObject = JSONObject().apply {
        put("id", id)
        put("name", name)
        put("stage", stage)
        put("notes", notes)
        put("avoid", avoid)
        put("spiceCap", spiceCap)
        put("style", style)
    }

    companion object {
        fun fromJson(o: JSONObject): Profile = Profile(
            id = o.optString("id", UUID.randomUUID().toString()),
            name = o.optString("name"),
            stage = o.optString("stage"),
            notes = o.optString("notes"),
            avoid = o.optString("avoid"),
            spiceCap = o.optInt("spiceCap", 2).coerceIn(1, 5),
            style = o.optString("style"),
        )
    }
}

object ProfileStore {

    fun load(): MutableList<Profile> {
        val out = mutableListOf<Profile>()
        try {
            val arr = JSONArray(SettingsStore.profilesJson)
            for (i in 0 until arr.length()) {
                out.add(Profile.fromJson(arr.getJSONObject(i)))
            }
        } catch (_: Exception) {
            // corrupted store -> start fresh rather than crash
        }
        return out
    }

    fun save(profiles: List<Profile>) {
        val arr = JSONArray()
        profiles.forEach { arr.put(it.toJson()) }
        SettingsStore.profilesJson = arr.toString()
    }

    fun selected(): Profile? {
        val id = SettingsStore.selectedProfileId
        if (id.isEmpty()) return null
        return load().firstOrNull { it.id == id }
    }
}
