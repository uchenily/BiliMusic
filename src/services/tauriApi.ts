import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open } from '@tauri-apps/plugin-shell'

export interface TauriBiliApi {
  downloadAudio: (audioUrl: string, filename: string) => Promise<{ filePath: string; size: number }>
  qrGenerate: () => Promise<{ url: string; qrcodeKey: string }>
  qrPoll: (qrcodeKey: string) => Promise<{ code: number; status: number; message: string; url: string }>
  getCookies: () => Promise<{ isLoggedIn: boolean; sessdata: string; biliJct: string; dedeUserId: string }>
  logout: () => Promise<{ success: boolean }>
}

export interface WebdavConfigInput {
  url: string
  username: string
  password: string
}

export interface WebdavConfigInfo {
  url: string
  username: string
  configured: boolean
}

export interface WebdavResult {
  ok: boolean
  status: number
  etag: string | null
  content: string | null
  message?: string
}

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function normalizeRemoteUrl(raw: string): string {
  if (!raw) return ''
  return raw.startsWith('//')
    ? `https:${raw}`
    : raw.startsWith('http://')
      ? `https://${raw.slice(7)}`
      : raw.startsWith('https://')
        ? raw
        : `https:${raw}`
}

const MEDIA_PROXY_BASE_URL = 'http://127.0.0.1:17823'

function isHttpsUrl(raw: string): boolean {
  try {
    return new URL(raw).protocol === 'https:'
  } catch {
    return false
  }
}

export function toBiliImageUrl(raw: string): string {
  const normalized = normalizeRemoteUrl(raw)
  if (!isTauri() || !isHttpsUrl(normalized)) return normalized
  return `${MEDIA_PROXY_BASE_URL}/image?url=${encodeURIComponent(normalized)}`
}

export function toBiliAudioUrl(raw: string): string {
  const normalized = normalizeRemoteUrl(raw)
  if (!isTauri() || !isHttpsUrl(normalized)) return normalized
  return `${MEDIA_PROXY_BASE_URL}/audio?url=${encodeURIComponent(normalized)}`
}

export function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args)
}

export async function openExternal(url: string): Promise<void> {
  if (!/^https?:\/\//i.test(url)) return
  if (isTauri()) {
    await open(url)
    return
  }
  window.open(url, '_blank', 'noopener,noreferrer')
}

export function currentWindow() {
  return getCurrentWindow()
}

export const tauriBiliApi: TauriBiliApi = {
  downloadAudio: (audioUrl, filename) => tauriInvoke('download_audio', { audioUrl, filename }),
  qrGenerate: () => tauriInvoke('qr_generate'),
  qrPoll: (qrcodeKey) => tauriInvoke('qr_poll', { qrcodeKey }),
  getCookies: () => tauriInvoke('get_cookies'),
  logout: () => tauriInvoke('logout'),
}

export const tauriWebdavApi = {
  configure: (config: WebdavConfigInput) => tauriInvoke<{ ok: boolean }>('configure_webdav', config as unknown as Record<string, unknown>),
  getConfig: () => tauriInvoke<WebdavConfigInfo>('get_webdav_config'),
  test: () => tauriInvoke<{ ok: boolean; message: string }>('test_webdav'),
  get: (relPath: string) => tauriInvoke<WebdavResult>('webdav_get', { relPath }),
  put: (relPath: string, content: string, etag?: string) => tauriInvoke<WebdavResult>('webdav_put', { relPath, content, etag }),
  clear: () => tauriInvoke<{ ok: boolean }>('clear_webdav'),
}
