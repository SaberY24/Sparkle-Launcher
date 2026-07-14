import { ref } from "vue";

/**
 * Composable para manejar el patrón inFlight (evitar llamadas concurrentes).
 * 
 * Unifica el patrón de:
 * - let xxxxInFlight = false;
 * - if (xxxxInFlight) return;
 * - xxxxInFlight = true;
 * - finally { xxxxInFlight = false; }
 * 
 * @returns Objeto con ref inFlight y función wrap para envolver async functions
 * 
 * @example
 * ```vue
 * <script setup>
 * const { inFlight, wrap } = useInFlight();
 * 
 * async function loadData() {
 *   await wrap(async () => {
 *     // Tu código aquí
 *   });
 * }
 * </script>
 * ```
 */
export function useInFlight() {
  const inFlight = ref(false);

  /**
   * Envuelve una función async para manejar inFlight automáticamente.
   */
  const wrap = async <T>(fn: () => Promise<T>): Promise<T | undefined> => {
    if (inFlight.value) return undefined;
    inFlight.value = true;
    try {
      return await fn();
    } finally {
      inFlight.value = false;
    }
  };

  /**
   * Verifica si está inFlight.
   */
  const isInFlight = (): boolean => inFlight.value;

  /**
   * Resetear el estado inFlight.
   */
  const reset = (): void => {
    inFlight.value = false;
  };

  return { inFlight, wrap, isInFlight, reset };
}

/**
 * Versión que devuelve solo la función wrap para casos simples.
 */
export function useSimpleInFlight() {
  const { wrap } = useInFlight();
  return { wrap };
}
