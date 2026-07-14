<script setup lang="ts">
import { ref, computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "../stores/auth";
import { useLauncherStore } from "../stores/launcher";
import AccountDropdown from "./AccountDropdown.vue";
import PlayerAvatar from "./PlayerAvatar.vue";
import SettingsModal from "./SettingsModal.vue";

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const launcher = useLauncherStore();
const showSettings = ref(false);

const isCollapsed = ref<boolean>(
  localStorage.getItem('sidebarCollapsed') === 'true' || launcher.sidebarCollapsed || false
);

// Sincronizar con el store
launcher.$subscribe(() => {
  if (launcher.sidebarCollapsed !== isCollapsed.value) {
    isCollapsed.value = launcher.sidebarCollapsed;
  }
});

const useCustomTitlebar = computed(() => launcher.customTitlebar);

const navItems = [
  { name: "Home", path: "/home", icon: "home" },
];

const iconPaths: Record<string, string> = {
  home: "M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 22V12h6v10",
};

function navigate(path: string) {
  if (path === route.path) return;
  router.push(path);
}

function openSettings() {
  showSettings.value = true;
}

function closeSettings() {
  showSettings.value = false;
}

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
  localStorage.setItem('sidebarCollapsed', String(isCollapsed.value));
  launcher.sidebarCollapsed = isCollapsed.value;
}
</script>

<template>
  <aside class="sidebar" :class="{ collapsed: isCollapsed }" role="navigation" aria-label="Main navigation">
    <nav class="nav-menu">
      <button
        v-for="item in navItems"
        :key="item.path"
        class="nav-item"
        :class="{ active: route.path === item.path }"
        @click="navigate(item.path)"
        :aria-current="route.path === item.path ? 'page' : undefined"
        v-tooltip="isCollapsed ? item.name : ''"
      >
        <span class="nav-icon" aria-hidden="true">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path :d="iconPaths[item.icon]" />
          </svg>
        </span>
        <span class="nav-label">{{ item.name }}</span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <div v-if="!useCustomTitlebar && auth.isAuthenticated" class="account-menu-wrapper">
        <AccountDropdown
          placement="top"
          align="left"
          :full-width="true"
          :use-fixed="true"
        >
          <template #trigger="{ toggle }">
            <button
              class="account-trigger"
              @click.stop="toggle"
              v-tooltip="isCollapsed ? auth.username : ''"
            >
              <PlayerAvatar
                :skin-url="auth.skinUrl"
                :texture-key="auth.skinTextureKey"
                :uuid="auth.account?.uuid || null"
                :username="auth.username"
                size="sm"
              />
              <div v-if="!isCollapsed" class="account-trigger-info">
                <span class="account-trigger-name">{{ auth.username }}</span>
                <span class="account-trigger-type">{{ auth.isPremium ? 'Premium' : 'Offline' }}</span>
              </div>
              <svg v-if="!isCollapsed" class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="6 9 12 15 18 9"/>
              </svg>
            </button>
          </template>
        </AccountDropdown>
      </div>

      <button class="footer-btn collapse-btn" @click="toggleSidebar" :aria-label="isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'" v-tooltip="isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'">
        <svg class="footer-icon" :class="{ rotated: isCollapsed }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="15 18 9 12 15 6"/>
        </svg>
        <span v-if="!isCollapsed">Collapse</span>
      </button>

      <button class="footer-btn settings-btn" @click="openSettings" aria-label="Open settings" v-tooltip="isCollapsed ? 'Settings' : ''">
        <svg class="footer-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 5 15.4a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.6a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
        </svg>
        <span v-if="!isCollapsed">Settings</span>
      </button>
    </div>
    
    <SettingsModal
      v-if="showSettings"
      @close="closeSettings"
    />
  </aside>
</template>

<style scoped>
.sidebar {
  width: 190px;
  background: color-mix(in srgb, var(--bg-secondary) 90%, transparent);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  padding: 20px 12px;
  flex-shrink: 0;
  overflow-y: auto;
  overflow-x: visible;
  transition: width 0.35s cubic-bezier(0.22, 1, 0.36, 1), padding 0.35s cubic-bezier(0.22, 1, 0.36, 1);
  /* Eliminamos backdrop-filter */
  position: relative;
  z-index: 10;
}

.sidebar.collapsed {
  width: 64px;
  padding: 20px 10px;
}

.nav-menu {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 14px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  text-align: left;
  position: relative;
  letter-spacing: -0.01em;
  -webkit-app-region: no-drag;
}

.sidebar.collapsed .nav-item {
  justify-content: center;
  padding: 12px;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
  transform: translateX(3px);
}

.nav-item.active {
  background: var(--accent-glow);
  color: var(--accent-display);
  font-weight: 600;
  transform: translateX(4px);
}

.nav-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 20px;
  background: var(--accent-primary);
  border-radius: 0 4px 4px 0;
  box-shadow: 0 0 10px var(--accent-glow);
}

.nav-icon {
  width: 19px;
  height: 19px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.nav-icon svg {
  width: 100%;
  height: 100%;
}

.nav-label {
  transition: opacity 0.25s ease;
}

.sidebar.collapsed .nav-label {
  display: none;
}

.sidebar-footer {
  margin-top: auto;
  padding-top: 14px;
  border-top: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  gap: 8px;
  position: relative;
}

.account-menu-wrapper {
  position: relative;
  width: 100%;
}

.account-trigger {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border-radius: 12px;
  border: 1px solid var(--border-color);
  background: color-mix(in srgb, var(--bg-hover) 80%, transparent);
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: left;
  font-family: inherit;
}

.sidebar.collapsed .account-trigger {
  justify-content: center;
  padding: 8px;
}

.account-trigger:hover {
  border-color: var(--border-hover);
  background: color-mix(in srgb, var(--bg-card) 80%, transparent);
}

.account-trigger-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  flex: 1;
  min-width: 0;
}

.sidebar.collapsed .account-trigger-info {
  display: none;
}

.account-trigger-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.account-trigger-type {
  font-size: 11px;
  color: var(--text-muted);
}

.chevron {
  width: 14px;
  height: 14px;
  color: var(--text-muted);
  transition: transform 0.2s ease;
  flex-shrink: 0;
}

.sidebar.collapsed .chevron {
  display: none;
}

.footer-btn {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 10px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  text-align: left;
  white-space: nowrap;
  -webkit-app-region: no-drag;
}

.sidebar.collapsed .footer-btn {
  justify-content: center;
  padding: 10px;
}

.footer-btn:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
  transform: translateX(2px);
}

.footer-btn.settings-btn:hover {
  color: var(--accent-primary);
}

.collapse-btn:hover {
  color: var(--accent-primary);
}

.footer-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  transition: transform 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}

.footer-icon.rotated {
  transform: rotate(180deg);
}

.footer-icon svg {
  width: 100%;
  height: 100%;
}
</style>