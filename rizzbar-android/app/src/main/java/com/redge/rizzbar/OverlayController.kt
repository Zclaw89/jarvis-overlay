package com.redge.rizzbar

import android.annotation.SuppressLint
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import kotlinx.coroutines.launch

/**
 * The floating ✨ bubble and the suggestion panel, drawn as accessibility
 * overlays (visible above the keyboard, no extra permission needed).
 */
class OverlayController(private val service: RizzAccessibilityService) {

    private val wm = service.getSystemService(WindowManager::class.java)
    private val density = service.resources.displayMetrics.density
    private fun dp(v: Int): Int = (v * density).toInt()

    private var bubble: TextView? = null
    private var panel: LinearLayout? = null

    private var currentMode: Mode = SettingsStore.defaultMode
    private var currentProfile: Profile? = ProfileStore.selected()
    private var lastTranscript: String = ""
    private var busy = false

    // panel widgets we update
    private var statusView: TextView? = null
    private var suggestionsBox: LinearLayout? = null
    private var nudgeRow: LinearLayout? = null
    private var profileButton: TextView? = null
    private val modeChips = mutableMapOf<Mode, TextView>()

    // ------------------------------------------------------------------
    // Bubble
    // ------------------------------------------------------------------

    @SuppressLint("ClickableViewAccessibility")
    fun showBubble() {
        if (bubble != null) return
        val b = TextView(service).apply {
            text = "✨"
            textSize = 22f
            gravity = Gravity.CENTER
            background = pill(Color.parseColor("#CC1E1B4B"), 24)
        }
        val lp = overlayParams(dp(48), dp(48)).apply {
            gravity = Gravity.TOP or Gravity.START
            x = SettingsStore.bubbleX
            y = SettingsStore.bubbleY
        }

        var downX = 0f; var downY = 0f; var startX = 0; var startY = 0; var moved = false
        b.setOnTouchListener { v, ev ->
            when (ev.action) {
                MotionEvent.ACTION_DOWN -> {
                    downX = ev.rawX; downY = ev.rawY; startX = lp.x; startY = lp.y; moved = false
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    val dx = (ev.rawX - downX).toInt(); val dy = (ev.rawY - downY).toInt()
                    if (Math.abs(dx) > dp(4) || Math.abs(dy) > dp(4)) moved = true
                    lp.x = startX + dx; lp.y = startY + dy
                    wm.updateViewLayout(v, lp)
                    true
                }
                MotionEvent.ACTION_UP -> {
                    if (moved) {
                        SettingsStore.bubbleX = lp.x
                        SettingsStore.bubbleY = lp.y
                    } else {
                        togglePanel()
                    }
                    true
                }
                else -> false
            }
        }
        wm.addView(b, lp)
        bubble = b
    }

    // ------------------------------------------------------------------
    // Panel
    // ------------------------------------------------------------------

    private fun togglePanel() {
        if (panel != null) hidePanel() else showPanel()
    }

    private fun showPanel() {
        // refresh state that may have changed in the settings app
        currentProfile = ProfileStore.selected()

        val root = LinearLayout(service).apply {
            orientation = LinearLayout.VERTICAL
            background = pill(Color.parseColor("#F21E1B4B"), 18)
            setPadding(dp(12), dp(10), dp(12), dp(10))
        }

        // --- mode chips row ---
        val chipRow = LinearLayout(service).apply { orientation = LinearLayout.HORIZONTAL }
        modeChips.clear()
        Mode.entries.forEach { mode ->
            val chip = TextView(service).apply {
                text = "${mode.emoji} ${mode.label}"
                textSize = 13f
                setTextColor(Color.WHITE)
                setPadding(dp(10), dp(6), dp(10), dp(6))
                setOnClickListener {
                    currentMode = mode
                    refreshChips()
                }
            }
            modeChips[mode] = chip
            val clp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { rightMargin = dp(6) }
            chipRow.addView(chip, clp)
        }
        refreshChips()
        val chipScroll = HorizontalScrollView(service).apply {
            isHorizontalScrollBarEnabled = false
            addView(chipRow)
        }
        root.addView(chipScroll)

        // --- profile + reply + close row ---
        val actionRow = LinearLayout(service).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }
        profileButton = TextView(service).apply {
            textSize = 13f
            setTextColor(Color.parseColor("#C7D2FE"))
            setPadding(dp(10), dp(8), dp(10), dp(8))
            background = pill(Color.parseColor("#33FFFFFF"), 14)
            setOnClickListener { cycleProfile() }
        }
        updateProfileButton()
        val replyBtn = TextView(service).apply {
            text = "✨ Reply"
            textSize = 15f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            setPadding(dp(16), dp(8), dp(16), dp(8))
            background = pill(Color.parseColor("#7C3AED"), 16)
            setOnClickListener { generate(nudge = null) }
        }
        val closeBtn = TextView(service).apply {
            text = "✕"
            textSize = 15f
            setTextColor(Color.parseColor("#A5B4FC"))
            setPadding(dp(10), dp(8), dp(6), dp(8))
            setOnClickListener { hidePanel() }
        }
        actionRow.addView(profileButton, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
        actionRow.addView(replyBtn, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { leftMargin = dp(8) })
        actionRow.addView(closeBtn, LinearLayout.LayoutParams(LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT).apply { leftMargin = dp(4) })
        root.addView(actionRow, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(8) })

