<script setup lang="ts">
// Botón compartido para el lenguaje visual "pill" que se repetía en varios
// lugares (resolution-preset, stat-btn, hero-folder-btn): fondo con
// degradado sutil tipo card, borde color-mix, hover con lift + glow de
// acento. A propósito NO cubre los botones con estados propios
// (loading/success/error como copy-btn, btn-clear-cache, mods-refresh-btn)
// — esos tienen suficiente lógica particular como para que forzarlos acá
// agregue más complejidad de la que ahorra.
withDefaults(
  defineProps<{
    size?: "sm" | "md";
    fullWidth?: boolean;
  }>(),
  { size: "md", fullWidth: false }
);

defineEmits<{ (e: "click", event: MouseEvent): void }>();
</script>

<template>
  <button
    type="button"
    class="app-button"
    :class="[size, { 'full-width': fullWidth }]"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style scoped>
.app-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: 8px;
  border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--bg-card) 50%, transparent),
    color-mix(in srgb, var(--bg-hover) 30%, transparent)
  );
  color: var(--text-muted);
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05), inset 0 1px 0 rgba(255, 255, 255, 0.04);
}

.app-button.md {
  height: 28px;
  padding: 0 12px;
  font-size: 11px;
}

.app-button.sm {
  height: 24px;
  padding: 0 10px;
  font-size: 10px;
}

.app-button.full-width {
  width: 100%;
  align-self: stretch;
}

.app-button:hover {
  border-color: color-mix(in srgb, var(--accent-primary) 50%, transparent);
  color: var(--accent-display);
  background: var(--accent-glow);
  transform: translateY(-1px);
  box-shadow: 0 4px 10px color-mix(in srgb, var(--accent-primary) 15%, transparent);
}

.app-button:active {
  transform: translateY(0) scale(0.96);
}
</style>