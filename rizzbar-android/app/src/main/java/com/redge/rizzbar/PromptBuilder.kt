package com.redge.rizzbar

/**
 * Turns (mode + profile + settings + scraped transcript) into the messages
 * sent to the chat-completions endpoint.
 */
object PromptBuilder {

    data class Request(
        val system: String,
        val user: String,
        val model: String,
        val temperature: Double,
        val count: Int,
    )

    fun build(
        mode: Mode,
        profile: Profile?,
        transcript: String,
        nudge: String? = null,
    ): Request {
        val count = SettingsStore.suggestionCount

        val length = when (SettingsStore.replyLength) {
            0 -> "Keep each reply to one short sentence — a punchy one-liner."
            1 -> "Keep each reply to 1-2 sentences."
            else -> "Replies can be 2-4 sentences when it helps."
        }

        val emoji = when (SettingsStore.emojiUse) {
            0 -> "Do not use emojis."
            1 -> "Use at most one emoji per reply, only when it lands."
            else -> "Match the other person's emoji energy."
        }

        val profileBlock = if (profile != null) buildString {
            appendLine("About the person I'm talking to:")
            if (profile.name.isNotBlank()) appendLine("- Name: ${profile.name}")
            if (profile.stage.isNotBlank()) appendLine("- Relationship stage: ${profile.stage}")
            if (profile.style.isNotBlank()) appendLine("- Their texting style: ${profile.style}")
            if (profile.notes.isNotBlank()) appendLine("- Context / inside jokes: ${profile.notes}")
            if (profile.avoid.isNotBlank()) appendLine("- Topics to avoid: ${profile.avoid}")
            appendLine(
                "- Spice ceiling for this person: ${profile.spiceCap}/5 " +
                    "(1 = fully innocent, 3 = suggestive but not explicit, 5 = no ceiling). " +
                    "Never exceed it, whatever the selected tone."
            )
        } else ""

        val system = buildString {
            appendLine(
                "You are my private texting assistant. You write reply options for ME to send " +
                    "in a one-on-one chat. Write in the first person, as me, matching how I talk " +
                    "in the transcript. Never mention being an AI, never add commentary."
            )
            appendLine()
            appendLine("Tone for these replies: ${mode.label}. ${mode.stylePrompt}")
            appendLine()
            if (profileBlock.isNotBlank()) {
                appendLine(profileBlock)
            }
            appendLine(length)
            appendLine(emoji)
            if (!nudge.isNullOrBlank()) {
                appendLine("Extra instruction for this round: $nudge")
            }
            appendLine()
            appendLine(
                "Output format: respond with ONLY a JSON array of exactly $count strings, " +
                    "each a distinct reply option. No markdown, no numbering, no other text."
            )
        }

        val user = buildString {
            appendLine("Here is the visible chat, oldest first. \"Me\" is me, \"Them\" is the other person:")
            appendLine()
            appendLine(transcript.ifBlank { "(no messages visible — open with something good)" })
            appendLine()
            append("Write $count reply options from Me.")
        }

        val model = if (mode == Mode.KINKY && SettingsStore.spicyModel.isNotBlank()) {
            SettingsStore.spicyModel
        } else {
            SettingsStore.model
        }

        return Request(system, user, model, mode.temperature, count)
    }
}
