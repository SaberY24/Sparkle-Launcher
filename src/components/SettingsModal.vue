<script setup lang="ts">
import { ref, onMounted, watch, computed, onUnmounted, shallowRef, nextTick } from "vue";
import { useLauncherStore } from "../stores/launcher";
import { useThemeStore } from "../stores/theme";
import { invoke } from "@tauri-apps/api/core";
import ColorPicker from "../components/ColorPicker.vue";
import ToggleSwitch from "../components/ui/ToggleSwitch.vue";  
import DropdownMenu from "../components/ui/DropdownMenu.vue";
import AppButton from "../components/ui/AppButton.vue";
import SettingsActionButton from "../components/ui/SettingsActionButton.vue";
import { formatRam, formatRamFull } from "../utils/formats";
import { useSelectedJavaLabel, useSelectedJavaMeta, useSelectedJavaVendor, useIsLauncherJavaSelected } from "../utils/selectors";
import { useDebouncedSave } from "../composables/useDebouncedSave";
import Icon from "../components/ui/Icon.vue";
import { useAppUpdater } from "../composables/useAppUpdater";
import { getVersion } from "@tauri-apps/api/app";

const appUpdater = useAppUpdater();
const appVersion = ref("");
onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch (e) {
    console.error("Failed to read app version:", e);
  }
});

const props = defineProps<{ initialSection?: string }>();
const emit = defineEmits<{ (e: "close"): void }>();
const launcher = useLauncherStore();
const themeStore = useThemeStore();
const currentSection = ref("appearance");
const useNativeTitlebar = ref(false);

const fullscreen = ref(false);
const javaDetectError = ref("");
const localJavaArgs = ref("");

// Resolution: separada en width y height
const resolutionWidth = ref(854);
const resolutionHeight = ref(480);

// OPTIMIZACIÓN: Usar shallowRef para arrays grandes para evitar reactividad profunda
const javaInstallations = shallowRef<{ path: string, version: string, major: number, vendor: string }[]>([]);
const selectedJavaPath = ref(launcher.javaPath || "");
const bundledJavaPath = ref("");
const javaLoading = ref(true);

const sections = [
  { id: "appearance", label: "Appearance", icon: "sun" },
  { id: "java", label: "Java", icon: "java" },
  { id: "game", label: "Game", icon: "game" },
  { id: "about", label: "About", icon: "about" },
];

const themeCards = [
  { value: "system" as const, label: "System default" },
  { value: "dark" as const, label: "Dark" },
  { value: "light" as const, label: "Light" },
];

// ---- Menú de opciones (⋮) de cada preset personalizado ----
// El posicionamiento dinámico, clic-afuera y Escape ahora los maneja
// DropdownMenu (un componente por cada preset personalizado).

// ---- Panel para agregar / editar / renombrar un color personalizado ----
const presetEditorOpen = ref(false);
const presetEditorMode = ref<"create" | "edit">("create");
const presetEditorIndex = ref<number | null>(null); // índice dentro de themeStore.customPresets, solo en modo "edit"
const presetEditorHex = ref("#000000");
const presetEditorName = ref("");
const presetEditorError = ref("");
const presetEditorNameInputRef = ref<HTMLInputElement | null>(null);
let presetEditorPreviousAccent = "";

function focusPresetEditorName() {
  nextTick(() => {
    presetEditorNameInputRef.value?.focus();
    presetEditorNameInputRef.value?.select();
  });
}

function startAddPreset() {
  presetEditorMode.value = "create";
  presetEditorIndex.value = null;
  presetEditorHex.value = themeStore.accentColor;
  presetEditorName.value = "";
  presetEditorError.value = "";
  presetEditorPreviousAccent = themeStore.accentColor;
  presetEditorOpen.value = true;
  focusPresetEditorName();
}

function startRenamePreset(idx: number) {
  const preset = themeStore.customPresets[idx];
  if (!preset) return;
  presetEditorMode.value = "edit";
  presetEditorIndex.value = idx;
  presetEditorHex.value = preset.hex;
  presetEditorName.value = preset.name;
  presetEditorError.value = "";
  presetEditorPreviousAccent = themeStore.accentColor;
  presetEditorOpen.value = true;
  focusPresetEditorName();
}

function startEditPresetColor(idx: number) {
  const preset = themeStore.customPresets[idx];
  if (!preset) return;
  presetEditorMode.value = "edit";
  presetEditorIndex.value = idx;
  presetEditorHex.value = preset.hex;
  presetEditorName.value = preset.name;
  presetEditorError.value = "";
  presetEditorPreviousAccent = themeStore.accentColor;
  presetEditorOpen.value = true;
  // No enfocamos el nombre acá: el usuario vino a tocar el color.
}

function onPresetEditorColorChange(hex: string) {
  presetEditorHex.value = hex;
  presetEditorError.value = "";
  // Vista previa en vivo mientras se elige el color
  themeStore.setAccent(hex);
}

function cancelPresetEditor() {
  // Si estábamos previsualizando un color en vivo, volvemos al que estaba activo antes de abrir el panel
  if (themeStore.accentColor.toLowerCase() !== presetEditorPreviousAccent.toLowerCase()) {
    themeStore.setAccent(presetEditorPreviousAccent);
  }
  presetEditorOpen.value = false;
  presetEditorError.value = "";
}

function savePresetEditor() {
  presetEditorError.value = "";
  const hex = presetEditorHex.value;
  const name = presetEditorName.value.trim();

  if (presetEditorMode.value === "create") {
    if (themeStore.customPresets.length >= 8) {
      presetEditorError.value = "You can save up to 8 custom colors.";
      return;
    }
    if (themeStore.customPresets.some((p) => p.hex.toLowerCase() === hex.toLowerCase())) {
      presetEditorError.value = "This color is already saved.";
      return;
    }
    themeStore.addCustomPreset(hex, name);
  } else if (presetEditorIndex.value !== null) {
    const ok = themeStore.updateCustomPreset(presetEditorIndex.value, { hex, name });
    if (!ok) {
      presetEditorError.value = "This color is already saved.";
      return;
    }
  }

  themeStore.setAccent(hex);
  saveAllSettings();
  presetEditorOpen.value = false;
}

function handleDeletePreset(idx: number) {
  const preset = themeStore.customPresets[idx];
  if (!preset) return;
  if (confirm(`Delete "${preset.name}"?`)) {
    themeStore.removeCustomPreset(idx);
    saveAllSettings();
  }
}

const systemIsDark = computed(() => {
  if (themeStore.theme === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  }
  return themeStore.theme === "dark";
});

const ramDisplay = computed(() => formatRam(launcher.ram));

const ramDisplayFull = computed(() => formatRamFull(launcher.ram));

const RAM_MIN = 1024;
const RAM_THUMB_PX = 16;

// RAM total real de la máquina (en MB), detectada por el backend con `sysinfo`.
// Mientras se detecta (o si falla), usamos un fallback razonable para que el
// slider no se rompa.
const RAM_FALLBACK_MAX = 16384;
const systemRamMb = ref<number | null>(null);

// La RAM física casi nunca se reporta exacta: el sistema operativo ve un
// poco menos de lo que dice la caja (memoria reservada para la placa madre,
// gráficos integrados, firmware, etc.), así que un kit de 16GB real puede
// figurar como 15.5-15.9GB para el SO. Antes usábamos Math.floor(), así que
// ese 15.9GB terminaba mostrando "15GB" en vez de "16GB". Ahora, si el valor
// detectado está cerca (±4%) de un tamaño "de fábrica" común, lo redondeamos
// a ese tamaño en vez de truncarlo.
const COMMON_RAM_SIZES_GB = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128];

function roundToCommonRamGb(rawGb: number): number {
  const closest = COMMON_RAM_SIZES_GB.reduce((best, size) =>
    Math.abs(size - rawGb) < Math.abs(best - rawGb) ? size : best
  );
  const withinTolerance = Math.abs(closest - rawGb) / closest <= 0.04;
  return withinTolerance ? closest : Math.round(rawGb);
}

// Tope del slider: la RAM total del sistema, redondeada al tamaño de
// fábrica más cercano para que calce con el `step` del input.
const RAM_MAX = computed(() => {
  if (!systemRamMb.value || systemRamMb.value < RAM_MIN + 1024) return RAM_FALLBACK_MAX;
  return roundToCommonRamGb(systemRamMb.value / 1024) * 1024;
});

