export interface OleTheme {
  id: string
  name: string
  dark: boolean
  colors: {
    background: string
    surface: string
    elevated: string
    border: string
    textPrimary: string
    textSecondary: string
    textTertiary: string
    accent: string
    termRed: string
    termGreen: string
  }
}

export const APPEARANCE_THEMES: OleTheme[] = [
  {
    id: 'default',
    name: 'MeshPilot Default',
    dark: true,
    colors: {
      background: 'oklch(0.1913 0 0)', surface: 'oklch(0.2264 0 0)', elevated: 'oklch(0.2850 0 0)',
      border: 'oklch(0.2931 0 0)', textPrimary: 'oklch(0.9173 0.0133 82.4015)', textSecondary: 'oklch(0.6348 0.0113 81.7875)',
      textTertiary: 'oklch(0.8520 0.0205 100.6306)', accent: 'oklch(0.8520 0.0205 100.6306)',
      termRed: '#ff5555', termGreen: '#50fa7b',
    },
  },
  {
    id: 'void',
    name: 'Void',
    dark: true,
    colors: {
      background: '#0a0a0a', surface: '#111111', elevated: '#1a1a1a',
      border: '#1e1e1e', textPrimary: '#ffffff', textSecondary: '#888888',
      textTertiary: '#444444', accent: '#ffffff', termRed: '#ff5555', termGreen: '#50fa7b',
    },
  },
  {
    id: 'mecha',
    name: 'Mecha',
    dark: true,
    colors: {
      background: '#181b1f', surface: '#23272d', elevated: '#323841',
      border: '#4c5563', textPrimary: '#cbd5e1', textSecondary: '#94a3b8',
      textTertiary: '#64748b', accent: '#eab308', termRed: '#ef4444', termGreen: '#22c55e',
    },
  },
  {
    id: 'nord',
    name: 'Nord',
    dark: true,
    colors: {
      background: '#2e3440', surface: '#2b303b', elevated: '#3b4252',
      border: '#434c5e', textPrimary: '#eceff4', textSecondary: '#d8dee9',
      textTertiary: '#7b88a1', accent: '#88c0d0', termRed: '#bf616a', termGreen: '#a3be8c',
    },
  },
  {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    dark: true,
    colors: {
      background: '#1a1b26', surface: '#16161e', elevated: '#292e42',
      border: '#2f3549', textPrimary: '#c0caf5', textSecondary: '#a9b1d6',
      textTertiary: '#565f89', accent: '#7aa2f7', termRed: '#f7768e', termGreen: '#9ece6a',
    },
  },
  {
    id: 'catppuccin-mocha',
    name: 'Catppuccin Mocha',
    dark: true,
    colors: {
      background: '#1e1e2e', surface: '#181825', elevated: '#313244',
      border: '#45475a', textPrimary: '#cdd6f4', textSecondary: '#a6adc8',
      textTertiary: '#6c7086', accent: '#cba6f7', termRed: '#f38ba8', termGreen: '#a6e3a1',
    },
  },
  {
    id: 'gruvbox',
    name: 'Gruvbox',
    dark: true,
    colors: {
      background: '#282828', surface: '#1d2021', elevated: '#3c3836',
      border: '#504945', textPrimary: '#ebdbb2', textSecondary: '#a89984',
      textTertiary: '#928374', accent: '#fabd2f', termRed: '#fb4934', termGreen: '#b8bb26',
    },
  },
  {
    id: 'rose-pine',
    name: 'Rose Pine',
    dark: true,
    colors: {
      background: '#191724', surface: '#1f1d2e', elevated: '#26233a',
      border: '#403d52', textPrimary: '#e0def4', textSecondary: '#908caa',
      textTertiary: '#6e6a86', accent: '#c4a7e7', termRed: '#eb6f92', termGreen: '#31748f',
    },
  },
  {
    id: 'one-dark',
    name: 'One Dark',
    dark: true,
    colors: {
      background: '#282c34', surface: '#21252b', elevated: '#2c313a',
      border: '#3b4048', textPrimary: '#abb2bf', textSecondary: '#7f848e',
      textTertiary: '#5c6370', accent: '#61afef', termRed: '#e06c75', termGreen: '#98c379',
    },
  },
  {
    id: 'slate',
    name: 'Slate',
    dark: true,
    colors: {
      background: '#3b4049', surface: '#434954', elevated: '#4e5562',
      border: '#5d6573', textPrimary: '#e9ecf1', textSecondary: '#aab2c0',
      textTertiary: '#818a99', accent: '#7dd3fc', termRed: '#f87171', termGreen: '#86efac',
    },
  },
  {
    id: 'driftwood',
    name: 'Driftwood',
    dark: true,
    colors: {
      background: '#45403a', surface: '#4d473f', elevated: '#585149',
      border: '#6a6157', textPrimary: '#f2ede5', textSecondary: '#c8bdb0',
      textTertiary: '#978c7d', accent: '#e0a458', termRed: '#e87d6f', termGreen: '#a3c585',
    },
  },
  {
    id: 'sage',
    name: 'Sage',
    dark: true,
    colors: {
      background: '#3c443e', surface: '#444d46', elevated: '#4f5a51',
      border: '#5f6b62', textPrimary: '#e9f0ea', textSecondary: '#afbdb2',
      textTertiary: '#7f8d83', accent: '#86c79a', termRed: '#e58a82', termGreen: '#86c79a',
    },
  },
  {
    id: 'dusk',
    name: 'Dusk',
    dark: true,
    colors: {
      background: '#403b48', surface: '#484252', elevated: '#534c5e',
      border: '#645a72', textPrimary: '#ece8f1', textSecondary: '#b9b1c6',
      textTertiary: '#897f98', accent: '#c4a7e7', termRed: '#e88aa0', termGreen: '#9bc995',
    },
  },
  {
    id: 'github-light',
    name: 'GitHub Light',
    dark: false,
    colors: {
      background: '#ffffff', surface: '#f6f8fa', elevated: '#eaeef2',
      border: '#d0d7de', textPrimary: '#1f2328', textSecondary: '#656d76',
      textTertiary: '#8c959f', accent: '#0969da', termRed: '#cf222e', termGreen: '#116329',
    },
  },
  {
    id: 'catppuccin-latte',
    name: 'Catppuccin Latte',
    dark: false,
    colors: {
      background: '#eff1f5', surface: '#e6e9ef', elevated: '#dce0e8',
      border: '#ccd0da', textPrimary: '#4c4f69', textSecondary: '#6c6f85',
      textTertiary: '#8c8fa1', accent: '#8839ef', termRed: '#d20f39', termGreen: '#40a02b',
    },
  },
  {
    id: 'solarized-light',
    name: 'Solarized Light',
    dark: false,
    colors: {
      background: '#fdf6e3', surface: '#eee8d5', elevated: '#e7e0c9',
      border: '#d6cfb3', textPrimary: '#073642', textSecondary: '#586e75',
      textTertiary: '#93a1a1', accent: '#268bd2', termRed: '#dc322f', termGreen: '#859900',
    },
  },
]

