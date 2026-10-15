package com.redge.rizzbar

import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.view.LayoutInflater
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

class MainActivity : AppCompatActivity() {

    private lateinit var profiles: MutableList<Profile>

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        SettingsStore.init(this)
        setContentView(R.layout.activity_main)

        findViewById<Button>(R.id.enableServiceBtn).setOnClickListener {
            startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }

        // API fields
        findViewById<EditText>(R.id.apiKey).setText(SettingsStore.apiKey)
        findViewById<EditText>(R.id.baseUrl).setText(SettingsStore.baseUrl)
        findViewById<EditText>(R.id.model).setText(SettingsStore.model)
        findViewById<EditText>(R.id.spicyModel).setText(SettingsStore.spicyModel)

        // Behavior spinners
        setupSpinner(R.id.suggestionCount, listOf("1", "2", "3", "4", "5"), SettingsStore.suggestionCount - 1)
        setupSpinner(R.id.replyLength, listOf("One-liner", "Normal", "Longer"), SettingsStore.replyLength)
        setupSpinner(R.id.emojiUse, listOf("None", "Light", "Match their energy"), SettingsStore.emojiUse)
        setupSpinner(R.id.contextAmount, listOf("Last ~10 messages", "Everything visible"), SettingsStore.contextAmount)
        setupSpinner(
            R.id.defaultMode,
            Mode.entries.map { "${it.emoji} ${it.label}" },
            Mode.entries.indexOf(SettingsStore.defaultMode),
        )

        profiles = ProfileStore.load()
        renderProfiles()
        findViewById<Button>(R.id.addProfileBtn).setOnClickListener { editProfile(null) }

        findViewById<Button>(R.id.saveBtn).setOnClickListener { saveAll() }
    }

    override fun onResume() {
        super.onResume()
        val enabled = isServiceEnabled()
        findViewById<TextView>(R.id.serviceStatus).text = if (enabled) {
            "Service: enabled ✅ — the ✨ bubble is on screen."
        } else {
            "Service: OFF — enable it below, then the ✨ bubble appears."
        }
    }

    private fun isServiceEnabled(): Boolean {
        val flat = Settings.Secure.getString(
            contentResolver, Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES
        ) ?: return false
        return flat.contains(packageName)
    }

    private fun setupSpinner(id: Int, items: List<String>, selected: Int) {
        val spinner = findViewById<Spinner>(id)
        spinner.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, items)
        spinner.setSelection(selected.coerceIn(0, items.size - 1))
    }

    private fun saveAll() {
        SettingsStore.apiKey = findViewById<EditText>(R.id.apiKey).text.toString()
        SettingsStore.baseUrl = findViewById<EditText>(R.id.baseUrl).text.toString()
            .ifBlank { SettingsStore.DEFAULT_BASE_URL }
        SettingsStore.model = findViewById<EditText>(R.id.model).text.toString()
            .ifBlank { SettingsStore.DEFAULT_MODEL }
        SettingsStore.spicyModel = findViewById<EditText>(R.id.spicyModel).text.toString()

        SettingsStore.suggestionCount = findViewById<Spinner>(R.id.suggestionCount).selectedItemPosition + 1
        SettingsStore.replyLength = findViewById<Spinner>(R.id.replyLength).selectedItemPosition
        SettingsStore.emojiUse = findViewById<Spinner>(R.id.emojiUse).selectedItemPosition
        SettingsStore.contextAmount = findViewById<Spinner>(R.id.contextAmount).selectedItemPosition
        SettingsStore.defaultMode = Mode.entries[findViewById<Spinner>(R.id.defaultMode).selectedItemPosition]

        ProfileStore.save(profiles)
        Toast.makeText(this, "Saved", Toast.LENGTH_SHORT).show()
    }

    // ------------------------------------------------------------------
    // Profiles
    // ------------------------------------------------------------------

    private fun renderProfiles() {
        val box = findViewById<LinearLayout>(R.id.profilesBox)
        box.removeAllViews()
        if (profiles.isEmpty()) {
            box.addView(TextView(this).apply {
                text = "No profiles yet. Add one per person you chat with."
                textSize = 13f
            })
            return
        }
        profiles.forEach { profile ->
            val row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
            row.addView(TextView(this).apply {
                text = "👤 ${profile.name.ifBlank { "Unnamed" }}  ·  ${profile.stage.ifBlank { "—" }}  ·  🌶${profile.spiceCap}/5"
                textSize = 15f
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            row.addView(TextView(this).apply {
                text = "✏️"
                textSize = 17f
                setPadding(16, 8, 16, 8)
                setOnClickListener { editProfile(profile) }
            })
            row.addView(TextView(this).apply {
                text = "🗑"
                textSize = 17f
                setPadding(16, 8, 16, 8)
                setOnClickListener {
                    profiles.remove(profile)
                    if (SettingsStore.selectedProfileId == profile.id) {
                        SettingsStore.selectedProfileId = ""
                    }
                    ProfileStore.save(profiles)
                    renderProfiles()
                }
            })
            box.addView(row)
        }
    }

    private fun editProfile(existing: Profile?) {
        val view = LayoutInflater.from(this).inflate(R.layout.dialog_profile, null)
        val name = view.findViewById<EditText>(R.id.pName)
        val stage = view.findViewById<EditText>(R.id.pStage)
        val style = view.findViewById<EditText>(R.id.pStyle)
        val notes = view.findViewById<EditText>(R.id.pNotes)
        val avoid = view.findViewById<EditText>(R.id.pAvoid)
        val spice = view.findViewById<SeekBar>(R.id.pSpice)
        val spiceLabel = view.findViewById<TextView>(R.id.pSpiceLabel)

        val profile = existing ?: Profile()
        name.setText(profile.name)
        stage.setText(profile.stage)
        style.setText(profile.style)
        notes.setText(profile.notes)
        avoid.setText(profile.avoid)
        spice.progress = profile.spiceCap - 1
        spiceLabel.text = "Spice ceiling: ${profile.spiceCap}/5"
        spice.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(sb: SeekBar?, p: Int, fromUser: Boolean) {
                spiceLabel.text = "Spice ceiling: ${p + 1}/5"
            }
            override fun onStartTrackingTouch(sb: SeekBar?) {}
            override fun onStopTrackingTouch(sb: SeekBar?) {}
        })

        AlertDialog.Builder(this)
            .setTitle(if (existing == null) "Add profile" else "Edit profile")
            .setView(view)
            .setPositiveButton("Save") { _, _ ->
                profile.name = name.text.toString()
                profile.stage = stage.text.toString()
                profile.style = style.text.toString()
                profile.notes = notes.text.toString()
                profile.avoid = avoid.text.toString()
                profile.spiceCap = spice.progress + 1
                if (existing == null) profiles.add(profile)
                ProfileStore.save(profiles)
                renderProfiles()
            }
            .setNegativeButton("Cancel", null)
            .show()
    }
}