async function loadSystemRam() {
  try {
    const totalMb = await invoke<number>("get_system_ram_mb");
    if (totalMb && totalMb > 0) {
      systemRamMb.value = totalMb;
      // Si el valor guardado supera la RAM real de esta máquina (por ejemplo,
      // si el settings.json vino de otro equipo con más RAM), lo bajamos al
      // nuevo máximo para no dejar el slider en un estado inválido.
      if (launcher.ram > RAM_MAX.value) {
        launcher.ram = RAM_MAX.value;
        saveAllSettings();
      }
    }
  } catch (e) {
    console.error("Failed to detect system RAM:", e);
  }
}

// OPTIMIZACIÓN GPU: ticks del slider calculados en función de la RAM real de
// la máquina en vez de una lista fija — antes siempre asumía 16GB de tope.
const ramTicksGb = computed(() => {
  const maxGb = Math.floor(RAM_MAX.value / 1024);
  if (maxGb <= 1) return [1];
  const base = [1, 2, 4, 6, 8, 12, 16].filter((g) => g <= maxGb);
  if (base[base.length - 1] !== maxGb) base.push(maxGb);
  return base;
});

// OPTIMIZACIÓN GPU: fórmula de posición en base al RAM_MAX dinámico
function ramTrackPosition(value: number): string {
  const pct = ((value - RAM_MIN) / (RAM_MAX.value - RAM_MIN)) * 100;
  return `calc(${RAM_THUMB_PX / 2}px + (${pct.toFixed(6)} / 100) * (100% - ${RAM_THUMB_PX}px))`;
}

// Fill del slider usando la misma fórmula que los ticks y el thumb
const ramFillWidth = computed(() => ramTrackPosition(launcher.ram));

// Usar función de hash unificada de utils/hash
// computeJavaHash ha sido reemplazado por hashJavaList

const isLauncherJavaSelected = useIsLauncherJavaSelected(selectedJavaPath, bundledJavaPath);
const selectedJavaLabel = useSelectedJavaLabel({ javaInstallations, selectedJavaPath, bundledJavaPath });

const selectedJavaMeta = useSelectedJavaMeta({ javaInstallations, selectedJavaPath, bundledJavaPath });
const selectedJavaVendor = useSelectedJavaVendor({ javaInstallations, selectedJavaPath, bundledJavaPath });

// OPTIMIZACIÓN: Bandera para evitar llamadas redundantes a saveAllSettings
let saveSettingsInFlight = false;

async function loadJavaInstallations() {
  try {
    const list = await invoke<any[]>("list_java_installations");
    javaInstallations.value = list.map(item => ({
      path: item.path,
      version: item.version,
      major: item.major,
      vendor: item.vendor || "Unknown",
    }));

    try {
      const bundled = await invoke<string | null>("get_bundled_java_path");
      bundledJavaPath.value = bundled || "";
    } catch (e) {
      bundledJavaPath.value = "";
    }

    if (launcher.javaPath) {
      const stillExists = javaInstallations.value.some(j => j.path === launcher.javaPath);
      if (stillExists) {
        selectedJavaPath.value = launcher.javaPath;
      } else if (bundledJavaPath.value) {
        selectedJavaPath.value = bundledJavaPath.value;
        launcher.javaPath = bundledJavaPath.value;
        await saveAllSettings();
      } else {
        selectedJavaPath.value = "";
        launcher.javaPath = "";
        await saveAllSettings();
      }
    } else if (bundledJavaPath.value) {
      selectedJavaPath.value = bundledJavaPath.value;
      launcher.javaPath = bundledJavaPath.value;
      await saveAllSettings();
    } else {
      selectedJavaPath.value = "";
      launcher.javaPath = "";
    }
  } catch (e) {
    console.error("Failed to list Java installations:", e);
  } finally {
    javaLoading.value = false;
  }
}

onMounted(async () => {
  const settings = await launcher.loadSettings();
  useNativeTitlebar.value = !(settings.custom_titlebar ?? true);

  loadSystemRam();

  // Parse resolution from settings
  const res = settings.resolution ?? "854x480";
  const resMatch = res.match(/(\d+)\s*x\s*(\d+)/i);
  if (resMatch) {
    resolutionWidth.value = parseInt(resMatch[1]);
    resolutionHeight.value = parseInt(resMatch[2]);
  }

  fullscreen.value = settings.fullscreen ?? false;

  localJavaArgs.value = launcher.javaArgs || settings.java_args || "";

  if (props.initialSection) {
    currentSection.value = props.initialSection;
  }

  if (!themeStore.isInitialized) {
    themeStore.initFromSettings(settings);
  }

  await loadJavaInstallations();

  document.addEventListener("keydown", onKeydown);
  document.addEventListener("click", onDocumentClick);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeydown);
  document.removeEventListener("click", onDocumentClick);
  if (launcher.javaArgs !== localJavaArgs.value) {
    launcher.javaArgs = localJavaArgs.value;
    saveAllSettings();
  }
});

function switchSection(section: string) {
  currentSection.value = section;
}

function getResolutionString() {
  return `${resolutionWidth.value}x${resolutionHeight.value}`;
}

async function saveAllSettings() {
  // OPTIMIZACIÓN: Evitar llamadas redundantes
  if (saveSettingsInFlight) return;
  saveSettingsInFlight = true;

  try {
    await invoke("save_settings", {
      settings: {
        ram: launcher.ram,
        java_path: launcher.javaPath,
        game_dir: launcher.gameDirectory,
        resolution: getResolutionString(),
        fullscreen: fullscreen.value,
        custom_titlebar: launcher.customTitlebar,
        theme: themeStore.theme,
        accent_color: themeStore.accentColor,
        custom_presets: themeStore.customPresets,
        java_args: localJavaArgs.value,
      },
    });
  } catch (e) {
    console.error("Failed to save settings:", e);
  } finally {
    saveSettingsInFlight = false;
  }
}

// Usar debounce unificado para guardado automático
const { save: debounceSave } = useDebouncedSave(saveAllSettings, 500);

watch([resolutionWidth, resolutionHeight], () => {
  debounceSave();
});

watch(fullscreen, () => {
  debounceSave();
});

watch(() => themeStore.theme, () => {
  debounceSave();
});

// Java dropdown
function selectLauncherJava() {
  if (bundledJavaPath.value) {
    selectedJavaPath.value = bundledJavaPath.value;
    launcher.javaPath = bundledJavaPath.value;
  } else {
    selectedJavaPath.value = "";
    launcher.javaPath = "";
  }
  javaDetectError.value = "";
  saveAllSettings();
}

function selectJava(path: string) {
  selectedJavaPath.value = path;
  launcher.javaPath = path;
  javaDetectError.value = "";
  saveAllSettings();
}

async function browseJavaAndClose(close: () => void) {
  await browseJava();
  close();
}

async function browseJava() {
  try {
    const path = await invoke<string>("browse_java");
    if (path) {
      // OPTIMIZACIÓN: Verificar si ya existe antes de hacer la llamada redundante
      if (!javaInstallations.value.some(j => j.path === path)) {
        let major = 0;
        let version = "Custom";
        let vendor = "Unknown";
        try {
          const list = await invoke<any[]>("list_java_installations");
          const found = list.find((j: any) => j.path === path);
          if (found) {
            major = found.major;
            version = found.version;
            vendor = found.vendor || "Unknown";
          }
        } catch (e) { /* ignore */ }

        // OPTIMIZACIÓN: Usar push directo en lugar de crear nuevo array
        javaInstallations.value.push({
          path,
          version,
          major,
          vendor,
        });
      }
      selectedJavaPath.value = path;
      launcher.javaPath = path;
      javaDetectError.value = "";
      await saveAllSettings();
    }
  } catch (e) {
    console.error("Browse Java failed:", e);
  }
}

const clearingCache = ref(false);
const clearCacheResult = ref<"idle" | "success" | "error">("idle");
let clearCacheResetTimeout: number | null = null;

async function clearModsCache() {
  if (clearingCache.value) return;
  if (!confirm("This clears the cached mod info and CurseForge icon data. The next time you open the mods list it will be rescanned from scratch (slower just that one time). Continue?")) {
    return;
  }
  clearingCache.value = true;
  clearCacheResult.value = "idle";
  try {
    await invoke("clear_mod_cache");
    clearCacheResult.value = "success";
  } catch (e) {
    console.error("Failed to clear mod cache:", e);
    clearCacheResult.value = "error";
  } finally {
    clearingCache.value = false;
    if (clearCacheResetTimeout) window.clearTimeout(clearCacheResetTimeout);
    clearCacheResetTimeout = window.setTimeout(() => {
      clearCacheResult.value = "idle";
    }, 3000);
  }
}

