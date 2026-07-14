import { type Ref } from "vue";

/**
 * Composable para manejar limpieza de búsqueda.
 * 
 * Unifica las funciones clearModSearch, clearShaderSearch, clearResourcePackSearch.
 * 
 * @param searchRef - Ref al string de búsqueda
 * @param debouncedRef - Ref al string de búsqueda debounced
 * @returns Función para limpiar la búsqueda
 * 
 * @example
 * ```vue
 * <script setup>
 * const modSearch = ref("");
 * const debouncedModSearch = useDebouncedValue(modSearch);
 * 
 * const clearModSearch = useClearSearch(modSearch, debouncedModSearch);
 * </script>
 * ```
 */
export function useClearSearch(
  searchRef: Ref<string>,
  debouncedRef: Ref<string>
): () => void {
  return () => {
    searchRef.value = "";
    debouncedRef.value = "";
  };
}

/**
 * Versión que solo requiere el searchRef (para cuando debouncedRef es computed).
 */
export function useClearSearchSimple(
  searchRef: Ref<string>
): () => void {
  return () => {
    searchRef.value = "";
  };
}
