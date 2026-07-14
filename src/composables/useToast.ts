import { ref } from "vue";

export type ToastVariant = "good" | "bad" | "info";

/**
 * Extracted from HomeView.vue's inline toast logic (toastDismissTimer /
 * scheduleToastDismiss / dismissToast). Behavior is identical, just reusable.
 *
 * HomeView keeps deciding WHEN to show a toast (it still writes to
 * `launcher.launchStatus`), this composable just owns the show/hide + timer
 * plumbing so the same pattern can be reused by other views without
 * duplicating the timer bookkeeping.
 */
export function useToast(defaultDelay = 3000) {
  const message = ref("");
  const variant = ref<ToastVariant>("info");
  let dismissTimer: number | null = null;

  const title = ref<string | null>(null); // null = derive from variant in the component

  function show(text: string, toastVariant: ToastVariant = "info", delay = defaultDelay) {
    message.value = text;
    variant.value = toastVariant;
    scheduleDismiss(delay);
  }

  function scheduleDismiss(delay = defaultDelay) {
    if (dismissTimer) window.clearTimeout(dismissTimer);
    dismissTimer = window.setTimeout(() => {
      message.value = "";
      dismissTimer = null;
    }, delay);
  }

  function dismiss() {
    if (dismissTimer) {
      window.clearTimeout(dismissTimer);
      dismissTimer = null;
    }
    message.value = "";
  }

  function dispose() {
    if (dismissTimer) {
      window.clearTimeout(dismissTimer);
      dismissTimer = null;
    }
  }

  return { message, variant, title, show, scheduleDismiss, dismiss, dispose };
}