async function close() {
  // Si el panel de agregar/editar color seguía abierto, puede haber un color
  // en vista previa en vivo que nunca se guardó como preset (el usuario solo
  // lo estaba probando). Antes de cerrar el modal lo revertimos al que
  // estaba activo antes de abrir el panel — si no, el color quedaba
  // aplicado igual aunque no apareciera en la lista de presets.
  if (presetEditorOpen.value) {
    cancelPresetEditor();
  }
  if (launcher.javaArgs !== localJavaArgs.value) {
    launcher.javaArgs = localJavaArgs.value;
  }
  await saveAllSettings();
  emit("close");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  // Escape cierra primero la capa que esté abierta encima del modal. El
  // panel de color se maneja acá directamente; el menú ⋮ de cada preset
  // ahora es un DropdownMenu independiente que se cierra solo y frena la
  // propagación del Escape (ver DropdownMenu.vue), así que no hace falta
  // repetir esa lógica acá.
  if (presetEditorOpen.value) {
    cancelPresetEditor();
    return;
  }
  close();
}

function onDocumentClick(e: MouseEvent) {
  // OJO: usamos composedPath() en vez de target.closest(). Si el clic dispara
  // una acción que remueve ese elemento del DOM (ej. confirm() al borrar un
  // color fuerza a Vue a aplicar sus cambios pendientes mientras el diálogo
  // está abierto, o el propio dropdown se cierra), target.closest() ya no
  // encuentra el ancestro y el modal se cerraba solo. composedPath() captura
  // la ruta del clic en el momento del evento, así que es inmune a eso.
  // También incluimos .preset-menu: al estar teletransportado a <body> para
  // poder posicionarse libremente, vive fuera de .settings-modal en el DOM.
  const path = e.composedPath() as HTMLElement[];
  const insideModal = path.some(
    (el) =>
      el instanceof HTMLElement &&
      (el.classList.contains("settings-modal") ||
        el.classList.contains("settings-close") ||
        el.classList.contains("preset-menu"))
  );
  if (insideModal) return;
  close();
}

function selectPreset(hex: string) {
  themeStore.setAccent(hex);
  saveAllSettings();
}

// Usar debounce unificado para javaArgs
const { save: saveJavaArgs } = useDebouncedSave(() => {
  launcher.javaArgs = localJavaArgs.value;
  saveAllSettings();
}, 1000);

watch(localJavaArgs, () => {
  launcher.javaArgs = localJavaArgs.value;
});

watch(useNativeTitlebar, async (newVal) => {
  launcher.customTitlebar = !newVal;
  try {
    await invoke("set_window_decorations", { decorations: newVal });
    await saveAllSettings();
  } catch (e) {
    console.error("Failed to toggle decorations:", e);
  }
});
</script>

