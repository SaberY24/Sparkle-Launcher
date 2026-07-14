<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from "vue";
import { useAuthStore } from "../stores/auth";
import PlayerAvatar from "./PlayerAvatar.vue";
import AppIcon from "./AppIcon.vue";
// OJO: ajustar esta ruta si en tu proyecto DropdownMenu.vue vive en otro
// lado (acá asumo components/ui/DropdownMenu.vue, junto al resto de la UI
// compartida — igual que ToggleSwitch.vue).
import DropdownMenu from "./ui/DropdownMenu.vue";

const props = defineProps<{
  placement?: "top" | "bottom";
  align?: "left" | "right" | "center";
  fullWidth?: boolean;
  useFixed?: boolean;
}>();

const auth = useAuthStore();
const showAddAccount = ref(false);
const addAccountLoading = ref(false);
const addAccountError = ref("");
const isPlayingOffline = ref(false);
const offlineUsername = ref("");

const otherAccounts = computed(() => {
  return auth.accounts.filter((_, idx) => idx !== auth.activeAccountIndex);
});

// El posicionamiento, clic-afuera y Escape del menú en sí ahora los maneja
// DropdownMenu. Acá solo queda la lógica propia de cuentas. Nota: en modo
// fixed (sidebar), el original SIEMPRE alineaba a la izquierda del trigger
// sin importar `align` — por eso se lo forzamos acá para no cambiar el
// comportamiento existente.
const effectiveAlign = computed(() => (props.useFixed ? "left" : props.align || "right"));

function switchAccount(index: number) {
  const targetAccount = otherAccounts.value[index];
  if (!targetAccount) return;
  const realIndex = auth.accounts.findIndex(a => a.id === targetAccount.id);
  if (realIndex !== -1) {
    auth.setActiveAccount(realIndex);
  }
}

function removeAccountById(id: string) {
  auth.removeAccountById(id);
}

function logoutCurrent() {
  auth.logout();
}

function openAddAccount() {
  showAddAccount.value = true;
  addAccountError.value = "";
}

function closeAddAccount() {
  showAddAccount.value = false;
  addAccountError.value = "";
}

async function handleMicrosoftLogin() {
  addAccountLoading.value = true;
  addAccountError.value = "";
  try {
    await auth.loginMicrosoft();
    if (auth.isAuthenticated) {
      closeAddAccount();
    }
  } catch (e: any) {
    addAccountError.value = e?.message || "Microsoft login failed";
  } finally {
    addAccountLoading.value = false;
  }
}

async function handleOfflineLogin() {
  if (!offlineUsername.value.trim()) {
    addAccountError.value = "Please enter a username";
    return;
  }
  if (offlineUsername.value.length < 3 || offlineUsername.value.length > 16) {
    addAccountError.value = "Username must be 3-16 characters";
    return;
  }
  addAccountError.value = "";
  isPlayingOffline.value = true;
  try {
    await auth.loginOffline(offlineUsername.value.trim());
    if (auth.isAuthenticated) {
      closeAddAccount();
    }
  } catch (e: any) {
    addAccountError.value = e?.message || "Offline login failed";
  } finally {
    isPlayingOffline.value = false;
  }
}

