<script setup lang="ts">
// Unifica los ~14 bloques de "estado vacío" que se repetían casi idénticos
// en mods/shaders/resourcepacks/changelog (cargando, sin resultados de
// búsqueda, sin items deshabilitados, vacío genérico) — mismo esqueleto,
// solo cambia el ícono, el texto, y si hay un botón de acción.
defineProps<{
  icon: "spinner" | "search" | "no-disabled" | "grid" | "book";
  message: string;
  actionLabel?: string;
}>();

defineEmits<{ (e: "action"): void }>();
</script>

<template>
  <div class="empty-state">
    <svg v-if="icon === 'spinner'" class="spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" width="28" height="28">
      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
    </svg>
    <svg v-else-if="icon === 'search'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="28" height="28">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
    <svg v-else-if="icon === 'no-disabled'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="28" height="28">
      <circle cx="12" cy="12" r="10" />
      <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
    </svg>
    <svg v-else-if="icon === 'book'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" width="28" height="28">
      <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
      <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
    </svg>
    <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" width="28" height="28">
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
    </svg>
    <span>{{ message }}</span>
    <button v-if="actionLabel" class="empty-state-action" @click="$emit('action')">{{ actionLabel }}</button>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 56px 20px;
  color: var(--text-dim);
  font-size: 13px;
  text-align: center;
}

.empty-state-action {
  border: none;
  background: var(--accent-glow);
  color: var(--accent-display);
  font-size: 11px;
  font-weight: 700;
  padding: 6px 14px;
  border-radius: 100px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.empty-state-action:hover {
  background: color-mix(in srgb, var(--accent-primary) 25%, transparent);
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.spin {
  animation: spin 1s linear infinite;
}
</style>