<template>
  <teleport to="body">
    <div
      class="modal-overlay"
      @click.self="close"
      tabindex="-1"
      role="dialog"
      aria-modal="true"
      aria-label="Settings"
    >
      <div class="settings-modal">
        <aside class="settings-sidebar">
          <div class="settings-sidebar-header">
            <h3>Settings</h3>
          </div>
          <nav class="settings-nav" role="tablist">
            <button
              v-for="section in sections"
              :key="section.id"
              class="settings-nav-item"
              :class="{ active: currentSection === section.id }"
              @click="switchSection(section.id)"
              role="tab"
              :aria-selected="currentSection === section.id"
            >
              <Icon v-if="section.icon === 'sun'" name="sun" :size="24" />
              <Icon v-else-if="section.icon === 'java'" name="java" :size="24" />
              <Icon v-else-if="section.icon === 'game'" name="game" :size="24" />
              <Icon v-else-if="section.icon === 'about'" name="info" :size="24" />
              <span>{{ section.label }}</span>
            </button>
          </nav>
        </aside>

        <div class="settings-content">
          <div class="settings-content-header">
            <button class="settings-close" @click="close" aria-label="Close settings">
              <Icon name="x" :size="24" />
            </button>
          </div>

          <div class="settings-content-body">
            <!-- Appearance Panel -->
            <div v-show="currentSection === 'appearance'" class="settings-panel" :class="{ active: currentSection === 'appearance' }">
              <h4 class="settings-panel-title">Appearance</h4>
              <p class="settings-panel-desc">Customize the visual style of the application.</p>

              <div class="settings-section-block">
                <span class="settings-section-label">Theme</span>
                <p class="settings-section-sublabel">Choose how the app looks</p>
                <div class="theme-picker" role="radiogroup" aria-label="Theme selection">
                  <button
                    v-for="t in themeCards"
                    :key="t.value"
                    type="button"
                    class="theme-card"
                    :class="{ active: themeStore.theme === t.value }"
                    @click="themeStore.setTheme(t.value)"
                    role="radio"
                    :aria-checked="themeStore.theme === t.value"
                  >
                    <div class="theme-preview" :data-theme="t.value === 'system' ? (systemIsDark ? 'dark' : 'light') : t.value">
                      <div class="tp-hud">
                        <div class="tp-sidebar">
                          <div class="tp-dot active"></div>
                          <div class="tp-dot"></div>
                          <div class="tp-dot"></div>
                        </div>
                        <div class="tp-content">
                          <div class="tp-line"></div>
                          <div class="tp-line short"></div>
                        </div>
                      </div>
                    </div>
                    <div class="theme-footer">
                      <div class="theme-radio">
                        <div class="theme-radio-dot"></div>
                      </div>
                      <span class="theme-name">{{ t.label }}</span>
                    </div>
                  </button>
                </div>
              </div>

              <div class="settings-section-block" style="margin-top: 28px">
                <span class="settings-section-label">Color</span>
                <p class="settings-section-sublabel">Choose the main app color, or create your own.</p>

                <div class="color-section">
                  <h5 class="color-section-title">Presets</h5>
                  <div class="color-presets-grid">
                    <button
                      v-for="preset in themeStore.defaultPresets"
                      :key="preset.id"
                      type="button"
                      class="color-preset"
                      :class="{ active: themeStore.accentColor.toLowerCase() === preset.hex.toLowerCase() }"
                      @click="selectPreset(preset.hex)"
                      :aria-label="`Select ${preset.label} color`"
                    >
                      <div class="color-preset-preview" :style="{ background: preset.hex }"></div>
                      <span class="color-preset-label">{{ preset.label }}</span>
                    </button>

                    <div
                      v-for="(preset, idx) in themeStore.customPresets"
                      :key="`custom-${idx}-${preset.hex}`"
                      class="color-preset custom"
                      :class="{ active: themeStore.accentColor.toLowerCase() === preset.hex.toLowerCase() }"
                    >
                      <button
                        type="button"
                        class="color-preset-main"
                        @click="selectPreset(preset.hex)"
                        :aria-label="`Select ${preset.name} color`"
                      >
                        <div class="color-preset-preview" :style="{ background: preset.hex }"></div>
                        <span class="color-preset-label">{{ preset.name }}</span>
                      </button>

                      <DropdownMenu use-fixed :menu-width="180" :menu-height="140">
                        <template #trigger="{ toggle, isOpen }">
                          <button
                            type="button"
                            class="preset-menu-btn"
                            :class="{ open: isOpen }"
                            @click.stop="toggle"
                            v-tooltip="'More options'"
                            :aria-label="`Options for ${preset.name}`"
                          >
                            <svg v-once viewBox="0 0 24 24" fill="currentColor">
                              <circle cx="12" cy="5" r="1.8"/>
                              <circle cx="12" cy="12" r="1.8"/>
                              <circle cx="12" cy="19" r="1.8"/>
                            </svg>
                          </button>
                        </template>
                        <template #default="{ close }">
                          <div class="preset-menu">
                            <button type="button" class="preset-menu-item" @click="close(); startRenamePreset(idx)">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <path d="M12 20h9"/>
                                <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4 12.5-12.5z"/>
                              </svg>
                              Rename
                            </button>
                            <button type="button" class="preset-menu-item" @click="close(); startEditPresetColor(idx)">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <circle cx="13.5" cy="6.5" r=".5"/>
                                <circle cx="17.5" cy="10.5" r=".5"/>
                                <circle cx="8.5" cy="7.5" r=".5"/>
                                <circle cx="6.5" cy="12.5" r=".5"/>
                                <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.688-1.688h1.972c3.71 0 6.687-2.977 6.687-6.688C22.12 6.5 17.5 2 12 2z"/>
                              </svg>
                              Edit color
                            </button>
                            <div class="preset-menu-divider"></div>
                            <button type="button" class="preset-menu-item danger" @click="close(); handleDeletePreset(idx)">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <polyline points="3 6 5 6 21 6"/>
                                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>
                              </svg>
                              Delete
                            </button>
                          </div>
                        </template>
                      </DropdownMenu>
                    </div>

                    <button
                      type="button"
                      class="color-preset color-preset-add"
                      @click="startAddPreset"
                      aria-label="Add a custom color"
                    >
                      <div class="color-preset-preview add-preview">
                        <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <line x1="12" y1="5" x2="12" y2="19"/>
                          <line x1="5" y1="12" x2="19" y2="12"/>
                        </svg>
                      </div>
                      <span class="color-preset-label">Add color</span>
                    </button>
                  </div>
                </div>

                <transition name="editor-expand">
                  <div v-if="presetEditorOpen" class="preset-editor-panel">
                    <div class="preset-editor-header">
                      <span>{{ presetEditorMode === "create" ? "Add a color" : "Edit color" }}</span>
                      <button type="button" class="preset-editor-close" @click="cancelPresetEditor" aria-label="Close">
                        <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <line x1="18" y1="6" x2="6" y2="18"/>
                          <line x1="6" y1="6" x2="18" y2="18"/>
                        </svg>
                      </button>
                    </div>

                    <div class="preset-editor-body">
                      <label class="preset-editor-field">
                        <span class="preset-editor-field-label">Name</span>
                        <input
                          ref="presetEditorNameInputRef"
                          v-model="presetEditorName"
                          type="text"
                          class="preset-editor-name-input"
                          placeholder="e.g. Ocean Blue"
                          maxlength="24"
                          @keydown.enter="savePresetEditor"
                        />
                      </label>

                      <div class="preset-editor-field">
                        <span class="preset-editor-field-label">Color</span>
                        <ColorPicker
                          :model-value="presetEditorHex"
                          @update:model-value="onPresetEditorColorChange"
                        />
                      </div>
                    </div>

                    <p v-if="presetEditorError" class="preset-editor-error">{{ presetEditorError }}</p>

                    <div class="preset-editor-footer">
                      <button type="button" class="btn-preset-editor-cancel" @click="cancelPresetEditor">Cancel</button>
                      <button type="button" class="btn-save-preset" @click="savePresetEditor">
                        <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <polyline points="20 6 9 17 4 12"/>
                        </svg>
                        {{ presetEditorMode === "create" ? "Add & apply" : "Save changes" }}
                      </button>
                    </div>
                  </div>
                </transition>
              </div>

              <div class="settings-row" style="margin-top: 28px; border-top: 1px solid var(--border-color); padding-top: 20px;">
                <div class="settings-row-info">
                  <span class="settings-row-label">Use native titlebar</span>
                  <span class="settings-row-desc">Use Windows native window controls instead of custom</span>
                </div>
                <ToggleSwitch v-model="useNativeTitlebar" size="sm" />
              </div>

              <div class="settings-section-block" style="margin-top: 20px; border-top: 1px solid var(--border-color); padding-top: 20px;">
                <span class="settings-section-label">Mods cache</span>
                <p class="settings-section-sublabel">
                  Mod info and CurseForge icons are cached on disk so the mods list loads instantly. Clear it if something looks outdated.
                </p>
                <SettingsActionButton
                  :status="clearingCache ? 'loading' : clearCacheResult"
                  :disabled="clearingCache"
                  success-style="neutral"
                  icon="folder"
                  @click="clearModsCache"
                  idle-label="Clear mods cache"
                  loading-label="Clearing..."
                  success-label="Cache cleared"
                  error-label="Failed, try again"
                />
              </div>
            </div>

            <!-- Java Panel -->
            <div v-show="currentSection === 'java'" class="settings-panel" :class="{ active: currentSection === 'java' }">
              <h4 class="settings-panel-title">Java</h4>
              <p class="settings-panel-desc">Select the Java runtime used to launch the game.</p>

              <div class="java-section">
                <div class="java-field">
                  <span class="settings-section-label">Java Runtime</span>
                  <p class="settings-section-sublabel">Choose a JVM. Sparkle can use its own bundled runtime or a system installation.</p>

                  <DropdownMenu full-width>
                    <template #trigger="{ toggle, isOpen }">
                      <button class="java-dropdown-trigger" @click="toggle" :class="{ open: isOpen, loading: javaLoading }" :disabled="javaLoading">
                        <div class="java-trigger-info">
                          <template v-if="javaLoading">
                            <div class="java-trigger-top">
                              <span class="java-trigger-name java-trigger-skeleton">Detecting Java...</span>
                            </div>
                            <span class="java-trigger-meta">Please wait a moment</span>
                          </template>
                          <template v-else>
                            <div class="java-trigger-top">
                              <span class="java-trigger-name">{{ selectedJavaLabel }}</span>
                              <span v-if="selectedJavaVendor" class="java-trigger-vendor" v-once>{{ selectedJavaVendor }}</span>
                            </div>
                            <span class="java-trigger-meta">{{ selectedJavaMeta }}</span>
                          </template>
                        </div>
                        <svg class="java-chevron" :class="{ open: isOpen }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                          <polyline points="6 9 12 15 18 9"/>
                        </svg>
                      </button>
                    </template>
                    <template #default="{ close }">
                      <div class="java-dropdown-menu">
                        <div
                          class="java-dropdown-item"
                          :class="{ active: isLauncherJavaSelected || !selectedJavaPath }"
                          @click="selectLauncherJava(); close()"
                        >
                          <div class="java-item-header">
                            <div class="java-item-icon">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                                <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
                              </svg>
                            </div>
                            <div class="java-item-info">
                              <span class="java-item-name">Use Launcher Java</span>
                              <span class="java-item-desc">Managed by Sparkle — always compatible</span>
                            </div>
                            <span v-if="bundledJavaPath" class="java-item-badge bundled">Ready</span>
                            <span v-else class="java-item-badge missing">Not installed</span>
                          </div>
                        </div>

                        <div v-if="javaInstallations.length > 0" class="java-dropdown-divider"></div>

                        <div
                          v-for="j in javaInstallations"
                          :key="j.path"
                          class="java-dropdown-item"
                          :class="{ active: selectedJavaPath === j.path }"
                          @click="selectJava(j.path); close()"
                        >
                          <div class="java-item-header">
                            <div class="java-item-icon">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                                <polyline points="16 18 22 12 16 6"/>
                                <polyline points="8 6 2 12 8 18"/>
                              </svg>
                            </div>
                            <div class="java-item-info">
                              <span class="java-item-name">{{ j.vendor }} {{ j.major }}</span>
                              <span class="java-item-desc">{{ j.version }}</span>
                              <span class="java-item-path-text">{{ j.path }}</span>
                            </div>
                            <span class="java-item-badge">{{ j.major }}</span>
                          </div>
                        </div>

                        <div class="java-dropdown-divider"></div>

                        <div class="java-dropdown-item browse" @click="browseJavaAndClose(close)">
                          <div class="java-item-header">
                            <div class="java-item-icon">
                              <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                                <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/>
                              </svg>
                            </div>
                            <span class="java-item-name">Browse for Java...</span>
                          </div>
                        </div>
                      </div>
                    </template>
                  </DropdownMenu>

                  <transition name="shake">
                    <div v-if="javaDetectError" class="java-error" role="alert">
                      <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <line x1="12" y1="8" x2="12" y2="12"/>
                        <line x1="12" y1="16" x2="12.01" y2="16"/>
                      </svg>
                      <span>{{ javaDetectError }}</span>
                    </div>
                  </transition>
                </div>

                <div class="java-field memory-field">
                  <div class="memory-header">
                    <div>
                      <span class="settings-section-label">Memory Allocation</span>
                      <p class="settings-section-sublabel">RAM assigned to the Minecraft process</p>
                    </div>
                    <div class="memory-value-block">
                      <span class="memory-gb">{{ ramDisplay }}</span>
                      <span class="memory-mb">{{ ramDisplayFull }}</span>
                    </div>
                  </div>
                  <div class="ram-slider-area">
                    <div class="ram-track-bg">
                      <div class="ram-ticks">
                        <div
                          v-for="tick in ramTicksGb"
                          :key="tick"
                          class="ram-tick"
                          :class="{ active: launcher.ram >= tick * 1024 }"
                          :style="{ left: ramTrackPosition(tick * 1024) }"
                        >
                          <div class="tick-line"></div>
                          <span class="tick-label">{{ tick }}G</span>
                        </div>
                      </div>
                      <div class="ram-fill" :style="{ width: ramFillWidth }"></div>
                    </div>
                    <input
                      v-model.number="launcher.ram"
                      type="range"
                      min="1024"
                      :max="RAM_MAX"
                      step="1024"
                      class="ram-slider"
                      aria-label="RAM allocation"
                      @change="saveAllSettings"
                    />
                  </div>
                </div>

                <div class="java-field args-field">
                  <span class="settings-section-label">Java Arguments</span>
                  <p class="settings-section-sublabel">Additional JVM flags passed when launching the game</p>
                  <div class="args-textarea-wrap">
                    <textarea
                      v-model="localJavaArgs"
                      class="args-textarea"
                      rows="3"
                      placeholder="-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200..."
                      spellcheck="false"
                      @blur="saveJavaArgs"
                    ></textarea>
                    <div class="args-hint">
                      <svg v-once viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"/>
                        <line x1="12" y1="16" x2="12" y2="12"/>
                        <line x1="12" y1="8" x2="12.01" y2="8"/>
                      </svg>
                      <span>One argument per line or space separated.</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Game Panel -->
            <div v-show="currentSection === 'game'" class="settings-panel" :class="{ active: currentSection === 'game' }">
              <h4 class="settings-panel-title">Game</h4>
              <p class="settings-panel-desc">Configure game launch settings.</p>

              <div class="settings-section-block">
                <span class="settings-section-label">Resolution</span>
                <p class="settings-section-sublabel">Set the window width and height in pixels</p>
                <div class="resolution-row">
                  <div class="resolution-field">
                    <label class="resolution-field-label">Width</label>
                    <input
                      v-model.number="resolutionWidth"
                      type="number"
                      min="1"
                      max="7680"
                      placeholder="854"
                      class="resolution-input"
                      aria-label="Screen width"
                    />
                  </div>
                  <span class="resolution-x">×</span>
                  <div class="resolution-field">
                    <label class="resolution-field-label">Height</label>
                    <input
                      v-model.number="resolutionHeight"
                      type="number"
                      min="1"
                      max="4320"
                      placeholder="480"
                      class="resolution-input"
                      aria-label="Screen height"
                    />
                  </div>
                </div>
                <div class="resolution-presets">
                  <AppButton @click="resolutionWidth = 854; resolutionHeight = 480">854 × 480</AppButton>
                  <AppButton @click="resolutionWidth = 1280; resolutionHeight = 720">1280 × 720</AppButton>
                  <AppButton @click="resolutionWidth = 1920; resolutionHeight = 1080">1920 × 1080</AppButton>
                  <AppButton @click="resolutionWidth = 2560; resolutionHeight = 1440">2560 × 1440</AppButton>
                </div>
              </div>

              <div class="settings-row" style="margin-top: 20px">
                <div class="settings-row-info">
                  <span class="settings-row-label">Fullscreen</span>
                </div>
                <ToggleSwitch v-model="fullscreen" size="sm" />
              </div>
            </div>

            <!-- About Panel -->
            <div v-show="currentSection === 'about'" class="settings-panel" :class="{ active: currentSection === 'about' }">
              <h4 class="settings-panel-title">About</h4>
              <p class="settings-panel-desc">Sparkle App version and updates.</p>

              <div class="settings-section-block">
                <span class="settings-section-label">Version</span>
                <p class="settings-section-sublabel">{{ appVersion ? `Sparkle App v${appVersion}` : "Reading version..." }}</p>

                <SettingsActionButton
                  :status="
                    appUpdater.status.value === 'checking' ? 'loading'
                    : appUpdater.status.value === 'downloading' ? 'loading'
                    : appUpdater.status.value === 'up-to-date' ? 'success'
                    : appUpdater.status.value === 'available' ? 'success'
                    : appUpdater.status.value === 'ready' ? 'success'
                    : appUpdater.status.value === 'error' ? 'error'
                    : 'idle'
                  "
                  :disabled="appUpdater.status.value === 'checking' || appUpdater.status.value === 'downloading'"
                  @click="
                    appUpdater.status.value === 'available' ? appUpdater.downloadAndInstall() :
                    appUpdater.status.value === 'ready' ? appUpdater.restartNow() :
                    appUpdater.checkForUpdate()
                  "
                  idle-label="Check for updates"
                  loading-label="Checking..."
                  :success-label="
                    appUpdater.status.value === 'available' ? `Update available — v${appUpdater.version.value}`
                    : appUpdater.status.value === 'ready' ? 'Ready — click to restart'
                    : 'Up to date'
                  "
                  error-label="Couldn't check, retry"
                >
                  <template #idle-icon>
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" width="16" height="16">
                      <polyline points="23 4 23 10 17 10" />
                      <polyline points="1 20 1 14 7 14" />
                      <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
                    </svg>
                  </template>
                </SettingsActionButton>

                <p v-if="appUpdater.status.value === 'error' && appUpdater.error.value" class="settings-section-sublabel" style="margin-top: 10px; color: var(--danger);">
                  {{ appUpdater.error.value }}
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 40;
  padding: 24px;
  /* OPTIMIZACIÓN GPU: Backface visibility para evitar artefactos */
  backface-visibility: hidden;
  /* Consistencia con HomeView: misma función de timing */
  animation: overlayFadeIn 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  contain: strict;
}

