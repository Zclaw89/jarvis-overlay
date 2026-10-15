import { emit } from '@tauri-apps/api/event'
import { useState } from 'react'
import {
  APPEARANCE_THEMES,
  applyThemeById,
  getStoredThemeId,
  storeThemeId,
  type OleTheme,
} from '../lib/appearance'

function ThemeCard({
  theme,
  active,
  onSelect,
}: {
  theme: OleTheme
  active: boolean
  onSelect: () => void
}) {
  const c = theme.colors
  return (
    <button
      type="button"
      onClick={onSelect}
      style={{
        minHeight: 132,
        padding: 12,
        borderRadius: 8,
        border: `1px solid ${active ? 'var(--primary)' : 'var(--border)'}`,
        background: active ? 'color-mix(in oklab, var(--primary) 12%, var(--surface))' : 'var(--surface)',
        boxShadow: active ? '0 0 0 1px var(--primary)' : 'none',
        cursor: 'pointer',
        textAlign: 'left',
        display: 'flex',
        flexDirection: 'column',
        gap: 10,
        transition: 'all 0.15s ease',
      }}
    >
      <div style={{
        height: 64,
        borderRadius: 6,
        border: `1px solid ${c.border}`,
        background: c.background,
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
      }}>
        <div style={{ height: 16, borderBottom: `1px solid ${c.border}`, display: 'flex', gap: 3, alignItems: 'center', padding: '0 6px' }}>
          <span style={{ width: 4, height: 4, borderRadius: 999, background: c.textSecondary }} />
          <span style={{ width: 4, height: 4, borderRadius: 999, background: c.textSecondary, opacity: 0.65 }} />
          <span style={{ width: 4, height: 4, borderRadius: 999, background: c.textSecondary, opacity: 0.4 }} />
        </div>
        <div style={{ flex: 1, display: 'flex' }}>
          <div style={{ width: 28, background: c.surface, borderRight: `1px solid ${c.border}`, padding: 5 }}>
            <div style={{ height: 4, borderRadius: 999, background: c.accent, marginBottom: 5 }} />
            <div style={{ height: 4, borderRadius: 999, background: c.elevated }} />
          </div>
          <div style={{ flex: 1, padding: 6 }}>
            <div style={{ height: 8, width: '56%', borderRadius: 999, background: c.textPrimary, opacity: 0.55, marginBottom: 6 }} />
            <div style={{ height: 16, width: '72%', borderRadius: 4, background: c.elevated }} />
          </div>
        </div>
      </div>
      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
          <strong style={{ color: 'var(--text)', fontSize: 12, fontWeight: 700 }}>{theme.name}</strong>
          {active && <span style={{ color: 'var(--primary)', fontSize: 9, fontWeight: 800, textTransform: 'uppercase' }}>Active</span>}
        </div>
        <div style={{ color: 'var(--text-muted)', fontSize: 10, marginTop: 3 }}>{theme.dark ? 'Dark' : 'Light'} appearance</div>
      </div>
    </button>
  )
}

export function AppearanceView() {
  const [themeId, setThemeId] = useState(getStoredThemeId())

  const selectTheme = async (id: string) => {
    setThemeId(id)
    storeThemeId(id)
    applyThemeById(id)
    await emit('appearance-theme-changed', id).catch(console.error)
  }

  return (
    <div style={{ flex:1, overflowY:'auto', padding:'24px 32px 48px', display:'flex', flexDirection:'column', gap:18 }}>
      <div>
        <h2 style={{ fontFamily:"'Instrument Serif',serif", fontSize:24, fontWeight:400, color:'var(--text)', margin:0 }}>Appearance</h2>
      </div>

      <div style={{ display:'grid', gridTemplateColumns:'repeat(auto-fit, minmax(170px, 1fr))', gap:10 }}>
        {APPEARANCE_THEMES.map(theme => (
          <ThemeCard
            key={theme.id}
            theme={theme}
            active={themeId === theme.id}
            onSelect={() => void selectTheme(theme.id)}
          />
        ))}
      </div>
    </div>
  )
}
