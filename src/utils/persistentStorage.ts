export function readStoredItem(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

export function writeStoredItem(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    // Ignore storage failures in private or restricted webviews.
  }
}

export function removeStoredItem(key: string): void {
  try {
    localStorage.removeItem(key)
  } catch {
    // Ignore storage failures in private or restricted webviews.
  }
}
