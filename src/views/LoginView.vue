<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import AppIcon from "../components/AppIcon.vue";

const router = useRouter();
const auth = useAuthStore();

const showOfflineModal = ref(false);
const offlineUsername = ref("");
const offlineError = ref("");
const isPlayingOffline = ref(false);

async function handleMicrosoftLogin() {
  await auth.loginMicrosoft();
  if (auth.isAuthenticated) {
    router.push("/home");
  }
}

async function handleOfflineLogin() {
  if (!offlineUsername.value.trim()) {
    offlineError.value = "Please enter a username";
    return;
  }
  if (offlineUsername.value.length < 3 || offlineUsername.value.length > 16) {
    offlineError.value = "Username must be 3-16 characters";
    return;
  }
  offlineError.value = "";
  isPlayingOffline.value = true;
  try {
    await auth.loginOffline(offlineUsername.value.trim());
    if (auth.isAuthenticated) {
      closeModal();
      router.push("/home");
    }
  } finally {
    isPlayingOffline.value = false;
  }
}

function closeModal() {
  showOfflineModal.value = false;
  offlineUsername.value = "";
  offlineError.value = "";
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape" && showOfflineModal.value) {
    closeModal();
  }
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));
</script>

<template>
  <div class="login-view">
    <div class="bg-layer">
      <div class="bg-gradient"></div>
      <div class="bg-grid"></div>
    </div>

    <div class="login-center">
      <div class="logo-mark" aria-hidden="true">
        <AppIcon />
      </div>

      <h1 class="brand-title">Sparkle</h1>
      <p class="brand-subtitle">Minecraft Launcher</p>

      <div class="login-actions">
        <button
          class="btn-microsoft"
          :disabled="auth.loading"
          @click="handleMicrosoftLogin"
          aria-label="Sign in with Microsoft"
        >
          <svg class="ms-icon" viewBox="0 0 21 21">
            <rect x="1" y="1" width="9" height="9" fill="#f25022"/>
            <rect x="1" y="11" width="9" height="9" fill="#00a4ef"/>
            <rect x="11" y="1" width="9" height="9" fill="#7fba00"/>
            <rect x="11" y="11" width="9" height="9" fill="#ffb900"/>
          </svg>
          <span>Sign in with Microsoft</span>
        </button>

        <div class="divider">
          <span>or</span>
        </div>

        <button
          class="btn-offline"
          :disabled="auth.loading"
          @click="showOfflineModal = true"
          aria-label="Play offline"
        >
          <span>Play Offline</span>
        </button>
      </div>

      <transition name="shake">
        <div v-if="auth.error" class="error-toast" role="alert">
          <svg class="icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" y1="8" x2="12" y2="12"/>
            <line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>{{ auth.error }}</span>
          <button class="error-close" @click="auth.clearError" aria-label="Dismiss error">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </transition>

      <span class="version">Sparkle v0.9.0</span>
    </div>

    <div v-if="auth.loading" class="loading-overlay">
      <div class="spinner"></div>
      <span class="loading-title">{{ auth.statusMessage }}</span>
      <span class="loading-hint">A popup window should have opened</span>
    </div>

    <transition name="modal">
      <div v-if="showOfflineModal" class="modal-overlay" @click.self="closeModal">
        <div class="modal-box" role="dialog" aria-modal="true" aria-labelledby="offline-title">
          <div class="modal-header">
            <h3 id="offline-title">Play Offline</h3>
            <p>Enter a username to continue without an account</p>
            <button class="modal-close" @click="closeModal" aria-label="Close modal">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>
          </div>
          <div class="modal-body">
            <div class="input-group">
              <label for="offline-username">Username</label>
              <input
                id="offline-username"
                v-model="offlineUsername"
                type="text"
                placeholder="Steve"
                maxlength="16"
                autocomplete="username"
                @keyup.enter="handleOfflineLogin"
              />
              <span class="hint">3-16 characters</span>
            </div>
            <transition name="shake">
              <div v-if="offlineError" class="modal-error" role="alert">
                <svg class="icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <circle cx="12" cy="12" r="10"/>
                  <line x1="12" y1="8" x2="12" y2="12"/>
                  <line x1="12" y1="16" x2="12.01" y2="16"/>
                </svg>
                <span>{{ offlineError }}</span>
              </div>
            </transition>
          </div>
          <div class="modal-footer">
            <button class="btn-cancel" @click="closeModal">Cancel</button>
            <button class="btn-play" :disabled="isPlayingOffline" @click="handleOfflineLogin">
              <span v-if="isPlayingOffline" class="btn-spinner"></span>
              <span v-else>Play</span>
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.login-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  overflow: hidden;
  background: var(--bg-primary);
}

.bg-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.bg-gradient {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(ellipse 80% 50% at 50% 0%, color-mix(in srgb, var(--accent-primary) 4%, transparent), transparent),
    radial-gradient(ellipse 60% 40% at 50% 100%, color-mix(in srgb, var(--accent-tertiary) 3%, transparent), transparent);
}

