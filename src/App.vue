<script setup lang="ts">
import { onMounted, onUnmounted, ref, onErrorCaptured, computed, watch } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "./stores/auth";
import { useLauncherStore } from "./stores/launcher";
import { useThemeStore } from "./stores/theme";
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "./components/Sidebar.vue";
import Titlebar from "./components/Titlebar.vue";
import SettingsModal from "./components/SettingsModal.vue";

const auth = useAuthStore();
const launcher = useLauncherStore();
const theme = useThemeStore();
const router = useRouter();
const useCustomTitlebar = computed(() => launcher.customTitlebar);
const appError = ref<string | null>(null);
const isInitializing = ref(true);
const showSettings = ref(false);
const settingsInitialSection = ref("appearance");

onErrorCaptured((err) => {
  appError.value = err instanceof Error ? err.message : "Unknown error";
  console.error("App error:", err);
  return false;
});

watch(() => auth.isAuthenticated, (newVal, oldVal) => {
  if (oldVal === true && newVal === false) {
    router.push("/");
  }
});

onMounted(async () => {
  await auth.loadSession();
  if (auth.isAuthenticated && router.currentRoute.value.path === "/") {
    router.replace("/home");
  }
  await launcher.loadSettings();
  try {
    const settings = await invoke<any>("load_settings");
    theme.initFromSettings(settings);
  } catch {
    theme.applyTheme();
  }
  isInitializing.value = false;
});

function dismissError() {
  appError.value = null;
}

function openSettings(section: string = "appearance") {
  settingsInitialSection.value = section;
  showSettings.value = true;
}

function closeSettings() {
  showSettings.value = false;
}

// Manejo del redimensionado para evitar repintados excesivos
let resizeSettleTimer: number | null = null;

function handleWindowResize() {
  document.documentElement.classList.add("is-resizing");
  if (resizeSettleTimer) window.clearTimeout(resizeSettleTimer);
  resizeSettleTimer = window.setTimeout(() => {
    document.documentElement.classList.remove("is-resizing");
    void document.body.offsetHeight;
  }, 150);
}

onMounted(() => {
  window.addEventListener("resize", handleWindowResize);
});

onUnmounted(() => {
  window.removeEventListener("resize", handleWindowResize);
  if (resizeSettleTimer) window.clearTimeout(resizeSettleTimer);
});
</script>

<template>
  <div class="app-wrapper" :class="{ 'native-titlebar': !useCustomTitlebar }">
    <Titlebar v-if="useCustomTitlebar" @openSettings="openSettings" />
    <div class="app-body" :class="{ 'with-titlebar': useCustomTitlebar, 'no-titlebar': !useCustomTitlebar }">
      <Sidebar v-if="auth.isAuthenticated" @openSettings="openSettings" />
      <main class="main-content" :class="{ 'full-width': !auth.isAuthenticated, 'with-sidebar': auth.isAuthenticated }">
        <router-view v-slot="{ Component }">
          <transition name="page" mode="out-in">
            <component :is="Component" @openSettings="openSettings" />
          </transition>
        </router-view>
      </main>
    </div>

    <SettingsModal
      v-if="showSettings"
      :initialSection="settingsInitialSection"
      @close="closeSettings"
    />

    <transition name="fade">
      <div v-if="appError" class="app-error-toast" @click="dismissError">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"/>
          <line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
        <span>{{ appError }}</span>
        <button class="error-dismiss" aria-label="Dismiss error">×</button>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.app-wrapper {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: var(--bg-primary);
  overflow: hidden;
  position: relative;
}

.app-wrapper.native-titlebar {
  border-radius: 0;
}

.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.app-body.with-titlebar {
  margin-top: 44px;
}

.app-body.no-titlebar {
  margin-top: 0;
}

.main-content {
  flex: 1;
  overflow: hidden;
  position: relative;
  container-type: inline-size;
  container-name: content-area;
  min-width: 0;
}

.main-content.full-width {
  width: 100%;
}

.main-content.with-sidebar {
  margin-left: 0;
}

.page-enter-active,
.page-leave-active {
  transition: all 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.page-enter-from {
  opacity: 0;
  transform: translateY(12px) scale(0.992);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-8px) scale(0.992);
}

.app-error-toast {
  position: fixed;
  bottom: 24px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 20px;
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--danger) 20%, transparent);
  border-radius: 14px;
  color: var(--danger);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  z-index: 100;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25), 0 0 0 1px rgba(255,255,255,0.03);
  animation: slideUp 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.app-error-toast svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.error-dismiss {
  margin-left: 8px;
  background: none;
  border: none;
  color: var(--danger);
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
  line-height: 1;
  opacity: 0.7;
  transition: opacity 0.15s;
}

.error-dismiss:hover {
  opacity: 1;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@keyframes slideUp {
  from { opacity: 0; transform: translateX(-50%) translateY(20px); }
  to { opacity: 1; transform: translateX(-50%) translateY(0); }
}
</style>

<style>
/* Desactivamos blur y transiciones durante el redimensionado */
html.is-resizing {
  --blur-mult: 0;
}

html.is-resizing * {
  transition: none !important;
  animation: none !important;
}

html.is-resizing .settings-modal,
html.is-resizing .add-account-overlay,
html.is-resizing .modal-overlay {
  backdrop-filter: none !important;
  -webkit-backdrop-filter: none !important;
}
</style>