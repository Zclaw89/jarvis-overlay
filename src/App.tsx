import React, { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import {
  BookOpen,
  ClipboardList,
  Clock3,
  Info,
  KeyRound,
  Mic2,
  PanelLeft,
  Palette,
  Settings2,
  Sparkles,
  WandSparkles,
} from 'lucide-react'
import { Dashboard } from './components/Dashboard'
import { Settings } from './components/Settings'
import { DictionaryEditor } from './components/DictionaryEditor'
import { MainApp as PromptApp } from './components/PromptApp'
import { AppearanceView } from './components/AppearanceView'
import { AboutView } from './components/AboutView'
import { VersionWidget } from './components/VersionWidget'
import { UpdateWidget } from './components/UpdateWidget'
import { applyStoredTheme } from './lib/appearance'
import { useAppStore } from './store/appStore'

import './styles-prompt.css'
import './App.css'

type View =
  | 'voice-history'
  | 'voice-dictionary'
  | 'voice-settings'
  | 'prompt-enhancer'
  | 'prompt-actions'
  | 'prompt-history'
  | 'prompt-providers'
  | 'prompt-settings'
  | 'appearance'
  | 'about'

interface NavItem {
  id: View
  label: string
  icon: React.ReactNode
}

interface NavGroup {
  title: string
  items: NavItem[]
}

const GROUPS: NavGroup[] = [
  {
    title: 'Voice Dictation',
    items: [
      { id: 'voice-history', label: 'Dictation History', icon: <Clock3 size={15} strokeWidth={1.8} /> },
      { id: 'voice-dictionary', label: 'Custom Dictionary', icon: <BookOpen size={15} strokeWidth={1.8} /> },
    ],
  },
  {
    title: 'Prompt Enhancer',
    items: [
      { id: 'prompt-enhancer', label: 'Enhance Prompt', icon: <WandSparkles size={15} strokeWidth={1.8} /> },
      { id: 'prompt-actions', label: 'Prompt Actions', icon: <Sparkles size={15} strokeWidth={1.8} /> },
      { id: 'prompt-history', label: 'Action History', icon: <ClipboardList size={15} strokeWidth={1.8} /> },
    ],
  },
  {
    title: 'Configuration',
    items: [
      { id: 'voice-settings', label: 'Voice & Hotkey', icon: <Mic2 size={15} strokeWidth={1.8} /> },
      { id: 'prompt-providers', label: 'API Keys', icon: <KeyRound size={15} strokeWidth={1.8} /> },
      { id: 'prompt-settings', label: 'Enhancer Settings', icon: <Settings2 size={15} strokeWidth={1.8} /> },
      { id: 'appearance', label: 'Appearance', icon: <Palette size={15} strokeWidth={1.8} /> },
      { id: 'about', label: 'About', icon: <Info size={15} strokeWidth={1.8} /> },
    ],
  },
]

const ALL_VIEWS = GROUPS.flatMap((group) => group.items.map((item) => item.id))

function getInitialView(): View {
  if (typeof window === 'undefined') return 'voice-history'
  const candidate = new URLSearchParams(window.location.search).get('view')
  return ALL_VIEWS.includes(candidate as View) ? candidate as View : 'voice-history'
}

function handleBarMouseDown(e: React.MouseEvent<HTMLDivElement>) {
  if (e.button !== 0) return
  let el: HTMLElement | null = e.target as HTMLElement
  while (el && el !== e.currentTarget) {
    if (el.tagName.toLowerCase() === 'button') return
    el = el.parentElement
  }
  e.preventDefault()
  getCurrentWindow().startDragging().catch(() => {})
}

function shouldSkipDrag(target: HTMLElement, boundary: HTMLElement) {
  let el: HTMLElement | null = target
  while (el && el !== boundary) {
    const tag = el.tagName.toLowerCase()
    if (tag === 'button' || tag === 'input' || tag === 'select' || tag === 'textarea' || tag === 'a') return true
    el = el.parentElement
  }
  return false
}

function handleDragPointerDown(e: React.PointerEvent<HTMLDivElement>, appWindow: ReturnType<typeof getCurrentWindow> | null) {
  if (e.button !== 0 || !appWindow) return
  if (shouldSkipDrag(e.target as HTMLElement, e.currentTarget)) return
  e.preventDefault()
  appWindow.startDragging().catch(() => {})
}

async function winClose(appWindow: ReturnType<typeof getCurrentWindow> | null) {
  if (!appWindow) return
  try { await appWindow.close() } catch { /* ignore */ }
  setTimeout(() => { appWindow.destroy?.().catch(() => {}) }, 200)
}

function WinButton({ label, onClick, danger, children }: {
  label: string
  onClick: () => void
  danger?: boolean
  children: React.ReactNode
}) {
  return (
    <button
      className={danger ? 'utility-win-button danger' : 'utility-win-button'}
      title={label}
      aria-label={label}
      onMouseDown={(e) => e.stopPropagation()}
      onClick={(e) => {
        e.stopPropagation()
        e.preventDefault()
        onClick()
      }}
    >
      {children}
    </button>
  )
}

export default function App() {
  const [view, setView] = useState<View>(getInitialView)
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false)
  const engine = useAppStore((state) => state.engine)
  const appWindowRef = useRef<ReturnType<typeof getCurrentWindow> | null>(null)

  useEffect(() => {
    try { appWindowRef.current = getCurrentWindow() } catch { /* ignore outside Tauri */ }
    applyStoredTheme()
  }, [])

  useEffect(() => {
    const unsub = listen<string>('navigate-view', (event) => {
      const targetView = event.payload
      if (targetView === 'dashboard') {
        setView('voice-history')
      } else if (targetView === 'prompt') {
        setView('prompt-enhancer')
      } else if (targetView === 'settings') {
        setView('voice-settings')
      } else if (targetView === 'dictionary') {
        setView('voice-dictionary')
      } else if (ALL_VIEWS.includes(targetView as View)) {
        setView(targetView as View)
      }
    })

    const unsubLegacy = listen('navigate-settings', () => setView('voice-settings'))

    const unsubPromptView = listen<string>('meshprompt://open-view', (event) => {
      const promptView = event.payload
      if (promptView === 'text') setView('prompt-enhancer')
      else if (promptView === 'providers') setView('prompt-providers')
      else if (promptView === 'actions') setView('prompt-actions')
      else if (promptView === 'history') setView('prompt-history')
      else if (promptView === 'settings') setView('prompt-settings')
    })

    return () => {
      unsub.then((f) => f())
      unsubLegacy.then((f) => f())
      unsubPromptView.then((f) => f())
    }
  }, [])

  return (
    <div className="utility-shell">
      <div
        className="utility-titlebar"
        data-tauri-drag-region
        onMouseDown={handleBarMouseDown}
        onPointerDown={(e) => handleDragPointerDown(e, appWindowRef.current)}
      >
        <div className="utility-titlebar-left" data-tauri-drag-region>
          <button
            className={sidebarCollapsed ? 'utility-icon-button' : 'utility-icon-button active'}
            title={sidebarCollapsed ? 'Show utility navigation' : 'Hide utility navigation'}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => {
              e.preventDefault()
              setSidebarCollapsed((current) => !current)
            }}
          >
            <PanelLeft size={14} strokeWidth={1.8} />
          </button>
        </div>

        <span className="utility-title-center" data-tauri-drag-region>
          <img src="/logo-prompt.png" alt="" width="18" height="18" />
          MeshUtility Suite
        </span>

        <div className="utility-window-controls" data-tauri-drag-region>
          <div className="utility-window-button-group">
            <WinButton label="Minimize" onClick={() => appWindowRef.current?.minimize()}>
              <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
                <line x1="2.5" y1="6" x2="9.5" y2="6" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
              </svg>
            </WinButton>
            <WinButton
              label="Maximize"
              onClick={async () => {
                const appWindow = appWindowRef.current
                if (!appWindow) return
                if (await appWindow.isMaximized()) await appWindow.unmaximize()
                else await appWindow.maximize()
              }}
            >
              <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
                <rect x="2.6" y="2.6" width="6.8" height="6.8" rx="1.4" stroke="currentColor" strokeWidth="1.3" />
              </svg>
            </WinButton>
            <WinButton
              label="Close"
              danger
              onClick={() => void winClose(appWindowRef.current)}
            >
              <svg width="11" height="11" viewBox="0 0 12 12" fill="none">
                <path d="M3 3l6 6M9 3l-6 6" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
              </svg>
            </WinButton>
          </div>
        </div>
      </div>

      <div className={sidebarCollapsed ? 'utility-body sidebar-collapsed' : 'utility-body'}>
        <aside className="utility-sidebar" aria-hidden={sidebarCollapsed}>
          <div className="utility-sidebar-scroll">
            {GROUPS.map((group) => (
              <div key={group.title} className="utility-nav-group">
                <span className="utility-nav-heading">{group.title}</span>
                <nav className="utility-nav-list">
                  {group.items.map(({ id, label, icon }) => (
                    <button
                      key={id}
                      onClick={() => setView(id)}
                      className={view === id ? 'utility-nav-item active' : 'utility-nav-item'}
                    >
                      <span className="utility-nav-icon">{icon}</span>
                      <span>{label}</span>
                    </button>
                  ))}
                </nav>
              </div>
            ))}
          </div>

          <div
            className="utility-sidebar-footer"
            data-tauri-drag-region
            onMouseDown={handleBarMouseDown}
            onPointerDown={(e) => handleDragPointerDown(e, appWindowRef.current)}
          >
            <UpdateWidget />
            <VersionWidget />
          </div>
        </aside>

        <main className="utility-main">
          <div style={{ display: view === 'voice-history' ? 'flex' : 'none', flexDirection: 'column', height: '100%' }}>
            <Dashboard />
          </div>
          <div style={{ display: view === 'voice-dictionary' ? 'flex' : 'none', flexDirection: 'column', height: '100%', overflowY: 'auto' }}>
            <DictionaryEditor />
          </div>
          <div style={{ display: view === 'voice-settings' ? 'flex' : 'none', flexDirection: 'column', height: '100%', overflowY: 'auto' }}>
            <Settings />
          </div>
          <div style={{ display: view === 'appearance' ? 'flex' : 'none', flexDirection: 'column', height: '100%', overflowY: 'auto' }}>
            <AppearanceView />
          </div>
          <div style={{ display: view === 'about' ? 'flex' : 'none', flexDirection: 'column', height: '100%', overflowY: 'auto' }}>
            <AboutView />
          </div>

          <div
            style={{
              display: ['prompt-enhancer', 'prompt-actions', 'prompt-history', 'prompt-providers', 'prompt-settings'].includes(view) ? 'flex' : 'none',
              flexDirection: 'column',
              height: '100%',
              width: '100%',
              overflow: 'hidden',
            }}
          >
            <PromptApp
              embed={true}
              hideSidebar={true}
              activeView={
                view === 'prompt-enhancer' ? 'text' :
                view === 'prompt-actions' ? 'actions' :
                view === 'prompt-history' ? 'history' :
                view === 'prompt-providers' ? 'providers' :
                view === 'prompt-settings' ? 'settings' :
                'text'
              }
            />
          </div>
        </main>
      </div>
    </div>
  )
}
