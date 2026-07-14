<script setup lang="ts">
import { onMounted, computed, ref, watch, onUnmounted, shallowRef, type ComponentPublicInstance } from "vue";
import { useAuthStore } from "../../stores/auth";
import { useLauncherStore, type InstallProgress } from "../../stores/launcher";
import { useThemeStore } from "../../stores/theme";
import { invoke, Channel } from "@tauri-apps/api/core";
import ToggleSwitch from "../ui/ToggleSwitch.vue";
import SearchBox from "../ui/SearchBox.vue";
import EmptyState from "../ui/EmptyState.vue";
import AppButton from "../ui/AppButton.vue";
import Icon from "../ui/Icon.vue";
import ToastNotification from "../ui/ToastNotification.vue";
import { useDebouncedValue } from "../../composables/useDebouncedValue";
import { formatRam, formatTimeSeconds } from "../../utils/formats";
import { countDisabled } from "../../utils/array";
import { useCopyToClipboard } from "../../composables/useCopyToClipboard";
import { useFilteredList, useFilteredMods } from "../../composables/useFilteredList";
import { useToggleItem } from "../../composables/useToggleItem";
import { useShowInFolder } from "../../composables/useShowInFolder";
import { useClearSearch } from "../../composables/useClearSearch";
import { useToastDismiss } from "../../composables/useToastDismiss";

// `active` only drives the v-show below (this component stays mounted the
// whole time HomeView is, exactly like when this code lived directly in
// HomeView, so mods/shaders/install state never resets on tab switches).
// `gameRunning` is shared with LogsTab, so HomeView owns the source of
// truth and both tabs sync to it via v-model.
const props = defineProps<{ active: boolean; gameRunning: boolean }>();
const emit = defineEmits<{
  (e: "update:gameRunning", value: boolean): void;
  (e: "openSettings", section: string): void;
}>();

const auth = useAuthStore();
const launcher = useLauncherStore();
const themeStore = useThemeStore();
const { toastVariant, scheduleToastDismiss, dismissToast, disposeToastTimer } = useToastDismiss();

const statusPollInterval = ref<number | null>(null);
const installingJava = ref(false);
const javaInstallError = ref("");

const MINECRAFT_VERSION = "1.20.1";
const FORGE_VERSION = "47.4.20";
const SERVER_IP = "149.130.188.154";

const { copied: ipCopied, copy: copyServerIp } = useCopyToClipboard();

interface ModInfo {
  fileName: string;
  modId: string;
  name: string;
  version: string;
  description: string | null;
  fingerprint: number;
  enabled: boolean;
  iconUrl?: string;
}

interface PackItem {
  fileName: string;
  name: string;
  enabled: boolean;
  iconUrl?: string;
  iconData?: string;
}

const contentTab = ref<"mods" | "shaders" | "resourcepacks">("mods");

const mods = shallowRef<ModInfo[]>([]);
const modsLoading = ref(false);
const modsError = ref("");
const modToggleError = ref("");

const modSearch = ref("");
const debouncedModSearch = useDebouncedValue(modSearch);

const clearModSearch = useClearSearch(modSearch, debouncedModSearch);

const javaVersionInfo = ref<{ path: string; version: string; major: number; vendor: string } | null>(null);

const ramDisplay = computed(() => formatRam(launcher.ram));

async function loadJavaInfo() {
  if (!launcher.javaPath) {
    javaVersionInfo.value = null;
    return;
  }
  try {
    const list = await invoke<any[]>("list_java_installations");
    const found = list.find((j: any) => j.path === launcher.javaPath);
    if (found) {
      javaVersionInfo.value = {
        path: found.path,
        version: found.version,
        major: found.major,
        vendor: found.vendor || "Unknown",
      };
    } else {
      javaVersionInfo.value = {
        path: launcher.javaPath,
        version: "Custom",
        major: 0,
        vendor: "Unknown",
      };
    }
  } catch (e) {
    console.error("Failed to load Java info:", e);
    javaVersionInfo.value = null;
  }
}

watch(() => launcher.javaPath, loadJavaInfo);

const showDisabledOnly = ref(false);

function toggleDisabledOnlyFilter() {
  showDisabledOnly.value = !showDisabledOnly.value;
}

const disabledModsCount = countDisabled(mods);

const filteredMods = useFilteredMods(mods, debouncedModSearch, showDisabledOnly);

let loadModsInFlight = false;

async function loadMods() {
  if (loadModsInFlight) return;
  loadModsInFlight = true;
  modsLoading.value = true;
  modsError.value = "";
  const modMap = new Map<string, ModInfo>();

  try {
    const channel = new Channel<ModInfo[]>();
    channel.onmessage = (batch: ModInfo[]) => {
      for (const mod of batch) {
        const existing = modMap.get(mod.fileName);
        if (existing) {
          const previousIcon = existing.iconUrl;
          Object.assign(existing, mod);
          existing.iconUrl = mod.iconUrl ?? previousIcon;
        } else {
          modMap.set(mod.fileName, { ...mod });
        }
      }
      mods.value = Array.from(modMap.values());
    };

    await invoke("list_installed_mods", { progress: channel });
  } catch (e: any) {
    modsError.value = e?.message || "Could not load the mods list.";
  } finally {
    modsLoading.value = false;
    loadModsInFlight = false;
  }
}

const modsRefreshing = ref(false);

async function refreshMods() {
  if (modsLoading.value || modsRefreshing.value) return;
  modsRefreshing.value = true;
  try {
    await loadMods();
  } finally {
    modsRefreshing.value = false;
  }
}

const toggleMod = useToggleItem({ items: mods, error: modToggleError, invokeCommand: "set_mod_enabled" });

function modInitial(name: string): string {
  return name.trim().charAt(0).toUpperCase() || "?";
}

function packIconSrc(item: PackItem): string | undefined {
  return item.iconUrl || item.iconData || undefined;
}

function modDisplayFileName(mod: ModInfo): string {
  return mod.enabled ? mod.fileName : `${mod.fileName}.disabled`;
}

const showModInFolder = useShowInFolder({ 
  error: modToggleError, 
  invokeCommand: "show_mod_in_folder" 
});

