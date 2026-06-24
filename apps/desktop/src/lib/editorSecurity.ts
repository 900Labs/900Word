const allowedLinkProtocols = new Set(['http:', 'https:', 'mailto:']);

export function sanitizeEditorHref(href: string): string | undefined {
  const trimmed = href.trim();
  if (!trimmed) {
    return undefined;
  }

  if (trimmed.startsWith('#')) {
    const fragment = sanitizeBookmarkId(trimmed.slice(1));
    return fragment ? `#${fragment}` : undefined;
  }

  try {
    const url = new URL(trimmed);
    return allowedLinkProtocols.has(url.protocol) ? trimmed : undefined;
  } catch {
    return undefined;
  }
}

export function sanitizeBookmarkId(value: string): string | undefined {
  const trimmed = value.trim();
  if (!/^[A-Za-z][A-Za-z0-9_-]{0,63}$/.test(trimmed)) {
    return undefined;
  }
  return trimmed;
}
