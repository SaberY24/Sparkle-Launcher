import { ref, computed, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useCopyToClipboard } from "./useCopyToClipboard";

export const ACTIVE_LOG_ID = "__active__";

// Module-scope (singleton) state: declared outside the exported function so
// it survives LogsTab.vue being unmounted/remounted when the user switches
// tabs (HomeView still uses v-if per tab), exactly like it did when this
// state lived directly in HomeView's long-lived script.
const minecraftLog = ref("");
const minecraftLogRef = ref<HTMLElement | null>(null);
const logPollInterval = ref<number | null>(null);
const selectedLogFile = ref(ACTIVE_LOG_ID);
const availableLogFiles = ref<string[]>(["latest.log"]);

const { copied: copySuccess, copy: baseCopyLogContent } = useCopyToClipboard();

const logLines = computed(() => minecraftLog.value.split("\n"));

// Limit rendered lines to avoid overloading the DOM on huge logs.
const MAX_VISIBLE_LOG_LINES = 10000;
const visibleLogLines = computed(() => {
  const allLines = logLines.value;
  if (allLines.length <= MAX_VISIBLE_LOG_LINES) return allLines;
  return allLines.slice(-MAX_VISIBLE_LOG_LINES);
});

const selectedLogLabel = computed(() =>
  selectedLogFile.value === ACTIVE_LOG_ID ? "Active Log" : selectedLogFile.value
);

function logLineLevel(line: string): "error" | "warn" | "" {
  if (/\/(ERROR|FATAL|SEVERE)\]|\b(ERROR|FATAL|SEVERE)\b|Exception|Caused by:/i.test(line)) {
    return "error";
  }
  if (/\/(WARN|WARNING)\]|\bWARN(ING)?\b/i.test(line)) {
    return "warn";
  }
  return "";
}

function isScrolledNearBottom(el: HTMLElement): boolean {
  const threshold = 60;
  const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
  return distanceFromBottom <= threshold;
}

async function loadLogFiles() {
  try {
    const files = await invoke<string[]>("list_log_files");
    availableLogFiles.value = files.length > 0 ? files : ["latest.log"];
    if (selectedLogFile.value !== ACTIVE_LOG_ID && !availableLogFiles.value.includes(selectedLogFile.value)) {
      selectedLogFile.value = availableLogFiles.value[0];
    }
  } catch (e: any) {
    console.error("Failed to list log files:", e);
  }
}

/**
 * `isGameRunning` lives in HomeView (shared with the Play tab's
 * play/stop flow), so it's passed in rather than owned here. This returns
 * an updater the caller uses to keep it in sync from the polling loop.
 */
export function useMinecraftLogs(getIsGameRunning: () => boolean) {
  async function fetchMinecraftLog() {
    if (selectedLogFile.value === ACTIVE_LOG_ID && !getIsGameRunning()) {
      minecraftLog.value = "Start the game to view the log.";
      return;
    }

    const filename = selectedLogFile.value === ACTIVE_LOG_ID ? "latest.log" : selectedLogFile.value;

    try {
      const text = await invoke<string>("read_log_file", { filename });
      const wasNearBottom = minecraftLogRef.value ? isScrolledNearBottom(minecraftLogRef.value) : true;

      minecraftLog.value = text;

      nextTick(() => {
        if (minecraftLogRef.value && wasNearBottom) {
          minecraftLogRef.value.scrollTop = minecraftLogRef.value.scrollHeight;
        }
      });
    } catch (e: any) {
      minecraftLog.value = "Failed to load log: " + (e?.message || String(e));
    }
  }

  const copyLogContent = () => baseCopyLogContent(minecraftLog.value);

  function selectLogFile(file: string) {
    selectedLogFile.value = file;
    fetchMinecraftLog();
  }

  function startPolling(syncGameRunning: () => Promise<void>) {
    if (logPollInterval.value) return;
    syncGameRunning().then(fetchMinecraftLog);
    logPollInterval.value = window.setInterval(() => {
      syncGameRunning().then(fetchMinecraftLog);
    }, 2000);
  }

  function stopPolling() {
    if (logPollInterval.value) {
      clearInterval(logPollInterval.value);
      logPollInterval.value = null;
    }
  }

  return {
    minecraftLog,
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
  };
}