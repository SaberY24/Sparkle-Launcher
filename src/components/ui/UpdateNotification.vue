<script setup lang="ts">
type UpdateStatus = "available" | "downloading" | "ready" | "error";

withDefaults(
  defineProps<{
    visible: boolean;
    version: string;
    status?: UpdateStatus;
    notes?: string;
    progress?: number; // 0-100, used while status === 'downloading'
    error?: string;
  }>(),
  { status: "available", progress: 0 }
);

const emit = defineEmits<{
  (e: "update"): void;
  (e: "decline"): void;
  (e: "restart"): void;
  (e: "dismiss"): void;
}>();
</script>

<template>
  <transition name="toast">
    <div v-if="visible" class="update-container">
      <div class="update-card" :class="{ error: status === 'error' }">
        <div class="update-header">
          <span class="update-icon" aria-hidden="true">
            <svg v-if="status !== 'error'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="18" height="18">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="18" height="18">
              <circle cx="12" cy="12" r="10" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </span>
          <div class="update-heading">
            <span class="update-title">
              {{ status === "error" ? "Update failed" : status === "ready" ? "Update ready" : "Update available" }}
            </span>
            <span class="update-version">v{{ version }}</span>
          </div>
          <button v-if="status !== 'downloading'" class="update-close" @click="emit('dismiss')" aria-label="Dismiss">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>

        <p v-if="status === 'error'" class="update-message">{{ error || "Couldn't download the update." }}</p>
        <p v-else-if="notes" class="update-message">{{ notes }}</p>

        <div v-if="status === 'downloading'" class="update-progress">
          <div class="update-progress-bar">
            <div class="update-progress-fill" :style="{ width: (progress ?? 0) + '%' }"></div>
          </div>
          <span class="update-progress-label">Downloading… {{ Math.round(progress ?? 0) }}%</span>
        </div>

        <div v-else class="update-actions">
          <template v-if="status === 'ready'">
            <button class="update-btn update-btn-primary" @click="emit('restart')">Restart &amp; install</button>
            <button class="update-btn update-btn-ghost" @click="emit('dismiss')">Later</button>
          </template>
          <template v-else-if="status === 'error'">
            <button class="update-btn update-btn-primary" @click="emit('update')">Retry</button>
            <button class="update-btn update-btn-ghost" @click="emit('dismiss')">Dismiss</button>
          </template>
          <template v-else>
            <button class="update-btn update-btn-primary" @click="emit('update')">Update</button>
            <button class="update-btn update-btn-ghost" @click="emit('decline')">Decline</button>
          </template>
        </div>
      </div>
    </div>
  </transition>
</template>

<style scoped>
.update-container {
  position: fixed;
  right: 22px;
  bottom: 22px;
  z-index: 1000;
  width: 336px;
  max-width: calc(100vw - 44px);
  pointer-events: none;
}

.update-card {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px 16px 15px;
  border-radius: 14px;
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--bg-card) 94%, transparent) 0%,
    color-mix(in srgb, var(--bg-secondary) 88%, transparent) 100%
  );
  border: 1px solid color-mix(in srgb, var(--accent-primary) 35%, var(--border-color));
  box-shadow:
    0 16px 40px rgba(0, 0, 0, 0.32),
    0 2px 8px rgba(0, 0, 0, 0.16),
    inset 0 1px 0 rgba(255, 255, 255, 0.05);
  backdrop-filter: blur(20px) saturate(1.5);
  -webkit-backdrop-filter: blur(20px) saturate(1.5);
  pointer-events: auto;
}

.update-card.error {
  border-color: color-mix(in srgb, var(--danger) 45%, var(--border-color));
}

.update-header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.update-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  flex-shrink: 0;
  background: color-mix(in srgb, var(--accent-primary) 16%, transparent);
  color: var(--accent-primary);
}

.update-card.error .update-icon {
  background: color-mix(in srgb, var(--danger) 16%, transparent);
  color: var(--danger);
}

.update-heading {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  flex: 1;
}

.update-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-primary);
}

.update-version {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
}

.update-close {
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
  flex-shrink: 0;
  transition: opacity 0.15s ease, background 0.15s ease, color 0.15s ease;
}

.update-close:hover {
  opacity: 1;
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 10%, transparent);
}

.update-close svg {
  width: 12px;
  height: 12px;
}

.update-message {
  margin: 0;
  font-size: 12.5px;
  font-weight: 500;
  line-height: 1.4;
  color: var(--text-secondary);
  text-align: left;
}

.update-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.update-progress-bar {
  height: 6px;
  border-radius: 4px;
  background: color-mix(in srgb, var(--border-color) 60%, transparent);
  overflow: hidden;
}

.update-progress-fill {
  height: 100%;
  background: var(--accent-primary);
  transition: width 0.2s ease;
}

.update-progress-label {
  font-size: 11.5px;
  font-weight: 500;
  color: var(--text-muted);
}

.update-actions {
  display: flex;
  gap: 8px;
}

.update-btn {
  flex: 1;
  padding: 8px 12px;
  border-radius: 9px;
  border: 1px solid transparent;
  font-size: 12.5px;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s ease, border-color 0.15s ease, opacity 0.15s ease;
}

.update-btn-primary {
  background: var(--accent-primary);
  color: var(--bg-primary);
}

.update-btn-primary:hover {
  opacity: 0.9;
}

.update-btn-ghost {
  background: transparent;
  border-color: color-mix(in srgb, var(--border-color) 70%, transparent);
  color: var(--text-secondary);
}

.update-btn-ghost:hover {
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  color: var(--text-primary);
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