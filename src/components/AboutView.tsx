import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'

export function AboutView() {
  const [version, setVersion] = useState('')

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
  }, [])

  return (
    <div style={{ flex: 1, overflowY: 'auto', padding: '32px 40px', display: 'flex', flexDirection: 'column', gap: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 16, borderBottom: '1px solid var(--border)', paddingBottom: 24 }}>
        <img
          src="/ole-badge.svg"
          alt="Olé"
          style={{ width: 64, height: 64, borderRadius: '50%', border: '1px solid var(--border)', boxShadow: '0 8px 20px rgba(0, 0, 0, 0.3)' }}
        />
        <div>
          <h2 style={{ fontFamily: "'Instrument Serif', serif", fontSize: 28, fontWeight: 400, color: 'var(--text)', margin: 0 }}>Olé</h2>
          <p style={{ color: 'var(--text-muted)', fontSize: 12, margin: '4px 0 0' }}>Windows desktop companion for local drops and a living dossier.</p>
          <span style={{ display:'inline-flex', marginTop: 8, fontSize: 11, fontFamily: 'var(--font-mono)', background: 'var(--surface-2)', color: 'var(--text)', padding: '2px 8px', borderRadius: 12, border: '1px solid var(--border)' }}>
            v{version || '1.0.10'}
          </span>
        </div>
      </div>

      <section style={{ background: 'var(--surface)', border: '1px solid var(--border)', borderRadius: 12, padding: 20 }}>
        <h3 style={{ color:'var(--text)', fontSize: 15, margin: '0 0 8px' }}>What it does</h3>
        <p style={{ color:'var(--text-muted)', fontSize: 13, lineHeight: 1.6, margin: 0 }}>
          Drop an Olé to capture a normal screenshot, save it locally, and optionally add a short AI analysis when an API key is available.
        </p>
      </section>

      <section style={{ background: 'var(--surface)', border: '1px solid var(--border)', borderRadius: 12, padding: 20 }}>
        <h3 style={{ color:'var(--text)', fontSize: 15, margin: '0 0 8px' }}>License</h3>
        <p style={{ color:'var(--text-muted)', fontSize: 13, lineHeight: 1.6, margin: 0 }}>
          Apache-2.0. See the repository license and third-party notices for details.
        </p>
      </section>
    </div>
  )
}
