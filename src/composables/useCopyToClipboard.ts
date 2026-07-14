import { ref } from "vue";

/**
 * Composable para manejar copy to clipboard con feedback visual.
 * 
 * @param timeoutMs - Tiempo en milisegundos que el estado 'copied' permanece en true
 * @returns Objeto con estado 'copied' y función 'copy'
 * 
 * @example
 * ```vue
 * <script setup>
 * const { copied, copy } = useCopyToClipboard();
 * </script>
 * 
 * <template>
 * <button @click="copy('text to copy')">
 *   <span v-if="copied">Copied!</span>
 *   <span v-else>Copy</span>
 * </button>
 * </template>
 * ```
 */
export function useCopyToClipboard(timeoutMs: number = 1500) {
  const copied = ref(false);
  let timeoutId: number | null = null;

  const copy = async (text: string): Promise<boolean> => {
    // Limpiar timeout anterior
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      timeoutId = null;
    }

    try {
      await navigator.clipboard.writeText(text);
      copied.value = true;
      timeoutId = window.setTimeout(() => {
        copied.value = false;
        timeoutId = null;
      }, timeoutMs);
      return true;
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
      copied.value = false;
      return false;
    }
  };

  return { copied, copy };
}