const shaders = shallowRef<PackItem[]>([]);
const shadersLoading = ref(false);
const shadersRefreshing = ref(false);
const shadersError = ref("");
const shaderSearch = ref("");
const debouncedShaderSearch = useDebouncedValue(shaderSearch);
const showDisabledShadersOnly = ref(false);
let loadShadersInFlight = false;
let shadersLoadedOnce = false;

const clearShaderSearch = useClearSearch(shaderSearch, debouncedShaderSearch);

function toggleShadersDisabledOnlyFilter() {
  showDisabledShadersOnly.value = !showDisabledShadersOnly.value;
}

const disabledShadersCount = countDisabled(shaders);

const filteredShaders = useFilteredList(shaders, debouncedShaderSearch, showDisabledShadersOnly);

async function loadShaders() {
  if (loadShadersInFlight) return;
  loadShadersInFlight = true;
  shadersLoading.value = true;
  shadersError.value = "";
  try {
    shaders.value = await invoke<PackItem[]>("list_content_items", { kind: "shaderpacks" });
    shadersLoadedOnce = true;
  } catch (e: any) {
    shadersError.value = e?.message || "Could not load the shaders list.";
  } finally {
    shadersLoading.value = false;
    loadShadersInFlight = false;
  }
}

async function refreshShaders() {
  if (shadersLoading.value || shadersRefreshing.value) return;
  shadersRefreshing.value = true;
  try {
    await loadShaders();
  } finally {
    shadersRefreshing.value = false;
  }
}

const toggleShader = useToggleItem({ 
  items: shaders, 
  error: shadersError, 
  invokeCommand: "set_content_item_enabled",
  invokeParams: { kind: "shaderpacks" }
});

const showShaderInFolder = useShowInFolder({ 
  error: shadersError, 
  invokeCommand: "show_content_item_in_folder",
  invokeParams: { kind: "shaderpacks" }
});

const resourcePacks = shallowRef<PackItem[]>([]);
const resourcePacksLoading = ref(false);
const resourcePacksRefreshing = ref(false);
const resourcePacksError = ref("");
const resourcePackSearch = ref("");
const debouncedResourcePackSearch = useDebouncedValue(resourcePackSearch);
const showDisabledResourcePacksOnly = ref(false);
let loadResourcePacksInFlight = false;
let resourcePacksLoadedOnce = false;

const clearResourcePackSearch = useClearSearch(resourcePackSearch, debouncedResourcePackSearch);

function toggleResourcePacksDisabledOnlyFilter() {
  showDisabledResourcePacksOnly.value = !showDisabledResourcePacksOnly.value;
}

const disabledResourcePacksCount = countDisabled(resourcePacks);

const filteredResourcePacks = useFilteredList(resourcePacks, debouncedResourcePackSearch, showDisabledResourcePacksOnly);

const MOD_ROW_STEP = 65;
const VIRTUAL_OVERSCAN = 1;

