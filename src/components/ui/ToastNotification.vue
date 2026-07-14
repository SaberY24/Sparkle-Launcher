<script setup lang="ts">
import { computed } from "vue";

type ToastVariant = "good" | "bad" | "info";

const props = withDefaults(
  defineProps<{
    visible: boolean;
    message: string;
    variant?: ToastVariant;
    /** Overrides the auto-generated title (Success / Error / Info) */
    title?: string;
  }>(),
  { variant: "info" }
);

const emit = defineEmits<{ (e: "dismiss"): void }>();

const toastTitle = computed(() => {
  if (props.title) return props.title;
  if (props.variant === "good") return "Success";
  if (props.variant === "bad") return "Error";
  return "Info";
});
</script>

<template>
  <transition name="toast">
    <div v-if="visible && message" class="toast-container">
      <div class="toast" :class="variant">
        <span class="toast-icon" aria-hidden="true">
          <svg v-if="variant === 'good'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <svg v-else-if="variant === 'bad'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="16" x2="12" y2="12" />
            <line x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
        </span>
        <div class="toast-body">
          <span class="toast-title">{{ toastTitle }}</span>
          <span class="toast-message">{{ message }}</span>
        </div>
        <button class="toast-close" @click="emit('dismiss')" aria-label="Dismiss notification">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"/>
            <line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
        <div class="toast-timer" :key="message"></div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.toast-container {
  position: fixed;
  right: 22px;
  bottom: 22px;
  z-index: 999;
  width: 336px;
  max-width: calc(100vw - 44px);
  pointer-events: none;
}

.toast {
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 38px 15px 14px;
  border-radius: 14px;
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--bg-card) 94%, transparent) 0%,
    color-mix(in srgb, var(--bg-secondary) 88%, transparent) 100%
  );
  border: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
  box-shadow:
    0 16px 40px rgba(0, 0, 0, 0.32),
    0 2px 8px rgba(0, 0, 0, 0.16),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(20px) saturate(1.5);
  -webkit-backdrop-filter: blur(20px) saturate(1.5);
  overflow: hidden;
  pointer-events: auto;
}

.toast-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  flex-shrink: 0;
  margin-top: 1px;
}

.toast-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  padding-top: 1px;
}

.toast-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.toast-message {
  font-size: 12.5px;
  font-weight: 500;
  line-height: 1.4;
  color: var(--text-secondary);
  text-align: left;
  word-break: break-word;
}

.toast-close {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 22px;
  height: 22px;
  border-radius: 7px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  opacity: 0.7;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.toast-close:hover {
  opacity: 1;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 10%, transparent);
}

.toast-close svg {
  width: 12px;
  height: 12px;
}

.toast-timer {
  position: absolute;
  left: 0;
  bottom: 0;
  height: 2.5px;
  width: 100%;
  background: currentColor;
  opacity: 0.55;
  transform-origin: left;
  animation: toastCountdown 3s linear forwards;
}

@keyframes toastCountdown {
  from { transform: scaleX(1); }
  to { transform: scaleX(0); }
}

.toast.good {
  color: var(--success);
}

.toast.good .toast-icon {
  background: color-mix(in srgb, var(--success) 16%, transparent);
  color: var(--success);
}

.toast.good .toast-title {
  color: var(--success);
}

.toast.bad {
  color: var(--danger);
}

.toast.bad .toast-icon {
  background: color-mix(in srgb, var(--danger) 16%, transparent);
  color: var(--danger);
}

.toast.bad .toast-title {
  color: var(--danger);
}

.toast.info {
  color: var(--accent-primary);
}

.toast.info .toast-icon {
  background: color-mix(in srgb, var(--accent-primary) 16%, transparent);
  color: var(--accent-primary);
}

.toast.info .toast-title {
  color: var(--accent-primary);
}

.toast-enter-active,
.toast-leave-active {
  transition: all 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(20px) scale(0.94);
}
</style>