import { useEffect, useState } from 'react'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Radio } from 'lucide-react'
import { useAppStore } from '../store/appStore'

interface WhisperModel {
  id: string
  name: string
  filename: string
}

/** Turn a raw model filename into a readable label (fallback when the model list isn't available). */
function prettifyFilename(filename: string): string {
  return filename
    .replace(/^ggml-/, '')
    .replace(/\.(bin|onnx)$/i, '')
    .replace(/[-_]/g, ' ')
    .trim()
}

/**
 * VersionWidget — the sidebar footer card showing the app version
 * and the active local model name.
 */
export function VersionWidget() {
  const engine = useAppStore((state) => state.engine)
  const [version, setVersion] = useState('')
  const [modelName, setModelName] = useState('')

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {})
  }, [])

  useEffect(() => {
    const resolveModel = async () => {
      try {
        const [filename, models] = await Promise.all([
          invoke<string | null>('get_setting', { key: 'model' }),
          invoke<WhisperModel[]>('get_available_models').catch(() => [] as WhisperModel[]),
        ])
        const selected = filename ?? 'ggml-base.en.bin'
        const match = models.find((m) => m.filename === selected)
        setModelName(match?.name ?? prettifyFilename(selected))
      } catch {
        setModelName('')
      }
    }
    resolveModel()
    // Refresh when a different model is loaded from Settings.
    const sub = listen('model-loaded', resolveModel)
    return () => { sub.then((f) => f()) }
  }, [])

  const isCloud = engine === 'cloud'

  return (
    <div className="utility-version-widget">
      <div className="utility-version-widget-head">
        <img src="/ole-badge.svg" alt="" width="22" height="22" />
        <span className="utility-version-widget-name">Olé</span>
        <span className="utility-version-chip">v{version || '—'}</span>
      </div>
      <div className={`utility-engine-card ${engine}`}>
        <Radio size={12} strokeWidth={1.8} />
        <span>{isCloud ? 'Cloud transcription' : (modelName || 'Local Whisper engine')}</span>
      </div>
    </div>
  )
}