function useVirtualList<T>(getItems: () => T[]) {
  const scrollerRef = ref<HTMLElement | null>(null);
  const scrollTop = ref(0);
  const viewportHeight = ref(0);
  let resizeObserver: ResizeObserver | null = null;
  let scrollSettleTimer: number | null = null;
  let ticking = false;

  // OPTIMIZACIÓN: Cache para evitar recálculos redundantes de visibleEntries.
  // IMPORTANTE: estas variables deben vivir DENTRO de useVirtualList, no a nivel
  // de módulo, porque la función se instancia varias veces (mods, shaders,
  // resourcepacks). Si viven afuera, las tres listas comparten la misma caché
  // y una lista puede terminar devolviendo las filas visibles calculadas para
  // otra lista (ej: la modlist se ve corta/vacía tras cambiar de pestaña, o el
  // contador de shaderpacks no coincide con las filas renderizadas).
  let lastScrollTop = 0;
  let lastViewportHeight = 0;
  let lastItemsLength = 0;
  let lastStartIndex = 0;
  let lastVisibleCount = 0;
  let visibleEntriesCache: { item: T; index: number; top: number; slot: number }[] = [];
  
  // OPTIMIZACIÓN: Contador de frames para throttling
  let frameCounter = 0;
  const FRAME_SYNC_INTERVAL = 3;
  
  // OPTIMIZACIÓN: Último scrollTop procesado para evitar actualizaciones redundantes
  let lastProcessedScrollTop = 0;

  function onScroll(e: Event) {
    const target = e.target as HTMLElement;
    const currentScrollTop = target.scrollTop;

    // OPTIMIZACIÓN: Saltar si el scroll no ha cambiado lo suficiente
    if (Math.abs(currentScrollTop - lastProcessedScrollTop) < 0.5) {
      return;
    }
    lastProcessedScrollTop = currentScrollTop;

    // OPTIMIZACIÓN CRÍTICA: Bloquear pointer-events directamente por DOM
    // Esto previene que el navegador evalúe hover/tooltip durante scroll
    target.classList.add("is-scrolling");
    target.style.pointerEvents = "none";

    if (!ticking) {
      window.requestAnimationFrame(() => {
        frameCounter++;
        
        // OPTIMIZACIÓN: Solo sincronizar scrollTop cada FRAME_SYNC_INTERVAL frames
        if (frameCounter >= FRAME_SYNC_INTERVAL) {
          scrollTop.value = currentScrollTop;
          frameCounter = 0;
        }
        
        ticking = false;
        target.style.pointerEvents = "";
      });
      ticking = true;
    }

    if (scrollSettleTimer) window.clearTimeout(scrollSettleTimer);
    scrollSettleTimer = window.setTimeout(() => {
      target.classList.remove("is-scrolling");
      scrollTop.value = target.scrollTop;
      frameCounter = 0;
    }, 150);
  }

  function measure() {
    if (scrollerRef.value) viewportHeight.value = scrollerRef.value.clientHeight;
  }

  // IMPORTANTE: usar watch(scrollerRef) en vez de solo onMounted().
  // El elemento del scroller puede destruirse y recrearse (v-if de
  // loading/error/vacío/lista, cambios de pestaña, etc.), y con solo
  // onMounted el ResizeObserver quedaba observando el nodo DOM viejo,
  // que ya no existía. Al maximizar/restaurar la ventana después de eso,
  // viewportHeight nunca se actualizaba y la lista se veía cortada o vacía
  // hasta que un scroll forzaba un recálculo. Con watch + immediate nos
  // reenganchamos al nodo correcto cada vez que cambia.
  watch(
    scrollerRef,
    (el) => {
      resizeObserver?.disconnect();
      resizeObserver = null;
      if (el && typeof ResizeObserver !== "undefined") {
        resizeObserver = new ResizeObserver(measure);
        resizeObserver.observe(el);
      }

      // IMPORTANTE: el contenedor del scroller se destruye y se vuelve a
      // crear cada vez que se cambia de pestaña (v-if/v-else-if en el
      // template), así que el nodo DOM nuevo siempre arranca con
      // scrollTop = 0. Si no resincronizamos aquí, el scrollTop.value
      // interno se queda con el valor viejo (ej. si habías scrolleado
      // hacia abajo antes de cambiar de pestaña), y la virtualización
      // calcula las filas visibles según esa posición vieja mientras el
      // contenedor real está arriba del todo: el resultado es que las
      // primeras filas no se pintan hasta que el usuario vuelve a
      // scrollear y se dispara un recálculo con el valor correcto.
      if (el) el.scrollTop = 0;
      scrollTop.value = 0;
      lastProcessedScrollTop = 0;
      frameCounter = 0;
      if (scrollSettleTimer) {
        window.clearTimeout(scrollSettleTimer);
        scrollSettleTimer = null;
      }

      measure();
    },
    { immediate: true }
  );

  onUnmounted(() => {
    resizeObserver?.disconnect();
    if (scrollSettleTimer) window.clearTimeout(scrollSettleTimer);
  });

  const totalHeight = computed(() => getItems().length * MOD_ROW_STEP);

  const startIndex = computed(() =>
    Math.max(0, Math.floor(scrollTop.value / MOD_ROW_STEP) - VIRTUAL_OVERSCAN)
  );

  // Cantidad maxima de filas que se pueden llegar a renderizar a la vez
  // (viewport + overscan a ambos lados). Se usa tambien para calcular un
  // "slot" estable por fila (ver visibleEntries) que permite reciclar nodos
  // DOM en vez de destruir/crear uno nuevo cada vez que una fila entra o
  // sale de la ventana virtualizada.
  const visibleCount = computed(() =>
    Math.ceil(viewportHeight.value / MOD_ROW_STEP) + VIRTUAL_OVERSCAN * 2
  );

  const visibleEntries = computed(() => {
    const list = getItems();
    const currentScrollTop = scrollTop.value;
    const currentViewportHeight = viewportHeight.value;
    const currentItemsLength = list.length;
    const currentStartIndex = startIndex.value;
    const currentVisibleCount = visibleCount.value;
    
    // OPTIMIZACIÓN: Cachear resultado si ningún parámetro ha cambiado
    // Esto evita recálculos redundantes durante scroll continuo
    if (currentScrollTop === lastScrollTop && 
        currentViewportHeight === lastViewportHeight && 
        currentItemsLength === lastItemsLength &&
        currentStartIndex === lastStartIndex &&
        currentVisibleCount === lastVisibleCount) {
      return visibleEntriesCache;
    }
    
    lastScrollTop = currentScrollTop;
    lastViewportHeight = currentViewportHeight;
    lastItemsLength = currentItemsLength;
    lastStartIndex = currentStartIndex;
    lastVisibleCount = currentVisibleCount;
    
    const slots = Math.max(1, currentVisibleCount);
    const endIdx = Math.min(currentItemsLength, currentStartIndex + currentVisibleCount);
    const out: { item: T; index: number; top: number; slot: number }[] = [];
    
    // OPTIMIZACIÓN: Pre-allocar el array para evitar reallocaciones
    out.length = endIdx - currentStartIndex;
    
    // Bucle optimizado con acceso directo por índice
    for (let i = currentStartIndex; i < endIdx; i++) {
      out[i - currentStartIndex] = {
        item: list[i],
        index: i,
        top: i * MOD_ROW_STEP,
        slot: i % slots
      };
    }
    
    visibleEntriesCache = out;
    return out;
  });

  return { scrollerRef, onScroll, totalHeight, visibleEntries, measure };
}

const modsVirtual = useVirtualList(() => filteredMods.value);
const shadersVirtual = useVirtualList(() => filteredShaders.value);
const resourcePacksVirtual = useVirtualList(() => filteredResourcePacks.value);


function setModsScrollerRef(el: Element | ComponentPublicInstance | null) {
  modsVirtual.scrollerRef.value = el as HTMLElement | null;
}
function setShadersScrollerRef(el: Element | ComponentPublicInstance | null) {
  shadersVirtual.scrollerRef.value = el as HTMLElement | null;
}
function setResourcePacksScrollerRef(el: Element | ComponentPublicInstance | null) {
  resourcePacksVirtual.scrollerRef.value = el as HTMLElement | null;
}

async function loadResourcePacks() {
  if (loadResourcePacksInFlight) return;
  loadResourcePacksInFlight = true;
  resourcePacksLoading.value = true;
  resourcePacksError.value = "";
  try {
    resourcePacks.value = await invoke<PackItem[]>("list_content_items", { kind: "resourcepacks" });
    resourcePacksLoadedOnce = true;
  } catch (e: any) {
    resourcePacksError.value = e?.message || "Could not load the resource packs list.";
  } finally {
    resourcePacksLoading.value = false;
    loadResourcePacksInFlight = false;
  }
}

async function refreshResourcePacks() {
  if (resourcePacksLoading.value || resourcePacksRefreshing.value) return;
  resourcePacksRefreshing.value = true;
  try {
    await loadResourcePacks();
  } finally {
    resourcePacksRefreshing.value = false;
  }
}

const toggleResourcePack = useToggleItem({ 
  items: resourcePacks, 
  error: resourcePacksError, 
  invokeCommand: "set_content_item_enabled",
  invokeParams: { kind: "resourcepacks" }
});

const showResourcePackInFolder = useShowInFolder({ 
  error: resourcePacksError, 
  invokeCommand: "show_content_item_in_folder",
  invokeParams: { kind: "resourcepacks" }
});

watch(contentTab, (tab) => {
  if (tab === "shaders" && !shadersLoadedOnce) loadShaders();
  if (tab === "resourcepacks" && !resourcePacksLoadedOnce) loadResourcePacks();
});

