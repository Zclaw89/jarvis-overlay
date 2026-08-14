import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface DossierItem {
  id: string
  createdAt: string
  note: string
  screenshotPath: string
  screenshotHash: string
  sourceTitle?: string | null
  analysis?: string | null
}

export function DossierView() {
  const [items, setItems] = useState<DossierItem[]>([])
  const [status, setStatus] = useState('Drop an Olé from the floating badge to add a capture.')

  const refresh = () => {
    invoke<DossierItem[]>('list_dossier_items')
      .then((next) => {
        setItems(next)
        setStatus(next.length ? `${next.length} local dossier item${next.length === 1 ? '' : 's'}.` : 'No dossier items yet.')
      })
      .catch((error) => setStatus(String(error)))
  }

  useEffect(refresh, [])

  return (
    <div style={{ flex:1, overflowY:'auto', padding:'24px 32px 48px', display:'flex', flexDirection:'column', gap:18 }}>
      <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center', gap:12 }}>
        <div>
          <h2 style={{ fontFamily:"'Instrument Serif',serif", fontSize:24, fontWeight:400, color:'var(--text)', margin:0 }}>Living Dossier</h2>
          <p style={{ fontSize:12, color:'var(--text-muted)', margin:'4px 0 0' }}>{status}</p>
        </div>
        <button onClick={refresh} style={{ fontSize:12, color:'var(--text)', background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:10, padding:'8px 12px', cursor:'pointer' }}>Refresh</button>
      </div>

      {items.map((item) => (
        <article key={item.id} style={{ background:'var(--surface)', border:'1px solid var(--border)', borderRadius:12, padding:16, display:'flex', flexDirection:'column', gap:10 }}>
          <div style={{ display:'flex', justifyContent:'space-between', gap:12 }}>
            <div>
              <strong style={{ color:'var(--text)', fontSize:14 }}>{item.note || 'Drop an Olé'}</strong>
              <div style={{ fontSize:11, color:'var(--text-muted)' }}>{new Date(item.createdAt).toLocaleString()}</div>
            </div>
            <code style={{ color:'var(--text-muted)', fontSize:10 }}>{item.screenshotHash.slice(0, 12)}</code>
          </div>
          {item.sourceTitle && <div style={{ fontSize:12, color:'var(--text-muted)' }}>Source: {item.sourceTitle}</div>}
          <div style={{ fontSize:11, color:'var(--text-muted)', wordBreak:'break-all' }}>{item.screenshotPath}</div>
          {item.analysis ? (
            <div style={{ whiteSpace:'pre-wrap', fontSize:13, color:'var(--text)', lineHeight:1.5, background:'rgba(255,255,255,0.03)', border:'1px solid var(--border)', borderRadius:10, padding:12 }}>{item.analysis}</div>
          ) : (
            <div style={{ fontSize:12, color:'var(--text-muted)' }}>Analysis needs an API key.</div>
          )}
        </article>
      ))}
    </div>
  )
}