@keyframes overlayFadeIn {
  from { 
    opacity: 0; 
    will-change: opacity; 
  }
  to { 
    opacity: 1; 
    will-change: auto; 
  }
}

.settings-modal {
  width: min(900px, 92vw);
  height: min(640px, 85vh);
  max-width: 100%;
  max-height: 100%;
  background: linear-gradient(180deg, 
    var(--bg-card) 0%, 
    var(--bg-secondary) 100%
  );
  border: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
  border-radius: 16px;
  display: flex;
  overflow: hidden;
  box-shadow: 
    0 8px 24px rgba(0, 0, 0, 0.22),
    inset 0 1px 0 rgba(255,255,255,0.04),
    inset 0 -1px 0 rgba(0,0,0,0.1);
  text-align: left;
  contain: content;
  will-change: transform, opacity;
  animation: modalSlideIn 0.2s cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes modalSlideIn {
  from { 
    opacity: 0; 
    transform: scale(0.98) translateY(4px); 
    will-change: opacity, transform; 
  }
  to { 
    opacity: 1; 
    transform: scale(1) translateY(0); 
    will-change: auto; 
  }
}

.settings-sidebar {
  width: 180px;
  min-width: 180px;
  background: var(--bg-secondary);
  border-right: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  display: flex;
  flex-direction: column;
  padding: 16px 12px;
  flex-shrink: 0;
  contain: content;
  box-shadow: inset -1px 0 0 rgba(0,0,0,0.05);
}

.settings-sidebar-header {
  padding: 0 10px 12px;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
  margin-bottom: 10px;
  box-shadow: inset 0 -1px 0 rgba(0,0,0,0.05);
  background: var(--bg-secondary);
}

.settings-sidebar-header h3 {
  font-size: 15px;
  font-weight: 800;
  color: var(--text-primary);
  letter-spacing: -0.3px;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.settings-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  text-align: left;
  -webkit-app-region: no-drag;
  position: relative;
  will-change: background-color, color, transform;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

.settings-nav-item:hover {
  background: color-mix(in srgb, var(--bg-hover) 60%, transparent);
  color: var(--text-secondary);
  transform: translateX(2px) translateY(-1px);
  box-shadow: 0 2px 6px rgba(0,0,0,0.05);
}

.settings-nav-item.active {
  background: var(--accent-glow);
  color: var(--accent-display);
  font-weight: 700;
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-primary) 20%, transparent);
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
}

.settings-nav-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 16px;
  background: var(--accent-primary);
  border-radius: 0 3px 3px 0;
}

