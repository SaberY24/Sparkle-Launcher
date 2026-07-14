import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";

export interface InstallProgress {
  stage: string;
  current: number;
  total: number;
  percent: number;
  detail: string;
  speed: number;
  eta: number | null;
}

export type ModpackProfile = "default" | "no_dh";

export const useLauncherStore = defineStore("launcher", () => {
  const isLaunching = ref(false);
  const launchProgress = ref(0);
  const launchStatus = ref("");
  const javaPath = ref("");
  const javaDetecting = ref(false);
  const javaArgs = ref("");
  const ram = ref(4096);
  const gameDirectory = ref("");
  const customTitlebar = ref(true);
  const sidebarCollapsed = ref(true);

  const installed = ref(false);
  const installing = ref(false);
  const installProgress = ref<InstallProgress | null>(null);

  // "Modpack Profile": lets the user always keep Distant Horizons disabled
  // ("no_dh"), regardless of jar version. Persisted in the backend
  // independently of the regular settings, and enforced again on every
  // modpack sync, so it survives both app restarts and updates.
  const modpackProfile = ref<ModpackProfile>("default");
  const modpackProfileLoading = ref(false);
  const modpackProfileSaving = ref(false);
  const modpackProfileError = ref("");

  const canLaunch = computed(() => !isLaunching.value && installed.value);

  function resetInstallProgress() {
    installProgress.value = null;
    installing.value = false;
  }

  async function play() {
    if (isLaunching.value) return;
    isLaunching.value = true;
    launchProgress.value = 0;
    launchStatus.value = "Preparing...";
    installing.value = true;

    const channel = new Channel<InstallProgress>();
    channel.onmessage = (msg) => {
      installProgress.value = msg;
      launchProgress.value = msg.percent;
      launchStatus.value = msg.detail;
    };

    try {
      await invoke("play", { 
        progressChannel: channel,
        ram: ram.value,
        javaArgs: javaArgs.value,
        javaPath: javaPath.value,
      });
      installed.value = await invoke<boolean>("check_installation");
      launchStatus.value = "Game launched successfully!";
      launchProgress.value = 100;
    } catch (e: any) {
      launchStatus.value = "Failed: " + (e?.message || e?.toString());
      console.error("Play error:", e);
    } finally {
      isLaunching.value = false;
      installing.value = false;
      setTimeout(() => {
        launchStatus.value = "";
        launchProgress.value = 0;
        installProgress.value = null;
      }, 4000);
    }
  }

  async function checkInstallation() {
    try {
      installed.value = await invoke<boolean>("check_installation");
    } catch {
      installed.value = false;
    }
  }

  async function scanJava() {
    try {
      const path = await invoke<string | null>("scan_java");
      if (path) javaPath.value = path;
    } catch {
      // No Java found
    }
  }

  async function detectJava17() {
    javaDetecting.value = true;
    try {
      const path = await invoke<string | null>("detect_java_17");
      if (path) javaPath.value = path;
    } catch {
      // fallback
    } finally {
      javaDetecting.value = false;
    }
  }

  async function loadSettings(): Promise<any> {
    try {
      const settings = await invoke<any>("load_settings");
      ram.value = settings.ram ?? 4096;
      javaPath.value = settings.java_path ?? "";
      javaArgs.value = settings.java_args ?? "";
      gameDirectory.value = settings.game_dir ?? "";
      customTitlebar.value = settings.custom_titlebar ?? true;
      sidebarCollapsed.value = settings.sidebar_collapsed ?? true;
      return settings;
    } catch {
      return {
        ram: 4096,
        java_path: "",
        game_dir: "",
        resolution: "1920 x 1080",
        fullscreen: false,
        custom_titlebar: true,
        sidebar_collapsed: true,
        java_args: "",
      };
    }
  }

  async function loadModpackProfile() {
    modpackProfileLoading.value = true;
    try {
      const profile = await invoke<string>("get_modpack_profile");
      modpackProfile.value = profile === "no_dh" ? "no_dh" : "default";
    } catch (e) {
      console.error("Failed to load the modpack profile:", e);
    } finally {
      modpackProfileLoading.value = false;
    }
  }

  async function setModpackProfile(profile: ModpackProfile) {
    if (modpackProfileSaving.value || modpackProfile.value === profile) return;
    const previous = modpackProfile.value;
    modpackProfile.value = profile;
    modpackProfileSaving.value = true;
    modpackProfileError.value = "";
    try {
      await invoke("set_modpack_profile", { profile });
    } catch (e: any) {
      modpackProfile.value = previous;
      modpackProfileError.value = e?.message || "Could not change the modpack profile.";
    } finally {
      modpackProfileSaving.value = false;
    }
  }

  return {
    isLaunching,
    launchProgress,
    launchStatus,
    javaPath,
    javaDetecting,
    javaArgs,
    ram,
    gameDirectory,
    customTitlebar,
    sidebarCollapsed,
    installed,
    installing,
    installProgress,
    modpackProfile,
    modpackProfileLoading,
    modpackProfileSaving,
    modpackProfileError,
    canLaunch,
    play,
    scanJava,
    detectJava17,
    loadSettings,
    checkInstallation,
    resetInstallProgress,
    loadModpackProfile,
    setModpackProfile,
  };
});