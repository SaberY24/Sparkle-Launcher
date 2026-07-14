import { ref, type Ref } from "vue";

/**
 * Composable simple para togglear un booleano.
 * 
 * Unifica funciones como:
 * - toggleDisabledOnlyFilter
 * - toggleShadersDisabledOnlyFilter
 * - toggleResourcePacksDisabledOnlyFilter
 * 
 * @param initialValue - Valor inicial
 * @returns Ref al booleano y función toggle
 * 
 * @example
 * ```vue
 * <script setup>
 * const { state: showDisabledOnly, toggle } = useToggle(false);
 * 
 * function toggleDisabledOnlyFilter() {
 *   toggle();
 * }
 * </script>
 * 
 * <template>
 * <button @click="toggle">Toggle</button>
 * </template>
 * ```
 */
export function useToggle(initialValue: boolean = false) {
  const state = ref(initialValue);
  
  const toggle = () => {
    state.value = !state.value;
  };
  
  return {
    state,
    toggle
  };
}

/**
 * Versión que acepta una Ref externa.
 * 
 * @param ref - Ref al booleano
 * @returns Función toggle
 * 
 * @example
 * ```vue
 * <script setup>
 * const showDisabledOnly = ref(false);
 * const toggleDisabledOnly = useToggleRef(showDisabledOnly);
 * </script>
 * 
 * <template>
 * <button @click="toggleDisabledOnly">Toggle</button>
 * </template>
 * ```
 */
export function useToggleRef(ref: Ref<boolean>) {
  return () => {
    ref.value = !ref.value;
  };
}
