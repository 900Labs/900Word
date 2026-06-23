const allowedLinkProtocols = new Set(['http:', 'https:', 'mailto:']);

export function sanitizeEditorHref(href: string): string | undefined {
  const trimmed = href.trim();
  if (!trimmed) {
    return undefined;
  }

  try {
    const url = new URL(trimmed);
    return allowedLinkProtocols.has(url.protocol) ? trimmed : undefined;
  } catch {
    return undefined;
  }
}