const instanceFolderError = ref("");

async function showInstanceInFolder() {
  instanceFolderError.value = "";
  try {
    await invoke("show_instance_in_folder");
  } catch (e: any) {
    instanceFolderError.value = e?.message || "Could not open the instance folder.";
  }
}

const modpackCheckInterval = ref<number | null>(null);

interface InstanceStats {
  playCount: number;
  lastPlayed: string | null;
  totalTimePlayedSecs: number;
}

const instanceStats = ref<InstanceStats | null>(null);
const playtimeTickInterval = ref<number | null>(null);

async function loadInstanceStats() {
  try {
    instanceStats.value = await invoke<InstanceStats>("get_instance_stats");
  } catch (e) {
    console.error("Failed to load instance stats:", e);
  }
}

const playtimeDisplay = computed(() => {
  const secs = instanceStats.value?.totalTimePlayedSecs ?? 0;
  return formatTimeSeconds(secs);
});

onMounted(async () => {
  loadMods();
  loadShaders();
  loadResourcePacks();
  await launcher.loadSettings();
  await launcher.checkInstallation();
  await loadJavaInfo();
  await launcher.loadModpackProfile();
  loadInstanceStats();

  if (launcher.installed) {
    checkModpackUpdate();
  }

  statusPollInterval.value = window.setInterval(async () => {
    await launcher.checkInstallation();
  }, 5000);

  modpackCheckInterval.value = window.setInterval(() => {
    if (launcher.installed && !modpackSyncing.value) {
      checkModpackUpdate();
    }
  }, 5 * 60 * 1000);
});

onUnmounted(() => {
  if (statusPollInterval.value) {
    clearInterval(statusPollInterval.value);
  }
  if (modpackCheckInterval.value) {
    clearInterval(modpackCheckInterval.value);
  }
  if (playtimeTickInterval.value) {
    clearInterval(playtimeTickInterval.value);
  }
  disposeToastTimer();
});

const isInstalled = computed(() => launcher.installed);
const isInstalling = computed(() => launcher.installing);
const progress = computed(() => launcher.installProgress);
const isLaunching = computed(() => launcher.isLaunching);

const playButtonText = computed(() => {
  if (props.gameRunning) return "Stop";
  if (isLaunching.value) return "Launching...";
  if (isInstalling.value) return "Installing...";
  if (!isInstalled.value) return "Download";
  if (modpackSyncing.value) return "Downloading Update...";
  if (modpackUpdateAvailable.value) return "Download Update";
  return "Play";
});

const hasJava = computed(() => launcher.javaPath && launcher.javaPath.length > 0);

interface ModpackInfo {
  version: string;
  publishedAt: string;
  size: number;
  upToDate: boolean;
}

const modpackInfo = ref<ModpackInfo | null>(null);
const modpackChecking = ref(false);
const modpackSyncing = ref(false);
const modpackError = ref("");

const modpackUpdateAvailable = computed(() => !!modpackInfo.value && !modpackInfo.value.upToDate);

const modpackSizeDisplay = computed(() => {
  if (!modpackInfo.value) return "";
  const mb = modpackInfo.value.size / (1024 * 1024);
  return mb >= 1024 ? `${(mb / 1024).toFixed(2)} GB` : `${mb.toFixed(0)} MB`;
});

async function checkModpackUpdate() {
  if (modpackChecking.value) return;
  modpackChecking.value = true;
  try {
    modpackInfo.value = await invoke<ModpackInfo>("check_modpack_update");
  } catch (e: any) {
    console.error("Could not check the modpack:", e);
  } finally {
    modpackChecking.value = false;
  }
}

async function syncModpack() {
  if (modpackSyncing.value) return;
  modpackSyncing.value = true;
  modpackError.value = "";

  const channel = new Channel<InstallProgress>();
  channel.onmessage = (msg) => {
    launcher.installProgress = msg;
    launcher.launchProgress = msg.percent;
    launcher.launchStatus = msg.detail;
  };

  try {
    const version = await invoke<string>("sync_modpack", { progressChannel: channel });
    launcher.launchStatus = `Modpack updated to ${version}.`;
    await checkModpackUpdate();
    await loadMods();
  } catch (e: any) {
    modpackError.value = e?.message || e?.toString() || "Could not update the modpack.";
    launcher.launchStatus = "Couldn't update the modpack: " + modpackError.value;
  } finally {
    modpackSyncing.value = false;
    scheduleToastDismiss();
  }
}

async function handleSetModpackProfile(profile: "default" | "no_dh") {
  await launcher.setModpackProfile(profile);
  if (isInstalled.value) {
    await loadMods();
  }
}

async function installRecommendedJava() {
  if (installingJava.value) return;
  installingJava.value = true;
  javaInstallError.value = "";
  launcher.launchProgress = 0;
  launcher.launchStatus = "Installing Java...";

  const channel = new Channel<InstallProgress>();
  channel.onmessage = (msg) => {
    launcher.installProgress = msg;
    launcher.launchProgress = msg.percent;
    launcher.launchStatus = msg.detail;
  };

  try {
    const path = await invoke<string>("install_recommended_java", { progressChannel: channel });
    launcher.javaPath = path;
    await invoke("save_settings", {
      settings: {
        ram: launcher.ram,
        java_path: launcher.javaPath,
        game_dir: launcher.gameDirectory,
        resolution: "854 x 480",
        fullscreen: false,
        custom_titlebar: launcher.customTitlebar,
        theme: "system",
        accent_color: themeStore.accentColor,
        custom_presets: [],
        java_args: launcher.javaArgs,
      }
    });
    await loadJavaInfo();
    launcher.launchProgress = 100;
    launcher.launchStatus = "Java installed and ready to go.";
  } catch (e: any) {
    javaInstallError.value = e?.message || "Failed to install Java";
    launcher.launchStatus = "Couldn't install Java: " + javaInstallError.value;
  } finally {
    installingJava.value = false;
    scheduleToastDismiss();
  }
}

