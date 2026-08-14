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

  const importFile = async (file: File) => {
    const buffer = await file.arrayBuffer()
    let binary = ''
    const bytes = new Uint8Array(buffer)
    for (let i = 0; i < bytes.length; i += 1) binary += String.fromCharCode(bytes[i])
    await invoke('import_dossier_file', {
      fileName: file.name,
      mime: file.type || 'application/octet-stream',
      dataBase64: btoa(binary),
    })
    refresh()
  }

  return (
    <div
      onDragOver={(event) => event.preventDefault()}
      onDrop={(event) => {
        event.preventDefault()
        const file = event.dataTransfer.files.item(0)
        if (file) void importFile(file)
      }}
      style={{ flex:1, overflowY:'auto', padding:'24px 32px 48px', display:'flex', flexDirection:'column', gap:18 }}
    >
      <div style={{ display:'flex', justifyContent:'space-between', alignItems:'center', gap:12 }}>
        <div>
          <h2 style={{ fontFamily:"'Instrument Serif',serif", fontSize:24, fontWeight:400, color:'var(--text)', margin:0 }}>Living Dossier</h2>
          <p style={{ fontSize:12, color:'var(--text-muted)', margin:'4px 0 0' }}>{status}</p>
        </div>
        <button onClick={refresh} style={{ fontSize:12, color:'var(--text)', background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:10, padding:'8px 12px', cursor:'pointer' }}>Refresh</button>
      </div>

      <section style={{ minHeight: 220, border:'1px solid var(--border)', borderRadius:16, background:'radial-gradient(circle at center, rgba(255,229,168,0.10), transparent 34%), var(--surface)', position:'relative', overflow:'hidden', padding:18 }}>
        <div style={{ position:'absolute', left:'50%', top:'50%', transform:'translate(-50%, -50%)', width:108, height:108, borderRadius:'50%', display:'grid', placeItems:'center', background:'rgba(0,0,0,0.45)', border:'1px solid rgba(255,229,168,0.36)', color:'#ffe9a8', fontWeight:800 }}>
          Olé
        </div>
        {items.slice(0, 10).map((item, index) => {
          const angle = (Math.PI * 2 * index) / Math.max(items.slice(0, 10).length, 1)
          const x = 50 + Math.cos(angle) * 34
          const y = 50 + Math.sin(angle) * 34
          return (
            <div key={item.id} title={item.note} style={{ position:'absolute', left:`${x}%`, top:`${y}%`, transform:'translate(-50%, -50%)', maxWidth:112, padding:'8px 10px', borderRadius:999, background:'rgba(255,229,168,0.10)', border:'1px solid rgba(255,229,168,0.24)', color:'var(--text)', fontSize:11, overflow:'hidden', textOverflow:'ellipsis', whiteSpace:'nowrap' }}>
              {item.note || 'Drop'}
            </div>
          )
        })}
      </section>

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
