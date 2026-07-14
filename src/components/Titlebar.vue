<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useThemeStore } from "../stores/theme";
import { useAuthStore } from "../stores/auth";
import { useLauncherStore } from "../stores/launcher";
import AccountDropdown from "./AccountDropdown.vue";
import PlayerAvatar from "./PlayerAvatar.vue";
import AppIcon from "./AppIcon.vue";

const isMaximized = ref(false);
const appWindow = getCurrentWindow();
const theme = useThemeStore();
const auth = useAuthStore();
const launcher = useLauncherStore();
let unlisten: (() => void) | null = null;

onMounted(async () => {
  try {
    isMaximized.value = await appWindow.isMaximized();
    unlisten = await appWindow.onResized(async () => {
      isMaximized.value = await appWindow.isMaximized();
    });
  } catch (e) {
    console.error("Titlebar init error:", e);
  }
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

async function minimize() {
  try { await appWindow.minimize(); } catch (e) { console.error(e); }
}

async function toggleMaximize() {
  try {
    await appWindow.toggleMaximize();
    isMaximized.value = await appWindow.isMaximized();
  } catch (e) { console.error(e); }
}

async function closeWindow() {
  try { await appWindow.close(); } catch (e) { console.error(e); }
}

function onDblClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  if (target.closest(".titlebar-controls") || target.closest(".account-dropdown-wrapper")) return;
  toggleMaximize();
}

const isLight = computed(() => !theme.isDarkMode);
const useCustomTitlebar = computed(() => launcher.customTitlebar);
</script>

<template>
  <teleport to="body">
    <div
      class="titlebar"
      :class="{ light: isLight, 'native-titlebar': !useCustomTitlebar }"
      data-tauri-drag-region
      @dblclick="onDblClick"
    >
      <div class="titlebar-left">
        <div class="titlebar-logo" aria-hidden="true">
          <AppIcon />
        </div>
        <span class="titlebar-title">Sparkle</span>
        <div class="titlebar-badge" aria-label="Beta version">Beta</div>
      </div>

      <div class="titlebar-center" data-tauri-drag-region></div>

      <div class="titlebar-right">
        <AccountDropdown
          v-if="auth.isAuthenticated && useCustomTitlebar"
          placement="bottom"
        >
          <template #trigger="{ toggle }">
            <button
              class="account-trigger"
              @click.stop="toggle"
              aria-label="Account menu"
            >
              <PlayerAvatar
                :skin-url="auth.skinUrl"
                :texture-key="auth.skinTextureKey"
                :uuid="auth.account?.uuid || null"
                :username="auth.username"
                :is-premium="auth.isPremium"
                size="xs"
              />
            </button>
          </template>
        </AccountDropdown>

        <div class="titlebar-controls">
          <button class="ctrl-btn minimize" @click="minimize" v-tooltip="'Minimize'" aria-label="Minimize window">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="5" y1="12" x2="19" y2="12"/>
            </svg>
          </button>
          <button class="ctrl-btn maximize" @click="toggleMaximize" v-tooltip="'Maximize'" aria-label="Maximize window">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="3" width="18" height="18" rx="2"/>
            </svg>
          </button>
          <button class="ctrl-btn close" @click="closeWindow" v-tooltip="'Close'" aria-label="Close window">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.titlebar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 44px;
  background: var(--titlebar-bg);
  border-bottom: 1px solid var(--titlebar-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 0 0 16px;
  user-select: none;
  -webkit-app-region: drag;
  app-region: drag;
  z-index: 50;
  flex-shrink: 0;
  transition: background 0.3s ease, border-color 0.3s ease;
  /* Eliminamos backdrop-filter para reducir capas */
}

.titlebar::after {
  content: '';
  position: absolute;
  bottom: -1px;
  left: 0;
  right: 0;
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--titlebar-glow-line), transparent);
  opacity: 0.5;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 10px;
  pointer-events: none;
}

.titlebar-logo {
  width: 20px;
  height: 20px;
  color: var(--accent-primary);
  display: flex;
  align-items: center;
  justify-content: center;
  filter: drop-shadow(0 0 8px var(--accent-glow));
}

.titlebar-logo svg {
  width: 100%;
  height: 100%;
}

.titlebar-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--titlebar-text);
  letter-spacing: 0.5px;
  transition: color 0.3s ease;
}

.titlebar-badge {
  font-size: 9px;
  font-weight: 700;
  color: var(--accent-primary);
  background: var(--accent-glow);
  border: 1px solid var(--border-hover);
  padding: 1px 6px;
  border-radius: 100px;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.titlebar-center {
  flex: 1;
  height: 100%;
  -webkit-app-region: drag;
}

.titlebar-right {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 10px;
  padding: 0 6px;
}

.account-trigger {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  border: 2px solid transparent;
  background: transparent;
  cursor: pointer;
  padding: 0;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  overflow: hidden;
}

.account-trigger:hover {
  border-color: var(--border-hover);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.account-trigger :deep(.avatar-container) {
  width: 100% !important;
  height: 100% !important;
  border-radius: 8px !important;
  box-shadow: none !important;
}

.titlebar-controls {
  display: flex;
  align-items: center;
  height: 100%;
  gap: 2px;
}

.ctrl-btn {
  width: 44px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--titlebar-text-muted);
  cursor: pointer;
  transition: all 0.2s ease;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  border-radius: 6px;
  position: relative;
}

.ctrl-btn svg {
  width: 15px;
  height: 15px;
  transition: transform 0.2s ease;
}

.ctrl-btn.minimize:hover,
.ctrl-btn.maximize:hover {
  background: var(--titlebar-btn-hover);
  color: var(--titlebar-text);
}

.ctrl-btn.minimize:hover svg,
.ctrl-btn.maximize:hover svg {
  transform: scale(1.1);
}

.ctrl-btn.close:hover {
  background: rgba(244, 63, 94, 0.15);
  color: var(--danger);
}

.ctrl-btn.close:hover svg {
  transform: scale(1.15) rotate(90deg);
}

.ctrl-btn:active {
  transform: scale(0.95);
}

.titlebar.native-titlebar .titlebar-controls {
  display: none;
}

.titlebar.native-titlebar {
  padding-right: 16px;
}
</style>