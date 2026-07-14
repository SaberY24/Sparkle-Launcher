import { watch, type Ref, type WatchSource } from "vue";
/**
 * Composable para manejar guardado automático con debounce.
 * 
 * @param saveFn - Función de guardado a ejecutar
 * @param delay - Tiempo de espera en milisegundos (default: 500)
 * @returns Objeto con función save para disparar manualmente
 * 
 * @example
 * ```vue
 * <script setup>
 * const value = ref("");
 * const { save } = useDebouncedSave(() => {
 *   // Guardar valor
 * }, 500);
 * 
 * watch(value, () => {
 *   save();
 * });
 * </script>
 * ```
 */
export function useDebouncedSave(
  saveFn: () => void | Promise<void>,
  delay: number = 500
) {
  let timeoutId: number | null = null;
  let inFlight = false;

  const save = () => {
    // Limpiar timeout anterior
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      timeoutId = null;
    }

    // Evitar llamadas concurrentes
    if (inFlight) return;
    inFlight = true;

    timeoutId = window.setTimeout(async () => {
      try {
        await saveFn();
      } catch (e) {
        console.error("Debounced save failed:", e);
      } finally {
        inFlight = false;
      }
    }, delay);
  };

  // Función para cancelar el debounce
  const cancel = () => {
    if (timeoutId) {
      window.clearTimeout(timeoutId);
      timeoutId = null;
    }
    inFlight = false;
  };

  return { save, cancel };
}

/**
 * Versión simplificada que observa automáticamente una lista de refs.
 */
export function useAutoDebouncedSave(
  saveFn: () => void | Promise<void>,
  sources: (Ref<any> | WatchSource<any>)[],
  delay: number = 500
) {
  const { save, cancel } = useDebouncedSave(saveFn, delay);

  watch(sources, () => {
    save();
  }, { deep: true });

  return { save, cancel };
}
