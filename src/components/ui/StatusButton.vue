<script setup lang="ts">
// Botón con múltiples estados (idle, loading, success, error) para acciones asíncronas.
// Unifica el patrón del botón "Clear mods cache" en SettingsModal.

const props = withDefaults(
  defineProps<{
    status: "idle" | "loading" | "success" | "error";
    idleLabel: string;
    loadingLabel?: string;
    successLabel?: string;
    errorLabel?: string;
    size?: "sm" | "md";
    disabled?: boolean;
  }>(),
  {
    loadingLabel: "Loading...",
    successLabel: "Success",
    errorLabel: "Error",
    size: "md",
    disabled: false
  }
);

const emit = defineEmits<{ (e: "click", event: MouseEvent): void }>();

function handleClick(e: MouseEvent) {
  if (!props.disabled) {
    emit("click", e);
  }
}
</script>

<template>
  <button
    type="button"
    class="status-button"
    :class="[size, status]"
    :disabled="disabled || status === 'loading'"
    @click="handleClick"
  >
    <!-- Idle: Icono por defecto (heredado del slot) -->
    <slot v-if="status === 'idle'" name="idle-icon" />
    <!-- Loading: Spinner -->
    <svg
      v-else-if="status === 'loading'"
      class="spin"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
    >
      <path d="M12 2v4m0 12v4M4.93 4.93l2.83 2.83m8.48 8.48l2.83 2.83M21 12h4m-16 0H3m16.07 7.07l-2.83-2.83M6.17 6.17L3.34 3.34" />
    </svg>
    <!-- Success: Checkmark -->
    <svg
      v-else-if="status === 'success'"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2.5"
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
    <!-- Error: X circle -->
    <svg
      v-else-if="status === 'error'"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="8" x2="12" y2="12" />
      <line x1="12" y1="16" x2="12.01" y2="16" />
    </svg>

    <!-- Label -->
    <span class="status-button-label">
      <slot v-if="status === 'idle'" name="idle-label">{{ idleLabel }}</slot>
      <span v-else-if="status === 'loading'">{{ loadingLabel }}</span>
      <span v-else-if="status === 'success'">{{ successLabel }}</span>
      <span v-else>{{ errorLabel }}</span>
    </span>
  </button>
</template>

<style scoped>
.status-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: 8px;
  border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
  background: linear-gradient(
    180deg,
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
  height: 28px;
  padding: 0 12px;
  font-size: 11px;
}

.status-button.sm {
  height: 24px;
  padding: 0 10px;
  font-size: 10px;
}

.status-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.status-button:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent-primary) 50%, transparent);
  color: var(--accent-display);
  background: var(--accent-glow);
  transform: translateY(-1px);
  box-shadow: 0 4px 10px color-mix(in srgb, var(--accent-primary) 15%, transparent);
}

.status-button:active:not(:disabled) {
  transform: translateY(0) scale(0.96);
}

/* Estado loading */
.status-button.loading {
  color: var(--text-muted);
}

/* Estado success */
.status-button.success {
  border-color: color-mix(in srgb, var(--success) 50%, transparent);
  color: var(--success);
  background: color-mix(in srgb, var(--success) 10%, transparent);
}

.status-button.success:hover:not(:disabled) {
  border-color: var(--success);
  color: var(--success);
  background: color-mix(in srgb, var(--success) 20%, transparent);
}

/* Estado error */
.status-button.error {
  border-color: color-mix(in srgb, var(--danger) 50%, transparent);
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}

.status-button.error:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 20%, transparent);
}

.status-button-label {
  display: inline;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.spin {
  animation: spin 1s linear infinite;
  width: 14px;
  height: 14px;
}
</style>
