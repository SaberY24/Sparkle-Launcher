import { computed, type Ref } from "vue";
import { hashEnabledList } from "../utils/hash";

/**
 * Tipos de items que pueden ser filtrados.
 * Deben tener al menos 'name' y opcionalmente 'enabled' y 'fileName'.
 */
export interface FilterableItem {
  name: string;
  fileName?: string;
  enabled?: boolean;
}

/**
 * Resultado del filtrado con caching.
 */
export interface FilteredListResult<T> {
  filtered: Ref<T[]>;
  cache: { value: T[] };
  resetCache: () => void;
}

/**
 * Opciones para el filtrado.
 */
export interface FilteredListOptions {
  /** Si true, solo incluye items deshabilitados cuando disabledOnly es true */
  filterByEnabled?: boolean;
  /** Propiedades adicionales para buscar (además de name y fileName) */
  additionalSearchProps?: string[];
}

/**
 * Composable para crear listas filtradas con caching de alto rendimiento.
 * 
 * Unifica la lógica de filtrado que se repetía para mods, shaders y resource packs.
 * 
 * @param source - Ref al array original de items
 * @param searchQuery - Ref al string de búsqueda (ya debounced)
 * @param disabledOnly - Ref a si solo mostrar items deshabilitados
 * @param options - Opciones adicionales
 * @returns Ref computada con los items filtrados
 * 
 * @example
 * ```vue
 * <script setup>
 * const mods = ref<ModInfo[]>([]);
 * const modSearch = ref("");
 * const debouncedModSearch = useDebouncedValue(modSearch);
 * const showDisabledOnly = ref(false);
 * 
 * const filteredMods = useFilteredList(
 *   mods,
 *   debouncedModSearch,
 *   showDisabledOnly,
 *   { additionalSearchProps: ['modId'] }
 * );
 * </script>
 * ```
 */
export function useFilteredList<T extends FilterableItem>(
  source: Ref<T[]>,
  searchQuery: Ref<string>,
  disabledOnly: Ref<boolean>,
  options: FilteredListOptions = {}
): Ref<T[]> {
  const { filterByEnabled = true, additionalSearchProps = [] } = options;

  // Cache para evitar recálculos redundantes
  let filteredCache: T[] = [];
  let lastSourceHash = 0;
  let lastSearchQuery = "";
  let lastDisabledOnly = false;

  const filtered = computed(() => {
    const list = source.value;
    const q = searchQuery.value.toLowerCase();
    const disabledOnlyFlag = disabledOnly.value;

    // Calcular hash de la lista fuente
    const currentHash = hashEnabledList(list);

    // Si nada ha cambiado, devolver cache
    if (
      q === lastSearchQuery &&
      disabledOnlyFlag === lastDisabledOnly &&
      currentHash === lastSourceHash
    ) {
      return filteredCache;
    }

    // Actualizar valores de cache
    lastSourceHash = currentHash;
    lastSearchQuery = q;
    lastDisabledOnly = disabledOnlyFlag;

    let result: T[] = list;

    // Aplicar filtro de búsqueda
    if (q) {
      const filtered: T[] = [];
      for (let i = 0; i < list.length; i++) {
        const item = list[i];
        const nameLower = item.name.toLowerCase();
        const fileNameLower = item.fileName?.toLowerCase() || "";

        // Buscar en propiedades adicionales si se especificaron
        let matchesAdditional = false;
        for (const prop of additionalSearchProps) {
          const propValue = (item as Record<string, any>)[prop];
          if (propValue && typeof propValue === "string") {
            if (propValue.toLowerCase().startsWith(q)) {
              matchesAdditional = true;
              break;
            }
          }
        }

        if (
          nameLower.startsWith(q) ||
          fileNameLower.startsWith(q) ||
          matchesAdditional
        ) {
          filtered.push(item);
        }
      }
      result = filtered;
    }

    // Aplicar filtro de disabled only
    if (disabledOnlyFlag && filterByEnabled) {
      const disabledFiltered: T[] = [];
      // Optimización: filtrar solo si hay resultado previo
      const sourceToFilter = q ? result : list;
      for (let i = 0; i < sourceToFilter.length; i++) {
        if (!sourceToFilter[i].enabled) {
          disabledFiltered.push(sourceToFilter[i]);
        }
      }
      result = disabledFiltered;
    }

    // Guardar en cache
    filteredCache = result;
    return result;
  });

  return filtered;
}

/**
 * Versión especializada para ModInfo que incluye modId en la búsqueda.
 */
export function useFilteredMods<
  T extends { name: string; fileName: string; enabled: boolean; modId: string }
>(
  source: Ref<T[]>,
  searchQuery: Ref<string>,
  disabledOnly: Ref<boolean>
): Ref<T[]> {
  return useFilteredList(source, searchQuery, disabledOnly, { 
    additionalSearchProps: ['modId'] 
  });
}

/**
 * Composable simplificado para filtrado básico sin caching.
 * Útil cuando no se necesita el alto rendimiento del caching.
 */
export function useSimpleFilteredList<T extends FilterableItem>(
  source: Ref<T[]>,
  searchQuery: Ref<string>,
  filterFn?: (item: T, query: string) => boolean
): Ref<T[]> {
  return computed(() => {
    const list = source.value;
    const q = searchQuery.value.toLowerCase();

    if (!q) return list;

    return list.filter(item => {
      if (filterFn) return filterFn(item, q);
      
      const nameLower = item.name.toLowerCase();
      const fileNameLower = item.fileName?.toLowerCase() || "";
      return nameLower.includes(q) || fileNameLower.includes(q);
    });
  });
}