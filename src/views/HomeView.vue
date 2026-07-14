<script setup lang="ts">
import { onMounted, ref } from "vue";
import Icon from "../components/ui/Icon.vue";
import UpdateNotification from "../components/ui/UpdateNotification.vue";
import PlayTab from "../components/tabs/PlayTab.vue";
import LogsTab from "../components/tabs/LogsTab.vue";
import ChangelogTab from "../components/tabs/ChangelogTab.vue";
import { useAppUpdater } from "../composables/useAppUpdater";

const emit = defineEmits<{ (e: "openSettings", section: string): void }>();
const appUpdater = useAppUpdater();

const activeTab = ref<"play" | "logs" | "changelog">("play");

// Shared between PlayTab (sets it on play/stop) and LogsTab (reads it to
// decide whether "Active Log" has anything to show).
const isGameRunning = ref(false);

onMounted(() => {
  // Comprobación silenciosa al iniciar: si hay una actualización, el banner
  // de abajo la muestra sola. No bloquea ni interrumpe nada.
  appUpdater.checkForUpdate({ silent: true });
});
</script>

<template>
  <div class="home-view">
    <div class="home-tabs">
      <button class="home-tab" :class="{ active: activeTab === 'play' }" @click="activeTab = 'play'">
        <Icon name="game" :size="24" />
        Play
      </button>
      <button class="home-tab" :class="{ active: activeTab === 'logs' }" @click="activeTab = 'logs'">
        <Icon name="logs" :size="24" />
        Logs
      </button>
      <button class="home-tab" :class="{ active: activeTab === 'changelog' }" @click="activeTab = 'changelog'">
        <Icon name="book" :size="24" />
        Changelog
      </button>
    </div>

    <PlayTab :active="activeTab === 'play'" v-model:game-running="isGameRunning" @open-settings="(section) => emit('openSettings', section)" />

    <LogsTab v-if="activeTab === 'logs'" v-model:game-running="isGameRunning" />

    <ChangelogTab v-if="activeTab === 'changelog'" />

    <!-- Vive fuera de las tabs a propósito: una actualización de la app es
         relevante sin importar si estás en Play, Logs o Changelog. -->
    <UpdateNotification
      :visible="['available', 'downloading', 'ready', 'error'].includes(appUpdater.status.value)"
      :version="appUpdater.version.value"
      :status="(appUpdater.status.value as 'available' | 'downloading' | 'ready' | 'error')"
      :notes="appUpdater.notes.value"
      :progress="appUpdater.progress.value"
      :error="appUpdater.error.value"
      @update="appUpdater.downloadAndInstall"
      @decline="appUpdater.decline"
      @restart="appUpdater.restartNow"
      @dismiss="appUpdater.dismiss"
    />
  </div>
</template>

<style src="./HomeView.css"></style>