.settings-nav-item svg {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.settings-content {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-width: 0;
  background: var(--bg-primary);
}

.settings-content-header {
  height: 48px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
  flex-shrink: 0;
  box-shadow: inset 0 -1px 0 rgba(0,0,0,0.05);
  background: var(--bg-primary);
}

.settings-close {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  -webkit-app-region: no-drag;
  padding: 0;
  will-change: background, color, transform;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

.settings-close:hover {
  background: color-mix(in srgb, var(--bg-hover) 60%, transparent);
  color: var(--text-primary);
  transform: scale(1.05) translateY(-1px);
  box-shadow: 0 4px 10px color-mix(in srgb, var(--text-muted) 15%, transparent);
}

.settings-close svg {
  width: 15px;
  height: 15px;
}

.settings-content-body {
  padding: 20px 24px 24px;
  overflow-y: auto;
  flex-grow: 1;
  contain: strict;
}

.settings-content-body::-webkit-scrollbar {
  width: 5px;
}

.settings-content-body::-webkit-scrollbar-track {
  background: transparent;
}

.settings-content-body::-webkit-scrollbar-thumb {
  background: color-mix(in srgb, var(--border-color) 70%, transparent);
  border-radius: 3px;
}

.settings-panel {
  display: none;
  background: transparent;
  animation: fadeIn 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: display, opacity, transform;
}

.settings-panel.active {
  display: block;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateX(4px); will-change: opacity, transform; }
  to { opacity: 1; transform: translateX(0); will-change: auto; }
}

.settings-panel-title {
  font-size: 20px;
  font-weight: 800;
  color: var(--text-primary);
  margin-bottom: 4px;
  letter-spacing: -0.3px;
}

.settings-panel-desc {
  font-size: 13px;
  color: var(--text-muted);
  margin-bottom: 20px;
  line-height: 1.5;
  font-weight: 500;
}

.settings-section-block {
  margin-bottom: 8px;
}

.settings-section-label {
  display: block;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 4px;
  letter-spacing: -0.01em;
}

.settings-section-sublabel {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 14px;
  line-height: 1.5;
  font-weight: 500;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
  box-shadow: inset 0 -1px 0 rgba(0,0,0,0.03);
}

.settings-row:last-child {
  border-bottom: none;
  box-shadow: none;
}

.settings-row-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.settings-row-label {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.01em;
}

.settings-row-desc {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 500;
}

.theme-picker {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

@media (max-width: 720px) {
  .theme-picker {
    grid-template-columns: repeat(2, 1fr);
  }
}

@media (max-width: 480px) {
  .theme-picker {
    grid-template-columns: 1fr;
  }
}

.theme-card {
  position: relative;
  background: var(--bg-card);
  border: 2px solid transparent;
  border-radius: var(--radius-md);
  padding: 0;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  -webkit-app-region: no-drag;
  text-align: left;
  will-change: transform, border-color, box-shadow;
  box-shadow: inset 0 1px 0 rgba(0,0,0,0.04);
}

.theme-card:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--border-hover) 70%, transparent);
  box-shadow: 0 6px 16px rgba(0,0,0,0.12), inset 0 1px 0 rgba(255,255,255,0.04);
  will-change: transform, border-color, box-shadow;
}

.theme-card.active {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-primary) 20%, transparent), 0 6px 16px rgba(0, 0, 0, 0.12);
}

.theme-preview {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  isolation: isolate;
  border-bottom: 1px solid color-mix(in srgb, var(--border-color) 40%, transparent);
  --tp-hud-bg: #0f0f16;
  --tp-hud-border: #1a1a2e;
  --tp-sidebar-bg: #0a0a0f;
  --tp-sidebar-border: #1a1a2e;
  --tp-content-bg: #0f0f16;
  --tp-line-bg: #3a3a4a;
  --tp-dot-inactive: #4a4a5a;
}

.theme-preview[data-theme="dark"] {
  background: #1a1a24;
  --tp-hud-bg: #0f0f16;
  --tp-hud-border: #1a1a2e;
  --tp-sidebar-bg: #0a0a0f;
  --tp-sidebar-border: #1a1a2e;
  --tp-content-bg: #0f0f16;
  --tp-line-bg: #3a3a4a;
  --tp-dot-inactive: #4a4a5a;
}

.theme-preview[data-theme="light"] {
  background: #f0f0f4;
  --tp-hud-bg: #ffffff;
  --tp-hud-border: #dcdcdc;
  --tp-sidebar-bg: #e8e8e8;
  --tp-sidebar-border: #dcdcdc;
  --tp-content-bg: #ffffff;
  --tp-line-bg: #c8c8c8;
  --tp-dot-inactive: #989898;
}

.tp-hud {
  display: flex;
  width: 100%;
  height: 56px;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  border: 1px solid var(--tp-hud-border);
  background: var(--tp-hud-bg);
  will-change: border-color;
}

.tp-sidebar {
  width: 20%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 8px 0;
  gap: 6px;
  border-right: 1px solid var(--tp-sidebar-border);
  background: var(--tp-sidebar-bg);
}

.tp-dot {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--tp-dot-inactive);
  transition: background 0.3s ease;
}

.tp-dot.active {
  background: var(--accent-primary);
  box-shadow: 0 0 6px var(--accent-glow);
}

.tp-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 12px;
  gap: 8px;
  background: var(--tp-content-bg);
}

.tp-line {
  height: 5px;
  border-radius: 3px;
  width: 80%;
  background: var(--tp-line-bg);
}

.tp-line.short {
  width: 55%;
}

.theme-footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  background: var(--bg-secondary);
  border-top: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  box-shadow: inset 0 1px 0 rgba(0,0,0,0.04);
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: background-color;
}

.theme-radio {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--border-color) 85%, transparent);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: border-color, box-shadow, background-color;
  background: color-mix(in srgb, var(--bg-hover) 25%, transparent);
  box-shadow: inset 0 1px 2px rgba(0,0,0,0.06);
}

.theme-card:hover .theme-radio {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-primary) 10%, transparent);
}

.theme-card.active .theme-radio {
  border-color: var(--accent-primary);
  border-width: 2px;
  background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.theme-radio-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent-primary);
  transform: scale(0);
  opacity: 0;
  transition: all 0.25s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 0 0 0 var(--accent-glow);
  will-change: transform, opacity, box-shadow;
}

.theme-card.active .theme-radio-dot {
  transform: scale(1);
  opacity: 1;
  box-shadow: 0 0 4px var(--accent-glow);
}

.theme-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-muted);
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: color, font-weight;
  letter-spacing: 0.01em;
}

.theme-card:hover .theme-name {
  font-weight: 700;
  color: var(--text-primary);
}

.theme-card.active .theme-name {
  color: var(--accent-primary);
  font-weight: 700;
}

.color-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.color-section-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-muted);
  font-weight: 700;
  margin-bottom: 4px;
}

.color-presets-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
}

@media (max-width: 600px) {
  .color-presets-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

.color-preset {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border: 1px solid color-mix(in srgb, var(--border-color) 40%, transparent);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  color: var(--text-primary);
  font-family: inherit;
  -webkit-app-region: no-drag;
  position: relative;
  will-change: transform, border-color, box-shadow, background-color;
  box-shadow: inset 0 1px 0 rgba(0,0,0,0.04);
}

.color-preset:hover {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-primary) 8%, transparent);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
}

.color-preset.active {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-glow) 15%, transparent);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-primary) 30%, transparent);
}

.color-preset-preview {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 2px solid color-mix(in srgb, var(--border-color) 40%, transparent);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.12);
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  position: relative;
  will-change: transform, border-color, box-shadow;
}

.color-preset.active .color-preset-preview {
  border-color: var(--accent-primary);
  transform: scale(1.05);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-primary) 40%, transparent), 0 2px 6px rgba(0, 0, 0, 0.12);
}

.color-preset-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-muted);
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: color, font-weight;
  letter-spacing: 0.01em;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.color-preset:hover .color-preset-label {
  color: var(--text-primary);
  font-weight: 600;
}

.color-preset.active .color-preset-label {
  color: var(--accent-primary);
  font-weight: 700;
}

/* ---- Preset personalizado: wrapper (div) + botón principal + menú ⋮ ---- */
.color-preset.custom {
  padding: 0;
  display: block;
}

.color-preset-main {
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: transparent;
  border: none;
  border-radius: inherit;
  cursor: pointer;
  color: inherit;
  font-family: inherit;
  -webkit-app-region: no-drag;
}

.preset-menu-btn {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 20px;
  height: 20px;
  border-radius: 6px;
  border: 1px solid transparent;
  background: color-mix(in srgb, var(--bg-secondary) 70%, transparent);
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transform: scale(0.85);
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  padding: 0;
  will-change: opacity, transform, background, color;
  -webkit-app-region: no-drag;
}

.color-preset.custom:hover .preset-menu-btn,
.preset-menu-btn.open {
  opacity: 1;
  transform: scale(1);
}