        // --- status line ---
        statusView = TextView(service).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#A5B4FC"))
            visibility = View.GONE
        }
        root.addView(statusView, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(6) })

        // --- suggestions ---
        suggestionsBox = LinearLayout(service).apply { orientation = LinearLayout.VERTICAL }
        root.addView(suggestionsBox)

        // --- nudge row (appears once suggestions exist) ---
        nudgeRow = LinearLayout(service).apply {
            orientation = LinearLayout.HORIZONTAL
            visibility = View.GONE
        }
        listOf(
            "🔄" to null,
            "shorter" to "Make them shorter and punchier.",
            "spicier" to "Turn the heat up a notch (still within the spice ceiling).",
            "softer" to "Make them softer and sweeter.",
        ).forEach { (label, nudge) ->
            val btn = TextView(service).apply {
                text = label
                textSize = 13f
                setTextColor(Color.WHITE)
                setPadding(dp(10), dp(6), dp(10), dp(6))
                background = pill(Color.parseColor("#33FFFFFF"), 14)
                setOnClickListener { generate(nudge) }
            }
            nudgeRow?.addView(btn, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { rightMargin = dp(6); topMargin = dp(8) })
        }
        root.addView(nudgeRow)

        val lp = overlayParams(
            service.resources.displayMetrics.widthPixels - dp(16),
            WindowManager.LayoutParams.WRAP_CONTENT,
        ).apply {
            gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
            y = dp(64)
        }
        wm.addView(root, lp)
        panel = root
    }

    private fun hidePanel() {
        panel?.let { runCatching { wm.removeView(it) } }
        panel = null
        statusView = null
        suggestionsBox = null
        nudgeRow = null
        profileButton = null
        modeChips.clear()
    }

    fun destroy() {
        hidePanel()
        bubble?.let { runCatching { wm.removeView(it) } }
        bubble = null
    }

    // ------------------------------------------------------------------
    // Generation flow
    // ------------------------------------------------------------------

    private fun generate(nudge: String?) {
        if (busy) return
        busy = true
        setStatus("Reading chat…")
        suggestionsBox?.removeAllViews()
        nudgeRow?.visibility = View.GONE

        // Fresh scrape on a normal Reply; nudges reuse the same transcript.
        if (nudge == null || lastTranscript.isBlank()) {
            lastTranscript = service.scrapeChat()
        }
        val req = PromptBuilder.build(currentMode, currentProfile, lastTranscript, nudge)

        setStatus("Thinking… (${currentMode.emoji} ${currentMode.label}, ${req.model})")
        service.serviceScope.launch {
            try {
                val options = ApiClient.suggestions(req)
                showSuggestions(options)
                setStatus(null)
            } catch (e: Exception) {
                setStatus(e.message ?: "Something went wrong.")
            } finally {
                busy = false
            }
        }
    }

    private fun showSuggestions(options: List<String>) {
        val box = suggestionsBox ?: return
        box.removeAllViews()
        options.forEach { option ->
            val card = TextView(service).apply {
                text = option
                textSize = 14f
                setTextColor(Color.WHITE)
                setPadding(dp(12), dp(10), dp(12), dp(10))
                background = pill(Color.parseColor("#3B3663"), 14)
                setOnClickListener {
                    val ok = service.insertText(option)
                    if (ok) hidePanel() else setStatus("Couldn't find the text field — tap into it first.")
                }
            }
            box.addView(card, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(8) })
        }
        nudgeRow?.visibility = View.VISIBLE
    }

    private fun setStatus(msg: String?) {
        statusView?.apply {
            if (msg == null) {
                visibility = View.GONE
            } else {
                text = msg
                visibility = View.VISIBLE
            }
        }
    }

    // ------------------------------------------------------------------
    // Small helpers
    // ------------------------------------------------------------------

    private fun refreshChips() {
        modeChips.forEach { (mode, chip) ->
            chip.background = if (mode == currentMode) {
                pill(Color.parseColor("#7C3AED"), 14)
            } else {
                pill(Color.parseColor("#33FFFFFF"), 14)
            }
        }
    }

    private fun cycleProfile() {
        val profiles = ProfileStore.load()
        val options: List<Profile?> = listOf(null) + profiles
        val idx = options.indexOfFirst { it?.id == currentProfile?.id }
        currentProfile = options[(idx + 1) % options.size]
        SettingsStore.selectedProfileId = currentProfile?.id ?: ""
        updateProfileButton()
    }

    private fun updateProfileButton() {
        profileButton?.text = "👤 " + (currentProfile?.name?.ifBlank { "Unnamed" } ?: "No profile")
    }

    private fun pill(color: Int, radiusDp: Int): GradientDrawable =
        GradientDrawable().apply {
            setColor(color)
            cornerRadius = dp(radiusDp).toFloat()
        }

    private fun overlayParams(w: Int, h: Int) = WindowManager.LayoutParams(
        w, h,
        WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
        WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
        PixelFormat.TRANSLUCENT,
    )
}