.bg-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(255,255,255,0.012) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255,255,255,0.012) 1px, transparent 1px);
  background-size: 64px 64px;
  -webkit-mask-image: radial-gradient(ellipse 70% 60% at 50% 50%, black, transparent);
  mask-image: radial-gradient(ellipse 70% 60% at 50% 50%, black, transparent);
}

.login-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  z-index: 1;
  width: 100%;
  max-width: 360px;
  padding: 24px;
}

.logo-mark {
  width: 64px;
  height: 64px;
  margin-bottom: 4px;
  opacity: 0.8;
  color: var(--accent-primary);
}

.logo-mark svg {
  width: 100%;
  height: 100%;
}

.brand-title {
  font-size: 36px;
  font-weight: 800;
  letter-spacing: -1px;
  color: var(--text-primary);
}

.brand-subtitle {
  font-size: 13px;
  color: var(--text-muted);
  font-weight: 500;
  margin-top: -6px;
  margin-bottom: 20px;
  letter-spacing: 2px;
  text-transform: uppercase;
}

.login-actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 100%;
}

.btn-microsoft {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  width: 100%;
  padding: 12px 20px;
  border-radius: 6px;
  border: none;
  background: #0078d4;
  color: white;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  font-family: inherit;
}

.btn-microsoft:hover {
  background: #006cbd;
  transform: translateY(-1px);
}

.btn-microsoft:active {
  transform: translateY(0);
}

.btn-microsoft:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.ms-icon {
  width: 18px;
  height: 18px;
  background: white;
  border-radius: 2px;
  padding: 2px;
}

.divider {
  display: flex;
  align-items: center;
  gap: 14px;
  color: var(--text-dim);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin: 2px 0;
}

.divider::before,
.divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: var(--border-color);
}

.btn-offline {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 10px 20px;
  border-radius: 6px;
  border: 1px solid var(--border-color);
  background: transparent;
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  font-family: inherit;
}

.btn-offline:hover {
  border-color: var(--accent-primary);
  color: var(--accent-primary);
  background: var(--accent-glow);
}

.btn-offline:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.icon-sm {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.error-toast {
  margin-top: 6px;
  padding: 10px 14px;
  background: var(--danger-dim);
  border: 1px solid rgba(255, 71, 87, 0.15);
  border-radius: 8px;
  color: var(--danger);
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.error-close {
  margin-left: auto;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--danger);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease;
  padding: 0;
}

.error-close:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.error-close svg {
  width: 14px;
  height: 14px;
}

.version {
  margin-top: 28px;
  font-size: 11px;
  color: var(--text-dim);
  font-weight: 500;
}

.loading-overlay {
  position: fixed;
  inset: 0;
  background: var(--bg-primary);
  opacity: 0.95;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  z-index: 50;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 2px solid var(--accent-glow);
  border-top-color: var(--accent-primary);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.loading-title {
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 600;
}

.loading-hint {
  color: var(--text-muted);
  font-size: 12px;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(12px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 20px;
}

.modal-box {
  width: 400px;
  max-width: 90vw;
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 20px;
  padding: 28px;
  animation: modalIn 0.25s ease-out;
  position: relative;
}

@keyframes modalIn {
  from { opacity: 0; transform: translateY(8px) scale(0.98); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}

.modal-header {
  position: relative;
  margin-bottom: 24px;
  padding-right: 52px;
}

.modal-header h3 {
  font-size: 20px;
  font-weight: 700;
  margin-bottom: 4px;
  color: var(--text-primary);
}

.modal-header p {
  font-size: 13px;
  color: var(--text-muted);
}

.modal-close {
  position: absolute;
  top: 0;
  right: 0;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease;
  padding: 0;
}

.modal-close:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.modal-close svg {
  width: 15px;
  height: 15px;
}

.modal-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}

.input-group label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}

.input-group input {
  width: 100%;
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid var(--border-color);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.input-group input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.input-group input::placeholder {
  color: var(--text-dim);
}

.hint {
  font-size: 11px;
  color: var(--text-dim);
}

.modal-error {
  padding: 8px 10px;
  background: var(--danger-dim);
  border: 1px solid rgba(255, 71, 87, 0.15);
  border-radius: 8px;
  color: var(--danger);
  font-size: 13px;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}

.modal-footer {
  display: flex;
  gap: 8px;
}

.btn-cancel,
.btn-play {
  flex: 1;
  padding: 10px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  border: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-cancel {
  background: var(--bg-hover);
  color: var(--text-muted);
  border: 1px solid var(--border-color);
}

.btn-cancel:hover {
  background: var(--bg-card-hover);
  color: var(--text-secondary);
}

.btn-play {
  background: var(--accent-secondary);
  color: #fff;
  font-weight: 700;
}

.btn-play:hover {
  background: var(--accent-tertiary);
  transform: translateY(-1px);
}

.btn-play:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  transform: none;
}

.btn-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.shake-enter-active,
.shake-leave-active {
  transition: all 0.3s ease;
}

.shake-enter-from,
.shake-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}
</style>