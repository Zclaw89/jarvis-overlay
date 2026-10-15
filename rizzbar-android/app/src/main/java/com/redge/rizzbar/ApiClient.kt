package com.redge.rizzbar

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.TimeUnit

/**
 * Minimal client for any OpenAI-compatible chat-completions endpoint
 * (OpenRouter by default). The only data transmitted is the prompt built
 * from the visible chat at the moment Reply is tapped.
 */
object ApiClient {

    class ApiException(message: String) : Exception(message)

    private val http = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .build()

    private val JSON = "application/json; charset=utf-8".toMediaType()

    suspend fun suggestions(req: PromptBuilder.Request): List<String> =
        withContext(Dispatchers.IO) {
            val key = SettingsStore.apiKey
            if (key.isBlank()) throw ApiException("No API key set. Open the RizzBar app and add one.")

            val body = JSONObject().apply {
                put("model", req.model)
                put("temperature", req.temperature)
                put("messages", JSONArray().apply {
                    put(JSONObject().put("role", "system").put("content", req.system))
                    put(JSONObject().put("role", "user").put("content", req.user))
                })
            }

            val httpReq = Request.Builder()
                .url("${SettingsStore.baseUrl}/chat/completions")
                .addHeader("Authorization", "Bearer $key")
                .addHeader("Content-Type", "application/json")
                .post(body.toString().toRequestBody(JSON))
                .build()

            val text = http.newCall(httpReq).execute().use { resp ->
                val payload = resp.body?.string().orEmpty()
                if (!resp.isSuccessful) {
                    throw ApiException("API error ${resp.code}: ${payload.take(300)}")
                }
                payload
            }

            val content = try {
                JSONObject(text)
                    .getJSONArray("choices")
                    .getJSONObject(0)
                    .getJSONObject("message")
                    .getString("content")
            } catch (e: Exception) {
                throw ApiException("Unexpected API response: ${text.take(300)}")
            }

            parseSuggestions(content, req.count)
        }

    /** The model is asked for a JSON array; fall back to line-splitting if it rambles. */
    internal fun parseSuggestions(content: String, count: Int): List<String> {
        val trimmed = content.trim()
            .removePrefix("```json").removePrefix("```").removeSuffix("```").trim()

        // Preferred path: a JSON array of strings, possibly embedded in other text.
        val start = trimmed.indexOf('[')
        val end = trimmed.lastIndexOf(']')
        if (start >= 0 && end > start) {
            try {
                val arr = JSONArray(trimmed.substring(start, end + 1))
                val out = mutableListOf<String>()
                for (i in 0 until arr.length()) {
                    val s = arr.optString(i).trim()
                    if (s.isNotEmpty()) out.add(s)
                }
                if (out.isNotEmpty()) return out.take(count)
            } catch (_: Exception) {
                // fall through to line splitting
            }
        }

        // Fallback: strip list markers off non-empty lines.
        val lines = trimmed.lines()
            .map { it.trim().trimStart('-', '*', '•').trim() }
            .map { it.replace(Regex("^\\d+[.)]\\s*"), "") }
            .map { it.trim('"') }
            .filter { it.isNotBlank() }
        if (lines.isEmpty()) throw ApiException("Model returned no usable suggestions.")
        return lines.take(count)
    }
}