async function handleInstall() {
  if (isInstalling.value) return;
  if (!hasJava.value) {
    javaInstallError.value = "No Java selected yet — install the recommended version to continue.";
    return;
  }

  launcher.installing = true;
  launcher.launchProgress = 0;
  launcher.launchStatus = "Installing the modpack...";

  const channel = new Channel<InstallProgress>();
  channel.onmessage = (msg) => {
    launcher.installProgress = msg;
    launcher.launchProgress = msg.percent;
    launcher.launchStatus = msg.detail;
  };

  try {
    await invoke("install_game", {
      progressChannel: channel,
      ram: launcher.ram,
      javaArgs: launcher.javaArgs,
      javaPath: launcher.javaPath,
    });
    
    launcher.installed = await invoke<boolean>("check_installation");
    
    if (launcher.installed) {
      await syncModpack();
      launcher.launchStatus = "All set — ready to play.";
      await loadMods();
    } else {
      launcher.launchStatus = "Installed, but something looks off.";
    }
    launcher.launchProgress = 100;
  } catch (e: any) {
    const errMsg = e?.message || e?.toString() || "Unknown error";
    launcher.launchStatus = "Couldn't finish installing: " + errMsg;
    console.error("Install error:", e);
  } finally {
    launcher.installing = false;
    scheduleToastDismiss();
  }
}

async function handlePlay() {
  if (isLaunching.value || isInstalling.value || modpackSyncing.value) return;
  if (!hasJava.value) {
    javaInstallError.value = "No Java selected yet — install the recommended version to continue.";
    return;
  }
  if (!isInstalled.value) {
    return;
  }
  if (modpackUpdateAvailable.value) {
    return;
  }

  launcher.isLaunching = true;
  launcher.launchProgress = 0;
  emit("update:gameRunning", false);

  const channel = new Channel<InstallProgress>();
  channel.onmessage = (msg) => {
    launcher.installProgress = msg;
    launcher.launchProgress = msg.percent;
    if (msg.stage === "launched") {
      emit("update:gameRunning", true);
      if (!playtimeTickInterval.value) {
        playtimeTickInterval.value = window.setInterval(loadInstanceStats, 60000);
      }
    }
  };

  try {
    await invoke("play", {
      progressChannel: channel,
      ram: launcher.ram,
      javaArgs: launcher.javaArgs,
      javaPath: launcher.javaPath,
      account: auth.account,
    });
  } catch (e: any) {
    const errMsg = e?.message || e?.toString() || "Unknown error";
    launcher.launchStatus = "Couldn't launch the game: " + errMsg;
    console.error("Play error:", e);
  } finally {
    launcher.isLaunching = false;
    emit("update:gameRunning", false);
    if (playtimeTickInterval.value) {
      clearInterval(playtimeTickInterval.value);
      playtimeTickInterval.value = null;
    }
    loadInstanceStats();
    scheduleToastDismiss();
  }
}

async function handleStop() {
  try {
    await invoke("stop_game");
  } catch (e: any) {
    launcher.launchStatus = "Couldn't stop the game" + (e?.message ? `: ${e.message}` : ".");
    scheduleToastDismiss();
  }
}

async function handleMainButton() {
  if (props.gameRunning) {
    await handleStop();
  } else if (!isInstalled.value) {
    await handleInstall();
  } else if (modpackUpdateAvailable.value) {
    await syncModpack();
  } else {
    await handlePlay();
  }
}
</script>