function onKeydown(e: KeyboardEvent) {
  // Solo nos ocupamos acá del modal de "agregar cuenta" — el menú del
  // dropdown ya se cierra solo con Escape (ver DropdownMenu.vue), incluida
  // la lógica para no cerrar dos capas de un solo Escape.
  if (e.key === "Escape" && showAddAccount.value) {
    closeAddAccount();
  }
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
});
onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="account-dropdown-wrapper" :class="{ 'full-width': fullWidth }">
    <DropdownMenu
      :use-fixed="useFixed"
      :align="effectiveAlign"
      :full-width="fullWidth"
      :menu-width="280"
      :menu-height="320"
    >
      <template #trigger="{ toggle, isOpen }">
        <slot name="trigger" :toggle="toggle" :show="isOpen" />
      </template>

      <template #default="{ close }">
        <div class="account-dropdown" :class="{ 'full-width': fullWidth }" role="menu">
          <div class="account-current">
            <PlayerAvatar
              :skin-url="auth.skinUrl"
              :texture-key="auth.skinTextureKey"
              :uuid="auth.account?.uuid || null"
              :username="auth.username"
              :is-premium="auth.isPremium"
              size="md"
            />
            <div class="account-details">
              <span class="account-name">{{ auth.username }}</span>
              <span class="account-email">{{ auth.isPremium ? 'Microsoft Account' : 'Offline Account' }}</span>
            </div>
          </div>

          <div class="account-divider"></div>

          <div v-if="otherAccounts.length > 0" class="account-list">
            <div
              v-for="(acc, idx) in otherAccounts"
              :key="acc.id"
              class="account-item"
              role="menuitem"
            >
              <div class="account-item-main" @click="switchAccount(idx); close()">
                <PlayerAvatar
                  :skin-url="acc.skin_url || null"
                  :texture-key="acc.skin_texture_key || null"
                  :uuid="acc.uuid || null"
                  :username="acc.username"
                  :is-premium="acc.account_type === 'microsoft'"
                  size="sm"
                />
                <div class="account-item-info">
                  <span class="account-item-name">{{ acc.username }}</span>
                  <span class="account-item-type">{{ acc.account_type === 'microsoft' ? 'Microsoft' : 'Offline' }}</span>
                </div>
              </div>
              <button
                class="remove-account-btn"
                @click.stop="removeAccountById(acc.id)"
                title="Remove account"
                aria-label="Remove account"
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <polyline points="3 6 5 6 21 6"/>
                  <path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
                </svg>
              </button>
            </div>
          </div>

          <div v-if="otherAccounts.length > 0" class="account-divider"></div>

          <div class="account-actions">
            <button
              v-if="auth.accounts.length < 3"
              class="account-action add"
              @click="openAddAccount(); close()"
              role="menuitem"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="12" y1="5" x2="12" y2="19"/>
                <line x1="5" y1="12" x2="19" y2="12"/>
              </svg>
              <span>Add account</span>
            </button>

            <button
              class="account-action logout"
              @click="logoutCurrent(); close()"
              role="menuitem"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9"/>
              </svg>
              <span>Sign out of all accounts</span>
            </button>
          </div>
        </div>
      </template>
    </DropdownMenu>

    <teleport to="body">
      <transition name="modal">
        <div v-if="showAddAccount" class="add-account-overlay" @click.self="closeAddAccount">
          <div class="add-account-box" role="dialog" aria-modal="true" aria-labelledby="add-acc-title">
            <div class="bg-layer">
              <div class="bg-gradient"></div>
              <div class="bg-grid"></div>
            </div>

            <button class="modal-close" @click="closeAddAccount" aria-label="Close">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18"/>
                <line x1="6" y1="6" x2="18" y2="18"/>
              </svg>
            </button>

            <div class="add-account-center">
              <div class="logo-mark" aria-hidden="true">
                <AppIcon />
              </div>

              <h2 id="add-acc-title" class="brand-title">Add Account</h2>

              <div class="login-actions">
                <button
                  class="btn-microsoft"
                  :disabled="addAccountLoading"
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

                <div class="offline-section">
                  <div class="input-group">
                    <label for="add-offline-username">Offline Username</label>
                    <input
                      id="add-offline-username"
                      v-model="offlineUsername"
                      type="text"
                      placeholder="Steve"
                      maxlength="16"
                      autocomplete="username"
                      @keyup.enter="handleOfflineLogin"
                      :disabled="isPlayingOffline"
                    />
                    <span class="hint">3-16 characters</span>
                  </div>
                  <button
                    class="btn-offline"
                    :disabled="isPlayingOffline"
                    @click="handleOfflineLogin"
                    aria-label="Play offline"
                  >
                    <span v-if="isPlayingOffline" class="btn-spinner"></span>
                    <span v-else>Play Offline</span>
                  </button>
                </div>
              </div>

              <transition name="shake">
                <div v-if="addAccountError" class="error-toast" role="alert">
                  <svg class="icon-sm" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"/>
                    <line x1="12" y1="8" x2="12" y2="12"/>
                    <line x1="12" y1="16" x2="12.01" y2="16"/>
                  </svg>
                  <span>{{ addAccountError }}</span>
                </div>
              </transition>
            </div>
          </div>
        </div>
      </transition>
    </teleport>
  </div>
