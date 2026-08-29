package com.redge.rizzbar

enum class Mode(
    val label: String,
    val emoji: String,
    val temperature: Double,
    val stylePrompt: String,
) {
    KINKY(
        "Kinky", "😈", 0.95,
        "Bold, sexually charged and forward. Confident, explicit-leaning, escalates the tension — " +
            "but never exceed the spice ceiling set for this person. If the ceiling is low, stay " +
            "suggestive and leave things to the imagination instead of being explicit."
    ),
    TEASING(
        "Tease", "😏", 0.9,
        "Playful push-pull. A little withholding, keeps them chasing, never fully commits. " +
            "Light challenges and cheeky comebacks over compliments."
    ),
    FUNNY(
        "Funny", "😂", 0.9,
        "Witty and a bit absurd. Use callbacks to things said earlier in this chat, playful " +
            "exaggeration, and unexpected turns. Land the joke, don't explain it."
    ),
    SWEET(
        "Sweet", "🥰", 0.7,
        "Warm, affectionate, sincere. Make them feel noticed and special without being sappy " +
            "or over the top."
    ),
    COOL(
        "Cool", "🧊", 0.7,
        "Short, unbothered, high-value energy. Minimal-effort vibe while still clearly engaged. " +
            "No exclamation marks, no chasing."
    ),
    NEUTRAL(
        "Neutral", "✍️", 0.7,
        "A natural, good reply that matches the other person's tone and energy."
    );

    companion object {
        fun fromName(name: String?): Mode =
            entries.firstOrNull { it.name == name } ?: NEUTRAL
    }
}
