<script setup lang="ts">
import { onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import DropdownMenu from "../ui/DropdownMenu.vue";
import { useOptimizedScroll } from "../../composables/useOptimizedScroll";
import { useMinecraftLogs, ACTIVE_LOG_ID } from "../../composables/useMinecraftLogs";

const props = defineProps<{ gameRunning: boolean }>();
const emit = defineEmits<{ (e: "update:gameRunning", value: boolean): void }>();

const { onScroll: onLogScroll } = useOptimizedScroll({
  frameSyncInterval: 3,
  settleDelay: 150,
  minScrollDelta: 0.5,
});

const {
  minecraftLogRef,
  selectedLogFile,
  availableLogFiles,
  selectedLogLabel,
  visibleLogLines,
  copySuccess,
  logLineLevel,
  loadLogFiles,
  fetchMinecraftLog,
  copyLogContent,
  selectLogFile,
  startPolling,
  stopPolling,
} = useMinecraftLogs(() => props.gameRunning);

async function syncGameRunningState() {
  try {
    emit("update:gameRunning", await invoke<boolean>("is_game_running"));
  } catch (e) {
    // keep last known state
  }
}

// Mirrors the old `watch(isGameRunning, ...)`: refresh the log the instant
// the game's running state flips while we're looking at the active log.
watch(
  () => props.gameRunning,
  () => {
    if (selectedLogFile.value === ACTIVE_LOG_ID) fetchMinecraftLog();
  }
);

// Mirrors the old `watch(activeTab, tab => tab === 'logs' ? start : stop)`:
// HomeView mounts this component only while the Logs tab is active, so
// mount/unmount now does exactly what that watcher used to do.
onMounted(() => {
  loadLogFiles();
  startPolling(syncGameRunningState);
});
onUnmounted(() => {
  stopPolling();
});
</script>

<template>
  <main class="logs-tab">
    <div class="logs-panel minecraft-logs">
      <div class="logs-header">
        <div class="log-dropdown-wrapper">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
            <polyline points="4 17 10 11 4 5" />
            <line x1="12" y1="19" x2="20" y2="19" />
          </svg>
          <DropdownMenu align="left">
            <template #trigger="{ toggle, isOpen }">
              <button class="log-dropdown-trigger" :class="{ open: isOpen }" @click="toggle">
                <span class="log-dropdown-label">{{ selectedLogLabel }}</span>
                <svg class="log-dropdown-chevron" :class="{ rotated: isOpen }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" width="12" height="12">
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </button>
            </template>
            <template #default="{ close }">
              <div class="log-dropdown-menu">
                <button
                  class="log-dropdown-item active-log-item"
                  :class="{ active: selectedLogFile === ACTIVE_LOG_ID }"
                  @click="selectLogFile(ACTIVE_LOG_ID); close()"
                >
                  <span class="active-log-dot" :class="{ live: gameRunning }"></span>
                  <span>Active Log</span>
                </button>
                <div class="log-dropdown-divider"></div>
                <button
                  v-for="file in availableLogFiles"
                  :key="file"
                  class="log-dropdown-item"
                  :class="{ active: file === selectedLogFile }"
                  @click="selectLogFile(file); close()"
                >
                  <svg v-if="file.endsWith('.gz')" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="12" height="12">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                  <svg v-else-if="file.startsWith('crash-')" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="12" height="12">
                    <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                    <line x1="12" y1="9" x2="12" y2="13" />
                    <line x1="12" y1="17" x2="12.01" y2="17" />
                  </svg>
                  <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="12" height="12">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                    <polyline points="14 2 14 8 20 8" />
                    <line x1="16" y1="13" x2="8" y2="13" />
                    <line x1="16" y1="17" x2="8" y2="17" />
                    <polyline points="10 9 9 9 8 9" />
                  </svg>
                  <span>{{ file }}</span>
                </button>
              </div>
            </template>
          </DropdownMenu>
        </div>
        <button
          class="copy-btn"
          :class="{ success: copySuccess }"
          @click="copyLogContent"
          aria-label="Copy log"
          v-tooltip="'Copy to clipboard'"
        >
          <transition name="copy-icon" mode="out-in">
            <svg v-if="!copySuccess" key="copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
            </svg>
            <svg v-else key="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </transition>
        </button>
      </div>
      <div class="logs-content" ref="minecraftLogRef" @scroll="onLogScroll">
        <div
          v-for="(line, idx) in visibleLogLines"
          :key="idx"
          class="log-line"
          :class="logLineLevel(line) && `log-line-${logLineLevel(line)}`"
          v-once
        >{{ line.length ? line : '\u00A0' }}</div>
      </div>
    </div>
  </main>
</template>