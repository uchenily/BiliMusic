import { Maximize2, Minimize2, Minus, X } from 'lucide-react'
import { useEffect, useState, type ReactNode } from 'react'
import appIcon from '@/assets/icon.png'
import { currentWindow, isTauri } from '@/services/tauriApi'

const noDrag = { WebkitAppRegion: 'no-drag' } as React.CSSProperties

export default function TitleBar() {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    if (!isTauri()) return
    const appWindow = currentWindow()
    const sync = () => { void appWindow.isMaximized().then(setMaximized).catch(() => {}) }
    sync()
    const timer = window.setInterval(sync, 500)
    return () => window.clearInterval(timer)
  }, [])

  return (
    <div className="app-titlebar">
      <div className="app-titlebar__brand">
        <img src={appIcon} alt="" className="app-titlebar__logo" draggable={false} />
        BiliMusic
      </div>
      <div className="app-titlebar__drag-region" data-tauri-drag-region />

      <div className="app-titlebar__controls">
        <WindowButton icon={<Minus size={14} />} action="minimize" label="最小化" />
        <WindowButton icon={maximized ? <Minimize2 size={13} /> : <Maximize2 size={13} />} action="maximize" label={maximized ? '还原窗口' : '最大化'} />
        <WindowButton icon={<X size={15} />} action="close" label="关闭" isClose />
      </div>
    </div>
  )
}

function WindowButton({
  icon,
  action,
  label,
  isClose = false,
}: {
  icon: ReactNode
  action: 'minimize' | 'maximize' | 'close'
  label: string
  isClose?: boolean
}) {
  const handleClick = () => {
    if (!isTauri()) return
    const appWindow = currentWindow()
    if (action === 'minimize') void appWindow.minimize().catch(() => {})
    else if (action === 'maximize') void appWindow.toggleMaximize().catch(() => {})
    else void appWindow.hide().catch(() => {})
  }

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={`app-window-button ${isClose ? 'app-window-button--close' : ''}`}
      onClick={handleClick}
      style={noDrag}
    >
      {icon}
    </button>
  )
}