.preset-menu-btn:hover,
.preset-menu-btn.open {
  background: var(--bg-hover);
  border-color: color-mix(in srgb, var(--border-color) 50%, transparent);
  color: var(--text-primary);
}

.preset-menu-btn svg {
  width: 13px;
  height: 13px;
}

.preset-menu {
  /* position/top/bottom/right/min-width/z-index ahora los aplica el propio
     DropdownMenu.vue (su .dropdown-menu-panel), acá solo queda lo visual */
  background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-secondary) 100%);
  border: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
  border-radius: 12px;
  padding: 6px;
  box-shadow:
    0 6px 20px rgba(0,0,0,0.2),
    inset 0 1px 0 rgba(255,255,255,0.04),
    inset 0 -1px 0 rgba(0,0,0,0.1);
  text-align: left;
}

.preset-menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  -webkit-app-region: no-drag;
}

.preset-menu-item svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  color: var(--text-muted);
  transition: color 0.15s ease;
}

.preset-menu-item:hover {
  background: var(--bg-hover);
}

.preset-menu-item.danger {
  color: var(--danger);
}

.preset-menu-item.danger svg {
  color: var(--danger);
}

.preset-menu-item.danger:hover {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
}

.preset-menu-divider {
  height: 1px;
  background: color-mix(in srgb, var(--border-color) 45%, transparent);
  margin: 6px 4px;
}

/* ---- Tile "Add color" ---- */
.color-preset-add {
  border-style: dashed;
  background: transparent;
}

.color-preset-add:hover {
  background: color-mix(in srgb, var(--accent-primary) 6%, transparent);
}

.add-preview {
  display: flex;
  align-items: center;
  justify-content: center;
  border-style: dashed;
  color: var(--text-muted);
  background: var(--bg-secondary);
}

.color-preset-add:hover .add-preview {
  color: var(--accent-primary);
  border-color: var(--accent-primary);
}

.add-preview svg {
  width: 16px;
  height: 16px;
}

/* ---- Panel para agregar / editar un color, se abre debajo de la grilla ---- */
.preset-editor-panel {
  margin-top: 16px;
  padding: 16px;
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  border: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.03), 0 4px 16px rgba(0,0,0,0.12);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.preset-editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: 0.01em;
}

.preset-editor-close {
  width: 24px;
  height: 24px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s ease, color 0.15s ease;
  -webkit-app-region: no-drag;
}

.preset-editor-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.preset-editor-close svg {
  width: 14px;
  height: 14px;
}

.preset-editor-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.preset-editor-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.preset-editor-field-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  font-weight: 700;
}

.preset-editor-name-input {
  padding: 10px 12px;
  border-radius: var(--radius-md);
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.preset-editor-name-input:focus {
  outline: none;
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent-primary) 25%, transparent);
}

.preset-editor-error {
  margin: 0;
  font-size: 12px;
  color: var(--danger);
  font-weight: 600;
}

.preset-editor-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
}

.btn-preset-editor-cancel {
  padding: 10px 16px;
  border-radius: var(--radius-md);
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
  -webkit-app-region: no-drag;
}

.btn-preset-editor-cancel:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn-save-preset {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 18px;
  border-radius: var(--radius-md);
  border: 1px solid color-mix(in srgb, var(--accent-secondary) 20%, transparent);
  background: var(--accent-secondary);
  color: white;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.2s ease, border-color 0.2s ease;
  font-family: inherit;
  letter-spacing: 0.02em;
  -webkit-app-region: no-drag;
}

.btn-save-preset:hover {
  background: color-mix(in srgb, var(--accent-secondary) 85%, black);
  border-color: color-mix(in srgb, var(--accent-secondary) 30%, transparent);
}

.btn-save-preset:active {
  background: color-mix(in srgb, var(--accent-secondary) 80%, black);
  border-color: color-mix(in srgb, var(--accent-secondary) 25%, transparent);
}

.btn-save-preset svg {
  width: 16px;
  height: 16px;
}

/* ---- Transiciones ---- */
.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s cubic-bezier(0.22, 1, 0.36, 1), transform 0.15s cubic-bezier(0.22, 1, 0.36, 1);
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.editor-expand-enter-active,
.editor-expand-leave-active {
  transition: opacity 0.2s cubic-bezier(0.22, 1, 0.36, 1), transform 0.2s cubic-bezier(0.22, 1, 0.36, 1);
}

.editor-expand-enter-from,
.editor-expand-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

/* ========== RESOLUTION ========== */
.resolution-row {
  display: flex;
  align-items: flex-end;
  gap: 12px;
}

.resolution-field {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.resolution-field-label {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-dim);
}

.resolution-input {
  width: 100%;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 14px;
  font-family: var(--font-mono);
  outline: none;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  box-sizing: border-box;
  -moz-appearance: textfield;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05), inset 0 1px 0 rgba(255,255,255,0.04);
  will-change: border-color, box-shadow, transform;
}

.resolution-input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow), 0 2px 6px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.08);
  transform: translateY(-2px);
}

.resolution-input::-webkit-outer-spin-button,
.resolution-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.resolution-input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow), inset 0 1px 0 rgba(255,255,255,0.08);
}

.resolution-x {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-muted);
  padding-bottom: 10px;
  user-select: none;
}

.resolution-presets {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 12px;
}

/* ========== JAVA SECTION ========== */
.java-section {
  display: flex;
  flex-direction: column;
  gap: 28px;
}

.java-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.java-dropdown-trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 10px;
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  background: linear-gradient(180deg, var(--bg-input) 0%, color-mix(in srgb, var(--bg-secondary) 50%, var(--bg-input)) 100%);
  color: var(--text-primary);
  font-family: inherit;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  text-align: left;
  -webkit-app-region: no-drag;
  will-change: border-color, box-shadow, transform;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05), inset 0 1px 0 rgba(255,255,255,0.04);
}

.java-dropdown-trigger:hover {
  border-color: color-mix(in srgb, var(--border-hover) 70%, transparent);
  transform: translateY(-2px);
  box-shadow: 
    0 4px 10px rgba(0,0,0,0.08),
    inset 0 1px 0 rgba(255,255,255,0.04);
}

.java-dropdown-trigger.open {
  border-color: var(--accent-primary);
  box-shadow: 
    0 0 0 3px var(--accent-glow),
    inset 0 1px 0 rgba(255,255,255,0.08);
  transform: translateY(-2px);
}

.java-dropdown-trigger.loading {
  cursor: default;
  opacity: 0.85;
}

.java-trigger-skeleton {
  color: var(--text-dim);
  font-weight: 500;
}

.java-trigger-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.java-trigger-top {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.java-trigger-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.java-trigger-vendor {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 2px 8px;
  border-radius: 100px;
  background: color-mix(in srgb, var(--accent-primary) 16%, transparent);
  color: var(--accent-display);
  flex-shrink: 0;
  box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

.java-trigger-meta {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: var(--font-mono);
}

.java-chevron {
  width: 16px;
  height: 16px;
  color: var(--text-muted);
  flex-shrink: 0;
  transition: transform 0.15s ease, color 0.15s ease;
  will-change: transform, color;
}

.java-chevron.open {
  transform: rotate(180deg);
  color: var(--accent-primary);
}

.java-dropdown-menu {
  /* position/top/left/right/z-index ahora los aplica DropdownMenu.vue (su
     .dropdown-menu-panel.full-width); la animación de entrada/salida
     también la maneja su <transition name="dropdown">, acá solo queda
     lo visual. */
  background: linear-gradient(180deg, var(--bg-card) 0%, var(--bg-secondary) 100%);
  border: 1px solid color-mix(in srgb, var(--border-color) 55%, transparent);
  border-radius: 12px;
  padding: 6px;
  box-shadow: 
    0 6px 20px rgba(0,0,0,0.2),
    inset 0 1px 0 rgba(255,255,255,0.04),
    inset 0 -1px 0 rgba(0,0,0,0.1);
  max-height: 320px;
  overflow-y: auto;
  contain: content;
  text-align: left;
}

.java-dropdown-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  border: 1.5px solid transparent;
  -webkit-app-region: no-drag;
  will-change: background, border-color, transform;
  box-shadow: 0 1px 2px rgba(0,0,0,0.03);
  /* Coherencia con el modal */
  background: transparent;
}

.java-dropdown-item:hover {
  background: var(--bg-hover);
  transform: translateY(-2px);
  box-shadow: 0 2px 6px rgba(0,0,0,0.05);
}

.java-dropdown-item.active {
  background: var(--accent-glow);
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-primary) 20%, transparent);
  transform: translateY(-1px);
}