<template>
    <main v-show="active" class="play-view">
      <div class="hero-card">
        <div class="hero-left">
          <div class="hero-icon">
            <img src="/logo.png" alt="Logo">
          </div>
          <div class="hero-text">
            <div class="hero-title-row">
              <h1 class="hero-title">Beyond Promised Sparks</h1>
              <div class="hero-folder-wrap">
                <button
                  type="button"
                  class="hero-folder-btn"
                  v-tooltip="'Open instance folder'"
                  aria-label="Open instance folder"
                  @click="showInstanceInFolder"
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                  </svg>
                </button>
                <transition name="dropdown">
                  <span v-if="instanceFolderError" class="hero-folder-error">{{ instanceFolderError }}</span>
                </transition>
              </div>
            </div>
            <p class="hero-subtitle">Minecraft {{ MINECRAFT_VERSION }} · Forge {{ FORGE_VERSION }}</p>
          </div>
        </div>

        <div class="hero-right">
          <div class="hero-stat" :class="{ 'stat-alert': !hasJava }">
            <div class="stat-glow"></div>
            <div class="stat-top">
              <div class="stat-icon">
                <Icon name="java" :size="24" />
              </div>
              <div class="stat-info">
                <span class="stat-label">Java Runtime</span>
                <span class="stat-value" :class="{ bad: !hasJava }">
                  {{ hasJava ? (javaVersionInfo?.major ? `Java ${javaVersionInfo.major}` : 'Installed') : 'Not installed' }}
                </span>
              </div>
            </div>
            <AppButton
              full-width
              @click="hasJava ? emit('openSettings', 'java') : installRecommendedJava()"
            >
              {{ hasJava ? 'Change' : 'Install Java' }}
            </AppButton>
          </div>

          <div class="hero-stat">
            <div class="stat-glow ram"></div>
            <div class="stat-top">
              <div class="stat-icon ram">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M6 19v-3" />
                  <path d="M10 19v-3" />
                  <path d="M14 19v-3" />
                  <path d="M18 19v-3" />
                  <path d="M8 11V9" />
                  <path d="M16 11V9" />
                  <path d="M12 11V9" />
                  <path d="M2 15h20" />
                  <path d="M2 7a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v1.1a2 2 0 0 0 0 3.837V17a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2v-5.1a2 2 0 0 0 0-3.837Z" />
                </svg>
              </div>
              <div class="stat-info">
                <span class="stat-label">Memory</span>
                <span class="stat-value">{{ ramDisplay }}</span>
              </div>
            </div>
            <AppButton full-width @click="emit('openSettings', 'java')">Change</AppButton>
          </div>

          <div class="hero-stat hero-stat-profile">
            <div class="stat-glow profile"></div>
            <div class="stat-top">
              <div class="stat-icon profile">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="4" y1="21" x2="4" y2="14" />
                  <line x1="4" y1="10" x2="4" y2="3" />
                  <line x1="12" y1="21" x2="12" y2="12" />
                  <line x1="12" y1="8" x2="12" y2="3" />
                  <line x1="20" y1="21" x2="20" y2="16" />
                  <line x1="20" y1="12" x2="20" y2="3" />
                  <line x1="1" y1="14" x2="7" y2="14" />
                  <line x1="9" y1="8" x2="15" y2="8" />
                  <line x1="17" y1="16" x2="23" y2="16" />
                </svg>
              </div>
              <div class="stat-info">
                <span class="stat-label">Profile</span>
                <span class="stat-value profile-value">{{ launcher.modpackProfile === "no_dh" ? "No DH" : "Default" }}</span>
              </div>
            </div>
            <div class="profile-options" role="radiogroup" aria-label="Modpack profile">
              <button
                type="button"
                class="profile-option"
                role="radio"
                v-tooltip="'Includes Distant Horizons'"
                :aria-checked="launcher.modpackProfile === 'default'"
                :class="{ active: launcher.modpackProfile === 'default' }"
                :disabled="launcher.modpackProfileSaving"
                @click="handleSetModpackProfile('default')"
              >
                <span class="profile-option-dot" aria-hidden="true"></span>
                Default
              </button>
              <button
                type="button"
                class="profile-option"
                role="radio"
                v-tooltip="'Keeps Distant Horizons off'"
                :aria-checked="launcher.modpackProfile === 'no_dh'"
                :class="{ active: launcher.modpackProfile === 'no_dh' }"
                :disabled="launcher.modpackProfileSaving"
                @click="handleSetModpackProfile('no_dh')"
              >
                <span class="profile-option-dot" aria-hidden="true"></span>
                No DH
              </button>
            </div>
          </div>
        </div>
      </div>

      <transition name="dropdown">
        <p v-if="launcher.modpackProfileError" class="modpack-profile-error">{{ launcher.modpackProfileError }}</p>
      </transition>

      <div class="content-card">
        <div class="content-toolbar">
          <div class="content-tabs">
            <button class="ct-tab" :class="{ active: contentTab === 'mods' }" @click="contentTab = 'mods'">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
                <path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z" />
                <polyline points="3.27 6.96 12 12.01 20.73 6.96" />
                <line x1="12" y1="22.08" x2="12" y2="12" />
              </svg>
              Mods
              <span v-if="mods.length > 0" class="ct-tab-count">{{ mods.length }}</span>
            </button>
            <button class="ct-tab" :class="{ active: contentTab === 'shaders' }" @click="contentTab = 'shaders'">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
                <path d="M12 3l1.9 5.8a2 2 0 0 0 1.3 1.3L21 12l-5.8 1.9a2 2 0 0 0-1.3 1.3L12 21l-1.9-5.8a2 2 0 0 0-1.3-1.3L3 12l5.8-1.9a2 2 0 0 0 1.3-1.3L12 3z" />
              </svg>
              Shaders
              <span v-if="shaders.length > 0" class="ct-tab-count">{{ shaders.length }}</span>
            </button>
            <button class="ct-tab" :class="{ active: contentTab === 'resourcepacks' }" @click="contentTab = 'resourcepacks'">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="14" height="14">
                <polygon points="12 2 2 7 12 12 22 7 12 2" />
                <polyline points="2 17 12 22 22 17" />
                <polyline points="2 12 12 17 22 12" />
              </svg>
              Resource Packs
              <span v-if="resourcePacks.length > 0" class="ct-tab-count">{{ resourcePacks.length }}</span>
            </button>
          </div>
          <div v-if="contentTab === 'mods'" class="content-actions">
            <SearchBox v-model="modSearch" placeholder="Search mods..." />
            <button
              type="button"
              class="mods-filter-chip"
              :class="{ active: showDisabledOnly }"
              v-tooltip="showDisabledOnly ? 'Show all mods' : 'Show only disabled mods'"
              @click="toggleDisabledOnlyFilter"
            >
              <Icon name="no-disabled" :size="13" />
              Mods disabled
              <span v-if="disabledModsCount > 0" class="mods-filter-count">{{ disabledModsCount }}</span>
            </button>
            <button
              type="button"
              class="mods-refresh-btn"
              :disabled="modsLoading || modsRefreshing"
              v-tooltip="'Refresh mod list'"
              @click="refreshMods"
            >
              <svg
                :class="{ spin: modsLoading || modsRefreshing }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                width="14"
                height="14"
              >
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
            </button>
          </div>
          <div v-else-if="contentTab === 'shaders'" class="content-actions">
            <SearchBox v-model="shaderSearch" placeholder="Search shaders..." />
            <button
              type="button"
              class="mods-filter-chip"
              :class="{ active: showDisabledShadersOnly }"
              v-tooltip="showDisabledShadersOnly ? 'Show all shaders' : 'Show only disabled shaders'"
              @click="toggleShadersDisabledOnlyFilter"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
                <circle cx="12" cy="12" r="10" />
                <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
              </svg>
              Shaders disabled
              <span v-if="disabledShadersCount > 0" class="mods-filter-count">{{ disabledShadersCount }}</span>
            </button>
            <button
              type="button"
              class="mods-refresh-btn"
              :disabled="shadersLoading || shadersRefreshing"
              v-tooltip="'Refresh shader list'"
              @click="refreshShaders"
            >
              <svg
                :class="{ spin: shadersLoading || shadersRefreshing }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                width="14"
                height="14"
              >
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
            </button>
          </div>
          <div v-else class="content-actions">
            <SearchBox v-model="resourcePackSearch" placeholder="Search resource packs..." />
            <button
              type="button"
              class="mods-filter-chip"
              :class="{ active: showDisabledResourcePacksOnly }"
              v-tooltip="showDisabledResourcePacksOnly ? 'Show all resource packs' : 'Show only disabled resource packs'"
              @click="toggleResourcePacksDisabledOnlyFilter"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
                <circle cx="12" cy="12" r="10" />
                <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
              </svg>
              Packs disabled
              <span v-if="disabledResourcePacksCount > 0" class="mods-filter-count">{{ disabledResourcePacksCount }}</span>
            </button>
            <button
              type="button"
              class="mods-refresh-btn"
              :disabled="resourcePacksLoading || resourcePacksRefreshing"
              v-tooltip="'Refresh resource pack list'"
              @click="refreshResourcePacks"
            >
              <svg
                :class="{ spin: resourcePacksLoading || resourcePacksRefreshing }"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                width="14"
                height="14"
              >
                <polyline points="23 4 23 10 17 10" />
                <polyline points="1 20 1 14 7 14" />
                <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
              </svg>
            </button>
          </div>
        </div>

        <div v-if="contentTab === 'mods' && (modsError || modToggleError)" class="mods-error-banner">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span>{{ modsError || modToggleError }}</span>
        </div>
        <div v-else-if="contentTab === 'shaders' && shadersError" class="mods-error-banner">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span>{{ shadersError }}</span>
        </div>
        <div v-else-if="contentTab === 'resourcepacks' && resourcePacksError" class="mods-error-banner">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="13" height="13">
            <circle cx="12" cy="12" r="10" />
            <line x1="12" y1="8" x2="12" y2="12" />
            <line x1="12" y1="16" x2="12.01" y2="16" />
          </svg>
          <span>{{ resourcePacksError }}</span>
        </div>

        <div v-if="contentTab === 'mods'" class="mods-scroller" :ref="setModsScrollerRef" @scroll.passive="modsVirtual.onScroll">
          <EmptyState v-if="modsLoading && mods.length === 0" icon="spinner" message="Loading mods..." />
          <EmptyState
            v-else-if="filteredMods.length === 0 && debouncedModSearch"
            icon="search"
            :message="'No mods match &quot;' + modSearch + '&quot;'"
            action-label="Clear search"
            @action="clearModSearch"
          />
          <EmptyState
            v-else-if="filteredMods.length === 0 && showDisabledOnly"
            icon="no-disabled"
            message="No disabled mods."
            action-label="Show all mods"
            @action="toggleDisabledOnlyFilter"
          />
          <EmptyState
            v-else-if="filteredMods.length === 0"
            icon="grid"
            :message="isInstalled ? 'No mods found.' : 'Install to see mods here.'"
          />
          <div v-else class="mod-list mod-list-virtual" :style="{ height: modsVirtual.totalHeight.value + 'px' }">
            <div
              v-for="entry in modsVirtual.visibleEntries.value"
              :key="entry.slot"
              class="mod-item"
              :class="{ disabled: !entry.item.enabled }"
              :style="{ transform: `translateY(${entry.top}px)` }"
            >
              <div class="mod-left">
                <img
                  v-if="entry.item.iconUrl"
                  :src="entry.item.iconUrl"
                  class="mod-thumb"
                  :alt="entry.item.name"
                  loading="lazy"
                  decoding="async"
                  fetchpriority="low"
                />
                <div v-else class="mod-thumb mod-thumb-fallback" v-once>{{ modInitial(entry.item.name) }}</div>
                
                <div class="mod-text">
                  <div class="mod-name-row">
                    <span class="mod-name" v-tooltip="entry.item.name">{{ entry.item.name }}</span>
                  </div>
                  <div class="mod-meta-row">
                    <span class="mod-file" v-tooltip="modDisplayFileName(entry.item)" v-once>{{ modDisplayFileName(entry.item) }}</span>
                  </div>
                </div>
              </div>
              
              <div class="mod-right">
                <span class="mod-ver-badge" v-once>{{ entry.item.version }}</span>
                <ToggleSwitch
                  :model-value="entry.item.enabled"
                  v-tooltip="entry.item.enabled ? 'Click to disable' : 'Click to enable'"
                  @update:model-value="toggleMod(entry.item)"
                />
                <button
                  type="button"
                  class="mod-show-file-btn"
                  v-tooltip="'Show file in folder'"
                  aria-label="Show file in folder"
                  @click="showModInFolder(entry.item)"
                >
                  <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-else-if="contentTab === 'shaders'" class="mods-scroller" :ref="setShadersScrollerRef" @scroll.passive="shadersVirtual.onScroll">
          <EmptyState v-if="shadersLoading && shaders.length === 0" icon="spinner" message="Loading shaders..." />
          <EmptyState
            v-else-if="filteredShaders.length === 0 && debouncedShaderSearch"
            icon="search"
            :message="'No shaders match &quot;' + shaderSearch + '&quot;'"
            action-label="Clear search"
            @action="clearShaderSearch"
          />
          <EmptyState
            v-else-if="filteredShaders.length === 0 && showDisabledShadersOnly"
            icon="no-disabled"
            message="No disabled shaders."
            action-label="Show all shaders"
            @action="toggleShadersDisabledOnlyFilter"
          />
          <EmptyState
            v-else-if="filteredShaders.length === 0"
            icon="grid"
            :message="isInstalled ? 'No shaders found.' : 'Install to see shaders here.'"
          />
          <div v-else class="mod-list mod-list-virtual" :style="{ height: shadersVirtual.totalHeight.value + 'px' }">
            <div
              v-for="entry in shadersVirtual.visibleEntries.value"
              :key="entry.slot"
              class="mod-item"
              :class="{ disabled: !entry.item.enabled }"
              :style="{ transform: `translateY(${entry.top}px)` }"
            >
              <div class="mod-left">
                <img
                  v-if="packIconSrc(entry.item)"
                  :src="packIconSrc(entry.item)"
                  class="mod-thumb"
                  :alt="entry.item.name"
                  loading="lazy"
                  decoding="async"
                  fetchpriority="low"
                />
                <div v-else class="mod-thumb mod-thumb-fallback" v-once>{{ modInitial(entry.item.name) }}</div>
                <div class="mod-text">
                  <div class="mod-name-row">
                    <span class="mod-name" v-tooltip="entry.item.name">{{ entry.item.name }}</span>
                  </div>
                  <div class="mod-meta-row">
                    <span class="mod-file" v-tooltip="entry.item.fileName" v-once>{{ entry.item.fileName }}</span>
                  </div>
                </div>
              </div>

              <div class="mod-right">
                <ToggleSwitch
                  :model-value="entry.item.enabled"
                  v-tooltip="entry.item.enabled ? 'Click to disable' : 'Click to enable'"
                  @update:model-value="toggleShader(entry.item)"
                />
                <button
                  type="button"
                  class="mod-show-file-btn"
                  v-tooltip="'Show file in folder'"
                  aria-label="Show file in folder"
                  @click="showShaderInFolder(entry.item)"
                >
                  <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>

        <div v-else class="mods-scroller" :ref="setResourcePacksScrollerRef" @scroll.passive="resourcePacksVirtual.onScroll">
          <EmptyState v-if="resourcePacksLoading && resourcePacks.length === 0" icon="spinner" message="Loading resource packs..." />
          <EmptyState
            v-else-if="filteredResourcePacks.length === 0 && debouncedResourcePackSearch"
            icon="search"
            :message="'No resource packs match &quot;' + resourcePackSearch + '&quot;'"
            action-label="Clear search"
            @action="clearResourcePackSearch"
          />
          <EmptyState
            v-else-if="filteredResourcePacks.length === 0 && showDisabledResourcePacksOnly"
            icon="no-disabled"
            message="No disabled resource packs."
            action-label="Show all resource packs"
            @action="toggleResourcePacksDisabledOnlyFilter"
          />
          <EmptyState
            v-else-if="filteredResourcePacks.length === 0"
            icon="grid"
            :message="isInstalled ? 'No resource packs found.' : 'Install to see resource packs here.'"
          />
          <div v-else class="mod-list mod-list-virtual" :style="{ height: resourcePacksVirtual.totalHeight.value + 'px' }">
            <div
              v-for="entry in resourcePacksVirtual.visibleEntries.value"
              :key="entry.slot"
              class="mod-item"
              :class="{ disabled: !entry.item.enabled }"
              :style="{ transform: `translateY(${entry.top}px)` }"
            >
              <div class="mod-left">
                <img
                  v-if="packIconSrc(entry.item)"
                  :src="packIconSrc(entry.item)"
                  class="mod-thumb"
                  :alt="entry.item.name"
                  loading="lazy"
                  decoding="async"
                  fetchpriority="low"
                />
                <div v-else class="mod-thumb mod-thumb-fallback" v-once>{{ modInitial(entry.item.name) }}</div>
                <div class="mod-text">
                  <div class="mod-name-row">
                    <span class="mod-name" v-tooltip="entry.item.name">{{ entry.item.name }}</span>
                  </div>
                  <div class="mod-meta-row">
                    <span class="mod-file" v-tooltip="entry.item.fileName" v-once>{{ entry.item.fileName }}</span>
                  </div>
                </div>
              </div>

              <div class="mod-right">
                <ToggleSwitch
                  :model-value="entry.item.enabled"
                  v-tooltip="entry.item.enabled ? 'Click to disable' : 'Click to enable'"
                  @update:model-value="toggleResourcePack(entry.item)"
                />
                <button
                  type="button"
                  class="mod-show-file-btn"
                  v-tooltip="'Show file in folder'"
                  aria-label="Show file in folder"
                  @click="showResourcePackInFolder(entry.item)"
                >
                  <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                  </svg>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div v-if="(isInstalling || modpackSyncing || installingJava) && progress" class="install-progress">
        <div class="ip-icon">
          <Icon name="spinner" :size="16" spin />
        </div>
        <div class="ip-body">
          <div class="ip-row">
            <span class="ip-stage">{{ progress.stage }}</span>
            <span class="ip-percent">{{ Math.round(progress.percent) }}%</span>
          </div>
          <div class="ip-bar">
            <div class="ip-fill" :style="{ width: progress.percent + '%' }"></div>
          </div>
          <div class="ip-meta">{{ progress.detail }}</div>
        </div>
      </div>

      <ToastNotification
        :visible="!isInstalling && !modpackSyncing && !installingJava"
        :message="launcher.launchStatus"
        :variant="toastVariant"
        @dismiss="dismissToast"
      />

      <div class="play-panel">
        <div class="info-pill info-pill-server" :class="{ success: ipCopied }" v-tooltip="'Click to copy'" @click="copyServerIp(SERVER_IP)">
          <div class="info-pill-icon server">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="3" width="20" height="7" rx="2" />
              <rect x="2" y="14" width="20" height="7" rx="2" />
              <line x1="6" y1="6.5" x2="6.01" y2="6.5" />
              <line x1="6" y1="17.5" x2="6.01" y2="17.5" />
            </svg>
          </div>
          <div class="info-pill-text">
            <span class="info-pill-label">Server IP</span>
            <span class="info-pill-value">{{ SERVER_IP }}</span>
          </div>
          <transition name="copy-icon" mode="out-in">
            <svg v-if="!ipCopied" key="copy" class="info-pill-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="9" y="9" width="13" height="13" rx="2" />
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
            </svg>
            <svg v-else key="check" class="info-pill-copy" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="20 6 9 17 4 12" />
            </svg>
          </transition>
        </div>

        <div class="play-footer">
          <button
            class="play-btn"
            :class="{ 'is-stop': props.gameRunning, 'is-update': isInstalled && modpackUpdateAvailable && !props.gameRunning }"
            :disabled="isInstalling || (isLaunching && !props.gameRunning) || !hasJava || modpackSyncing"
            :title="isInstalled && modpackUpdateAvailable && modpackInfo ? `Modpack ${modpackInfo.version}${modpackSizeDisplay ? ' · ' + modpackSizeDisplay : ''}` : undefined"
            @click="handleMainButton"
          >
            <svg v-if="props.gameRunning" viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
              <rect x="5" y="5" width="14" height="14" rx="2" />
            </svg>
            <Icon v-else-if="isInstalling || isLaunching || modpackSyncing" name="spinner" :size="18" spin />
            <svg v-else-if="isInstalled && !modpackUpdateAvailable" viewBox="0 0 24 24" fill="currentColor" width="18" height="18">
              <polygon points="5 3 19 12 5 21 5 3" />
            </svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="18" height="18">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
          <span>{{ playButtonText }}</span>
        </button>
        </div>

        <div class="info-pill info-pill-playtime" v-if="instanceStats" v-tooltip="instanceStats.lastPlayed ? `Last played ${new Date(instanceStats.lastPlayed).toLocaleString()}` : 'No sessions yet'">
          <div class="info-pill-icon playtime">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10" />
              <polyline points="12 6 12 12 16 14" />
            </svg>
          </div>
          <div class="info-pill-text">
            <span class="info-pill-label">Total time played</span>
            <span class="info-pill-value">{{ playtimeDisplay }}</span>
          </div>
        </div>
      </div>
    </main>
</template>