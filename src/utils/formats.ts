// Funciones de formateo compartidas

export function formatRam(mb: number): string {
  const gb = (mb / 1024).toFixed(1);
  return `${gb} GB`;
}

export function formatRamFull(mb: number): string {
  return `${mb} MB`;
}

export function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    const gb = bytes / (1024 * 1024 * 1024);
    return gb >= 1024 ? `${(gb / 1024).toFixed(2)} TB` : `${gb.toFixed(2)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(0)} KB`;
  }
  return `${bytes} B`;
}

export function formatTimeSeconds(seconds: number): string {
  if (seconds <= 0) return "Never played";
  const totalMinutes = Math.floor(seconds / 60);
  if (totalMinutes < 1) return "< 1m";
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours === 0) return `${minutes}m`;
  if (minutes === 0) return `${hours}h`;
  return `${hours}h ${minutes}m`;
}
