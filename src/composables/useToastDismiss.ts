import { computed } from "vue";
import { useLauncherStore } from "../stores/launcher";

// Module-scope: PlayTab schedules a dismiss after install/play actions,
// and cancels/reads the SAME pending timer, so this can't be two
// independent copies.
let toastDismissTimer: number | null = null;

export function useToastDismiss() {
  const launcher = useLauncherStore();

  const toastVariant = computed<"good" | "bad" | "info">(() => {
    const status = launcher.launchStatus || "";
    if (!status) return "info";
    if (status.startsWith("Couldn't") || status.includes("something looks off")) return "bad";
    if (
      status.includes("ready to go") ||
      status.includes("ready to play") ||
      status.includes("updated to") ||
      status.includes("Cache cleared")
    ) {
      return "good";
    }
    return "info";
  });

  function scheduleToastDismiss(delay = 3000) {
    if (toastDismissTimer) window.clearTimeout(toastDismissTimer);
    toastDismissTimer = window.setTimeout(() => {
      launcher.launchStatus = "";
      launcher.launchProgress = 0;
      launcher.installProgress = null;
      toastDismissTimer = null;
    }, delay);
  }

  function dismissToast() {
    if (toastDismissTimer) {
      window.clearTimeout(toastDismissTimer);
      toastDismissTimer = null;
    }
    launcher.launchStatus = "";
    launcher.launchProgress = 0;
    launcher.installProgress = null;
  }

  function disposeToastTimer() {
    if (toastDismissTimer) {
      window.clearTimeout(toastDismissTimer);
      toastDismissTimer = null;
    }
  }

  return { toastVariant, scheduleToastDismiss, dismissToast, disposeToastTimer };
}