.java-dropdown-item.browse {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 10px;
  color: var(--accent-primary);
  font-weight: 700;
}

.java-dropdown-item.browse:hover {
  background: color-mix(in srgb, var(--accent-glow) 30%, var(--bg-hover));
  box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent-primary) 20%, transparent);
}

.java-dropdown-divider {
  height: 1px;
  background: color-mix(in srgb, var(--border-color) 45%, transparent);
  margin: 6px 8px;
  box-shadow: 0 1px 0 rgba(0,0,0,0.05);
  border-radius: 1px;
}

.java-item-header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  width: 100%;
}

.java-item-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-muted);
  flex-shrink: 0;
  margin-top: 2px;
  border: 1px solid color-mix(in srgb, var(--border-color) 30%, transparent);
}

.java-dropdown-item.active .java-item-icon {
  color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-primary) 15%, var(--bg-secondary));
  border-color: color-mix(in srgb, var(--accent-primary) 30%, transparent);
}

.java-item-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  flex: 1;
}

.java-item-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

.java-item-desc {
  font-size: 11px;
  color: var(--text-muted);
  line-height: 1.4;
}

.java-item-path-text {
  font-size: 10px;
  color: var(--text-dim);
  font-family: var(--font-mono);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}

.java-item-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 3px 8px;
  border-radius: 100px;
  flex-shrink: 0;
  margin-top: 2px;
}

.java-item-badge.bundled {
  background: color-mix(in srgb, var(--success) 16%, transparent);
  color: var(--success);
}

.java-item-badge.missing {
  background: color-mix(in srgb, var(--warning) 16%, transparent);
  color: var(--warning);
}

.java-error {
  margin-top: 8px;
  padding: 10px 14px;
  background: var(--danger-dim);
  border: 1px solid color-mix(in srgb, var(--danger) 28%, transparent);
  border-radius: 10px;
  color: var(--danger);
  font-size: 13px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 8px;
  box-shadow: 
    0 2px 8px rgba(0,0,0,0.08),
    inset 0 1px 0 rgba(255,255,255,0.03);
}

.java-error svg {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  opacity: 0.85;
}

.memory-field {
  margin-top: 4px;
}

.memory-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 12px;
  gap: 12px;
}

.memory-value-block {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  flex-shrink: 0;
}

.memory-gb {
  font-size: 22px;
  font-weight: 800;
  color: var(--accent-primary);
  line-height: 1;
  letter-spacing: -0.5px;
  text-shadow: 0 1px 2px rgba(0,0,0,0.1);
}

.memory-mb {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.ram-slider-area {
  position: relative;
  margin-top: 8px;
  margin-bottom: 8px;
}

.ram-track-bg {
  position: relative;
  width: 100%;
  height: 8px;
  background: color-mix(in srgb, var(--border-color) 45%, transparent);
  border-radius: 4px;
  overflow: visible;
  box-shadow: 
    inset 0 1px 0 rgba(255,255,255,0.04),
    inset 0 -1px 0 rgba(0,0,0,0.08);
}

.ram-fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: linear-gradient(90deg, var(--accent-primary), var(--accent-secondary));
  border-radius: 4px;
  transition: width 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  pointer-events: none;
  will-change: width;
}

.ram-ticks {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.ram-tick {
  position: absolute;
  top: 0;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.tick-line {
  width: 2px;
  height: 8px;
  background: color-mix(in srgb, var(--border-hover) 70%, transparent);
  border-radius: 1px;
  transition: background 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: background;
}

.ram-tick.active .tick-line {
  background: var(--accent-primary);
}

.tick-label {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-dim);
  font-family: var(--font-sans);
  transition: color 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: color;
}

.ram-tick.active .tick-label {
  color: var(--accent-primary);
}

.ram-slider {
  position: absolute;
  top: -5px;
  left: 0;
  width: 100%;
  height: 18px;
  -webkit-appearance: none;
  appearance: none;
  background: transparent;
  outline: none;
  cursor: pointer;
  margin: 0;
  z-index: 2;
}

.ram-slider::-webkit-slider-runnable-track {
  width: 100%;
  height: 8px;
  cursor: pointer;
  background: transparent;
  border: none;
}

.ram-slider::-moz-range-track {
  width: 100%;
  height: 8px;
  cursor: pointer;
  background: transparent;
  border: none;
}

.ram-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  margin-top: -4px;
  border-radius: 50%;
  background: var(--accent-primary);
  border: none;
  box-shadow: 
    0 0 0 2px var(--bg-card),
    0 0 0 4px var(--accent-glow),
    0 2px 6px rgba(0,0,0,0.25);
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
  will-change: transform, box-shadow;
}

.ram-slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
  box-shadow: 
    0 0 0 2px var(--bg-card),
    0 0 0 6px var(--accent-glow),
    0 4px 10px rgba(0,0,0,0.2);
}

.ram-slider::-webkit-slider-thumb:active {
  transform: scale(1.05) translateY(0);
}

.ram-slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  margin-top: -4px;
  border-radius: 50%;
  background: var(--accent-primary);
  border: none;
  box-shadow: 
    0 0 0 2px var(--bg-card),
    0 0 0 4px var(--accent-glow),
    0 2px 6px rgba(0,0,0,0.25);
  cursor: pointer;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.ram-slider::-moz-range-thumb:hover {
  transform: scale(1.15);
  box-shadow: 
    0 0 0 2px var(--bg-card),
    0 0 0 6px var(--accent-glow),
    0 4px 10px rgba(0,0,0,0.2);
}

.ram-slider::-moz-range-thumb:active {
  transform: scale(1.05) translateY(0);
}

/* ========== JAVA ARGUMENTS ========== */
.args-field {
  margin-top: 4px;
}

.args-textarea-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.args-textarea {
  box-sizing: border-box;
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  background: var(--bg-input);
  color: var(--text-primary);
  font-size: 13px;
  font-family: var(--font-mono);
  line-height: 1.6;
  resize: vertical;
  min-height: 80px;
  outline: none;
  transition: all 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  box-shadow: 0 1px 2px rgba(0,0,0,0.05), inset 0 1px 0 rgba(255,255,255,0.04);
  will-change: border-color, box-shadow, transform;
}

.args-textarea:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow), 0 2px 6px rgba(0,0,0,0.08), inset 0 1px 0 rgba(255,255,255,0.08);
  transform: translateY(-2px);
}

.args-textarea::placeholder {
  color: var(--text-dim);
}

.args-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-dim);
  font-weight: 500;
}

.args-hint svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  color: var(--text-muted);
  transition: color 0.2s cubic-bezier(0.22, 1, 0.36, 1);
  will-change: color;
}

@keyframes spin {
  to { transform: rotate(360deg); will-change: transform; }
}

.shake-enter-active,
.shake-leave-active {
  transition: all 0.3s cubic-bezier(0.22, 1, 0.36, 1);
}

.shake-enter-from,
.shake-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}

@media (max-width: 640px) {
  .modal-overlay {
    background: rgba(0, 0, 0, 0.55);
  }

  .settings-modal {
    width: 100vw;
    height: 100vh;
    max-width: 100%;
    max-height: 100%;
    border-radius: 0;
    background: var(--bg-primary);
    border: none;
    animation: none;
  }

  .settings-sidebar {
    width: 64px;
    min-width: 64px;
    padding: 16px 8px;
    background: var(--bg-secondary);
    border-right: 1px solid color-mix(in srgb, var(--border-color) 45%, transparent);
  }

  .settings-sidebar-header h3,
  .settings-nav-item span {
    display: none;
  }

  .settings-nav-item {
    justify-content: center;
    padding: 10px;
  }

  .settings-nav-item svg {
    width: 18px;
    height: 18px;
  }

  .settings-nav-item.active::before {
    display: none;
  }

  .settings-content-body {
    padding: 20px;
  }

  .color-presets-grid {
    grid-template-columns: repeat(3, 1fr);
  }

  .theme-picker {
    grid-template-columns: repeat(2, 1fr);
  }

  .memory-header {
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }

  .memory-value-block {
    align-items: flex-start;
  }
}

@media (max-width: 480px) {
  .theme-picker {
    grid-template-columns: 1fr;
  }

  .color-presets-grid {
    grid-template-columns: repeat(2, 1fr);
  }

  .color-preset {
    padding: 10px 8px;
  }

  .color-preset.custom .color-preset-main {
    padding: 10px 8px;
  }
}
</style>