</template>

<style scoped>
.account-dropdown-wrapper {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.account-dropdown-wrapper.full-width {
  display: flex;
  width: 100%;
}

.account-dropdown {
  /* position/top/left/right/z-index ahora los aplica DropdownMenu.vue (su
     .dropdown-menu-panel), acá solo queda lo visual */
  width: 280px;
  background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-secondary) 100%);
  border: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
  border-radius: 14px;
  box-shadow:
    0 6px 20px rgba(0, 0, 0, 0.2),
    inset 0 1px 0 rgba(255, 255, 255, 0.04),
    inset 0 -1px 0 rgba(0, 0, 0, 0.1);
  padding: 10px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.account-dropdown-wrapper.full-width .dropdown-menu-panel:not(.fixed) .account-dropdown {
  width: 100%;
  min-width: 260px;
  max-width: 300px;
}

.account-current {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 4px;
}

.account-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.account-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}

.account-email {
  font-size: 12px;
  color: var(--text-muted);
}

.account-divider {
  height: 1px;
  background: color-mix(in srgb, var(--border-color) 45%, transparent);
  margin: 4px 0;
}

.account-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.account-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px;
  border-radius: 8px;
  background: transparent;
  transition: all 0.2s ease;
  text-align: left;
  width: 100%;
}

.account-item-main {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 8px;
  border-radius: 8px;
  flex: 1;
  cursor: pointer;
  border: none;
  background: transparent;
  font-family: inherit;
  color: inherit;
}

.account-item-main:hover {
  background: var(--bg-hover);
}

.account-item-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.account-item-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.account-item-type {
  font-size: 11px;
  color: var(--text-muted);
}

.remove-account-btn {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.remove-account-btn:hover {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  color: var(--danger);
}

.remove-account-btn svg {
  width: 14px;
  height: 14px;
}

.account-actions {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.account-action {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 8px;
  border: none;
  background: transparent;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  text-align: left;
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  width: 100%;
}

.account-action:hover {
  background: var(--bg-hover);
}

.account-action.add {
  color: var(--accent-primary);
}

.account-action.add:hover {
  background: var(--accent-glow);
}

.account-action.logout {
  color: var(--danger);
}

.account-action.logout:hover {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
}

.account-action svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.modal-close {
  position: absolute;
  top: 16px;
  right: 16px;
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
  z-index: 2;
}

.modal-close:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.modal-close svg {
  width: 18px;
  height: 18px;
}

.add-account-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(10px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  padding: 20px;
}

.add-account-box {
  width: 420px;
  max-width: 90vw;
  max-height: 90vh;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: 24px;
  padding: 0;
  overflow: hidden;
  position: relative;
  animation: modalIn 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  display: flex;
  flex-direction: column;
}

@keyframes modalIn {
  from { opacity: 0; transform: scale(0.96) translateY(10px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.bg-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
  z-index: 0;
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

.add-account-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  z-index: 1;
  width: 100%;
  padding: 48px 36px 40px;
  overflow-y: auto;
}

.logo-mark {
  width: 56px;
  height: 56px;
  margin-bottom: 4px;
  opacity: 0.8;
  color: var(--accent-primary);
}

.logo-mark svg {
  width: 100%;
  height: 100%;
}

.brand-title {
  font-size: 28px;
  font-weight: 800;
  letter-spacing: -0.5px;
  color: var(--text-primary);
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
  border-radius: 12px;
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

.offline-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
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

.btn-offline {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  width: 100%;
  padding: 12px 20px;
  border-radius: 12px;
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

.btn-spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
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

@keyframes spin {
  to { transform: rotate(360deg); }
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