package com.redge.rizzbar

import android.accessibilityservice.AccessibilityService
import android.graphics.Rect
import android.os.Bundle
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel

/**
 * The engine behind the overlay bar. Reads the visible chat ONLY when the user
 * taps Reply, and inserts the picked suggestion into the focused text field.
 * It never sends messages and never reads the screen in the background.
 */
class RizzAccessibilityService : AccessibilityService() {

    val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var overlay: OverlayController? = null

    override fun onServiceConnected() {
        super.onServiceConnected()
        SettingsStore.init(this)
        overlay = OverlayController(this).also { it.showBubble() }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        // Events are only used to keep the service alive; all reading is
        // explicitly user-triggered from the overlay's Reply button.
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        overlay?.destroy()
        overlay = null
        serviceScope.cancel()
        super.onDestroy()
    }

    // ------------------------------------------------------------------
    // Chat scraping
    // ------------------------------------------------------------------

    private data class Snippet(val text: String, val bounds: Rect)

    /**
     * Generic any-app scrape: collect visible, non-editable text nodes in the
     * active window, order them top-to-bottom, and label each line Me/Them by
     * whether its bubble sits on the right or left half of the screen.
     */
    fun scrapeChat(): String {
        val root = rootInActiveWindow ?: return ""
        val screenWidth = resources.displayMetrics.widthPixels
        val screenHeight = resources.displayMetrics.heightPixels
        val snippets = mutableListOf<Snippet>()
        collectText(root, snippets, depth = 0)

        val timeRegex = Regex("^\\d{1,2}[:.]\\d{2}(\\s?[APap][Mm])?$")
        val lines = snippets
            .asSequence()
            .filter { it.text.isNotBlank() }
            .filter { !timeRegex.matches(it.text.trim()) }
            // skip the app's top chrome (contact name, status bar text)
            .filter { it.bounds.top > screenHeight * 0.08 }
            .sortedWith(compareBy({ it.bounds.top }, { it.bounds.left }))
            .map {
                val center = (it.bounds.left + it.bounds.right) / 2
                val who = if (center > screenWidth / 2) "Me" else "Them"
                "$who: ${it.text.trim()}"
            }
            .toList()

        val limited = if (SettingsStore.contextAmount == 0) lines.takeLast(10) else lines
        var transcript = limited.joinToString("\n")
        if (transcript.length > 6000) transcript = transcript.takeLast(6000)
        return transcript
    }

    private fun collectText(node: AccessibilityNodeInfo?, out: MutableList<Snippet>, depth: Int) {
        if (node == null || depth > 60) return
        try {
            if (node.isVisibleToUser && !node.isEditable) {
                val cls = node.className?.toString().orEmpty()
                val isButton = cls.contains("Button", ignoreCase = true)
                val text = node.text?.toString()
                if (!text.isNullOrBlank() && !isButton) {
                    val r = Rect()
                    node.getBoundsInScreen(r)
                    if (r.height() > 0 && r.width() > 0) out.add(Snippet(text, r))
                }
            }
            for (i in 0 until node.childCount) {
                collectText(node.getChild(i), out, depth + 1)
            }
        } catch (_: Exception) {
            // node can go stale mid-walk; skip it
        }
    }

    // ------------------------------------------------------------------
    // Inserting the chosen reply (never sends — user presses send)
    // ------------------------------------------------------------------

    fun insertText(text: String): Boolean {
        val root = rootInActiveWindow ?: return false
        val target = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT)
            ?: findEditable(root, 0)
            ?: return false
        val args = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
        }
        return target.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
    }

    private fun findEditable(node: AccessibilityNodeInfo?, depth: Int): AccessibilityNodeInfo? {
        if (node == null || depth > 60) return null
        if (node.isEditable && node.isVisibleToUser) return node
        for (i in 0 until node.childCount) {
            findEditable(node.getChild(i), depth + 1)?.let { return it }
        }
        return null
    }
}
