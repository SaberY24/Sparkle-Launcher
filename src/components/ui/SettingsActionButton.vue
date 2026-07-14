<script setup lang="ts">
import Icon from "./Icon.vue";

type IconName = "sun" | "java" | "game" | "spinner" | "search" | "no-disabled" | "grid" | "book" | "check" | "copy" | "x" | "alert" | "info" | "play" | "folder" | "file" | "crash" | "logs" | "list";

// Botón de acción "grande" para Settings — a propósito distinto del
// AppButton/StatusButton tipo "pill" que usan los presets de resolución,
// tema, etc. Pensado para acciones con más peso (limpiar caché, revisar
// actualizaciones): badge de ícono siempre visible + fondo con tinte de
// acento por defecto, no solo en hover.
const props = withDefaults(
  defineProps<{
    status: "idle" | "loading" | "success" | "error";
    // Si el ícono que necesitas no está en Icon.vue (ej. "refresh"), no
    // pases esta prop y usa el slot #idle-icon en su lugar.
    icon?: IconName;
    idleLabel: string;
    loadingLabel?: string;
    successLabel?: string;
    errorLabel?: string;
    // "success" = verde (para "hay algo bueno que reportar", ej. update
    // disponible). "neutral" = mismo tono de acento que el estado idle, sin
    // parpadeo a verde (ej. Clear cache, que no debería "celebrar" nada).
    successStyle?: "success" | "neutral";
    disabled?: boolean;
  }>(),
  {
    loadingLabel: "Loading...",
    successLabel: "Done",
    errorLabel: "Error",
    successStyle: "success",
    disabled: false,
    icon: "check",
  }
);

const emit = defineEmits<{ (e: "click", event: MouseEvent): void }>();

function handleClick(e: MouseEvent) {
  if (!props.disabled && props.status !== "loading") {
    emit("click", e);
  }
}
</script>

<template>
  <button
    type="button"
    class="settings-action-btn"
    :class="[status, status === 'success' ? successStyle : '']"
    :disabled="disabled || status === 'loading'"
    @click="handleClick"
  >
    <span class="settings-action-btn-badge">
      <slot v-if="status === 'idle'" name="idle-icon">
        <Icon :name="icon" :size="16" />
      </slot>
      <svg v-else-if="status === 'loading'" class="spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" width="15" height="15">
        <path d="M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M21 12h4m-16 0H3m16.07 7.07l-2.83-2.83M6.17 6.17L3.34 3.34" />
      </svg>
      <svg v-else-if="status === 'success'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" width="15" height="15">
        <polyline points="20 6 9 17 4 12" />
      </svg>
      <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" width="15" height="15">
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
    </span>
    <span class="settings-action-btn-label">
      <template v-if="status === 'idle'">{{ idleLabel }}</template>
      <template v-else-if="status === 'loading'">{{ loadingLabel }}</template>
      <template v-else-if="status === 'success'">{{ successLabel }}</template>
      <template v-else>{{ errorLabel }}</template>
    </span>
  </button>
</template>

<style scoped>
.settings-action-btn {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  height: 40px;
  padding: 0 16px 0 8px;
  border-radius: 12px;
  border: 1px solid color-mix(in srgb, var(--accent-primary) 28%, var(--border-color));
  background: linear-gradient(
    135deg,
    color-mix(in srgb, var(--accent-primary) 12%, var(--bg-card)) 0%,
    color-mix(in srgb, var(--accent-primary) 4%, var(--bg-card)) 100%
  );
  color: var(--text-primary);
  font-family: inherit;
  font-weight: 700;
  font-size: 12.5px;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.08), inset 0 1px 0 rgba(255, 255, 255, 0.05);
}

.settings-action-btn-badge {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--accent-primary) 18%, transparent);
  color: var(--accent-display);
}

.settings-action-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent-primary) 55%, transparent);
  transform: translateY(-1px);
  box-shadow: 0 6px 16px color-mix(in srgb, var(--accent-primary) 18%, transparent);
}

.settings-action-btn:active:not(:disabled) {
  transform: translateY(0) scale(0.97);
}

.settings-action-btn:disabled {
  opacity: 0.65;
  cursor: not-allowed;
}

.settings-action-btn.loading .settings-action-btn-badge {
  background: color-mix(in srgb, var(--text-muted) 16%, transparent);
  color: var(--text-muted);
}

/* success: verde real, para cuando SÍ hay algo que celebrar (update found) */
.settings-action-btn.success.success .settings-action-btn-badge {
  background: color-mix(in srgb, var(--success) 20%, transparent);
  color: var(--success);
}
.settings-action-btn.success.success {
  border-color: color-mix(in srgb, var(--success) 45%, transparent);
}

/* success neutral: mismo tono de acento de siempre, sin verde */
.settings-action-btn.success.neutral .settings-action-btn-badge {
  background: color-mix(in srgb, var(--accent-primary) 18%, transparent);
  color: var(--accent-display);
}

.settings-action-btn.error .settings-action-btn-badge {
  background: color-mix(in srgb, var(--danger) 18%, transparent);
  color: var(--danger);
}
.settings-action-btn.error {
  border-color: color-mix(in srgb, var(--danger) 45%, transparent);
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.spin {
  animation: spin 1s linear infinite;
}
</style>