/** Join class names, filtering falsy values */
export function cn(...classes: (string | false | null | undefined)[]): string {
  return classes.filter(Boolean).join(" ");
}

/** Format bytes to human-readable string */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** Format a timestamp (ms) to a time string */
export function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Format token count */
export function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
  return String(n);
}

/** Format USD cost */
export function formatCost(usd: number): string {
  if (usd < 0.001) return "<$0.001";
  return `$${usd.toFixed(4)}`;
}

/** Get file icon emoji based on extension */
export function fileIcon(ext: string | null, isDir: boolean): string {
  if (isDir) return "📁";
  switch (ext?.toLowerCase()) {
    case "rs":    return "🦀";
    case "ts":
    case "tsx":   return "🔷";
    case "js":
    case "jsx":   return "🟨";
    case "json":  return "📋";
    case "toml":  return "⚙️";
    case "yaml":
    case "yml":   return "📄";
    case "md":    return "📝";
    case "css":   return "🎨";
    case "html":  return "🌐";
    case "py":    return "🐍";
    case "sh":
    case "bash":  return "🖥️";
    case "ps1":   return "💙";
    case "sql":   return "🗄️";
    case "png":
    case "jpg":
    case "svg":   return "🖼️";
    default:      return "📄";
  }
}

/** Generate a short unique ID */
export function uid(): string {
  return Math.random().toString(36).slice(2, 9);
}