const STORAGE_KEY = 'ole-theme-id'

function hexToRgb(value: string) {
  const match = value.trim().match(/^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i)
  if (!match) return '217, 211, 190'
  return `${parseInt(match[1], 16)}, ${parseInt(match[2], 16)}, ${parseInt(match[3], 16)}`
}

export function getStoredThemeId() {
  if (typeof localStorage === 'undefined') return 'default'
  return localStorage.getItem(STORAGE_KEY) ?? 'default'
}

export function storeThemeId(id: string) {
  if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_KEY, id)
}

export function getTheme(id: string) {
  return APPEARANCE_THEMES.find((theme) => theme.id === id) ?? APPEARANCE_THEMES[0]
}

export function applyThemeById(id: string) {
  const theme = getTheme(id)
  const root = document.documentElement
  const c = theme.colors
  root.dataset.theme = theme.id
  root.dataset.themeMode = theme.dark ? 'dark' : 'light'
  root.style.setProperty('--background', c.background)
  root.style.setProperty('--foreground', c.textPrimary)
  root.style.setProperty('--card', c.surface)
  root.style.setProperty('--popover', c.surface)
  root.style.setProperty('--primary', c.accent)
  root.style.setProperty('--primary-foreground', theme.dark ? c.background : '#ffffff')
  root.style.setProperty('--muted', c.elevated)
  root.style.setProperty('--muted-foreground', c.textSecondary)
  root.style.setProperty('--accent', c.elevated)
  root.style.setProperty('--accent-foreground', c.textPrimary)
  root.style.setProperty('--destructive', c.termRed)
  root.style.setProperty('--border', c.border)
  root.style.setProperty('--input', c.border)
  root.style.setProperty('--ring', c.accent)
  root.style.setProperty('--panel', `color-mix(in oklab, ${c.background} 88%, #000)`)
  root.style.setProperty('--bg', c.background)
  root.style.setProperty('--bg-2', `color-mix(in oklab, ${c.background} 88%, #000)`)
  root.style.setProperty('--surface', c.surface)
  root.style.setProperty('--surface-2', c.elevated)
  root.style.setProperty('--elevated', c.elevated)
  root.style.setProperty('--text', c.textPrimary)
  root.style.setProperty('--text-2', c.textSecondary)
  root.style.setProperty('--text-muted', c.textSecondary)
  root.style.setProperty('--primary-fg', theme.dark ? c.background : '#ffffff')
  root.style.setProperty('--accent-rgb', hexToRgb(c.accent))
  root.style.setProperty('--accent-hover', c.textPrimary)
  root.style.setProperty('--accent-2', c.termRed)
  root.style.setProperty('--success', c.termGreen)
  root.style.setProperty('--sidebar-bg', `color-mix(in oklab, ${c.background} 88%, #000)`)
  root.style.setProperty('--sidebar-text', c.textPrimary)
  root.style.setProperty('--bg-page', c.background)
  root.style.setProperty('--bg-sidebar', `color-mix(in oklab, ${c.background} 88%, #000)`)
  root.style.setProperty('--bg-surface', c.surface)
  root.style.setProperty('--bg-surface-elevated', c.elevated)
  root.style.setProperty('--bg-hover', c.elevated)
  root.style.setProperty('--border-subtle', c.border)
  root.style.setProperty('--border-focus', c.accent)
  root.style.setProperty('--text-primary', c.textPrimary)
  root.style.setProperty('--text-secondary', c.textSecondary)
  root.style.setProperty('--text-accent', c.accent)
  root.style.setProperty('--primary-cta', c.accent)
  root.style.setProperty('--primary-cta-text', theme.dark ? c.background : '#ffffff')
  root.style.setProperty('--widget-bg', `color-mix(in oklab, ${c.background} 94%, transparent)`)
  root.style.setProperty('--widget-border', `color-mix(in oklab, ${c.border} 90%, transparent)`)
  root.style.setProperty('color-scheme', theme.dark ? 'dark' : 'light')
  return theme
}

export function applyStoredTheme() {
  return applyThemeById(getStoredThemeId())
}
