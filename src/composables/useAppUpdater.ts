import { ref } from "vue";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus = "idle" | "checking" | "up-to-date" | "available" | "downloading" | "ready" | "error" | "declined";

/**
 * Wraps @tauri-apps/plugin-updater so both HomeView's <UpdateNotification>
 * banner and SettingsModal's "Check for updates" button can drive/observe
 * the same update flow.
 *
 * Singleton on purpose: state is declared at module scope so every caller
 * of useAppUpdater() shares it, instead of each component getting its own
 * isolated copy (which would let you e.g. click "Check for updates" in
 * Settings and never see anything happen in HomeView).
 *
 * Requires (see integration notes):
 *  - tauri-plugin-updater registered in src-tauri/src/lib.rs
 *  - "updater" + "process" plugins added to Cargo.toml
 *  - `updater.endpoints` + `updater.pubkey` set in tauri.conf.json
 *  - `updater:default` and `process:allow-restart` permissions in your capabilities file
 */
const status = ref<UpdateStatus>("idle");
const version = ref("");
const notes = ref("");
const progress = ref(0);
const error = ref("");
const lastCheckedAt = ref<number | null>(null);

let pendingUpdate: Update | null = null;
let contentLength = 0;
let downloaded = 0;

async function checkForUpdate(opts: { silent?: boolean } = {}) {
  // `silent` = background/startup check: don't show "up to date" noise,
  // just quietly surface a banner if there IS an update.
  if (!opts.silent) status.value = "checking";
  try {
    const update = await check();
    lastCheckedAt.value = Date.now();
    if (update) {
      pendingUpdate = update;
      version.value = update.version;
      notes.value = update.body ?? "";
      status.value = "available";
    } else {
      pendingUpdate = null;
      status.value = opts.silent ? "idle" : "up-to-date";
    }
  } catch (e: any) {
    console.error("Update check failed:", e);
    if (!opts.silent) {
      error.value = e?.message || e?.toString() || "Couldn't check for updates.";
      status.value = "error";
    }
  }
}

async function downloadAndInstall() {
  if (!pendingUpdate) return;
  status.value = "downloading";
  progress.value = 0;
  downloaded = 0;
  contentLength = 0;

  try {
    await pendingUpdate.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          contentLength = event.data.contentLength ?? 0;
          break;
        case "Progress":
          downloaded += event.data.chunkLength;
          if (contentLength > 0) {
            progress.value = Math.min(100, (downloaded / contentLength) * 100);
          }
          break;
        case "Finished":
          progress.value = 100;
          break;
      }
    });
    status.value = "ready";
  } catch (e: any) {
    error.value = e?.message || e?.toString() || "Download failed.";
    status.value = "error";
  }
}

async function restartNow() {
  await relaunch();
}

function decline() {
  status.value = "declined";
  pendingUpdate = null;
}

function dismiss() {
  status.value = "idle";
}

export function useAppUpdater() {
  return { status, version, notes, progress, error, lastCheckedAt, checkForUpdate, downloadAndInstall, restartNow, decline, dismiss };
}