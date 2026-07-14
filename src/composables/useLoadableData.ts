import { ref, type Ref } from "vue";

/**
 * Estado de carga.
 */
export interface LoadableDataState<T> {
  data: Ref<T>;
  loading: Ref<boolean>;
  error: Ref<string>;
  refreshing: Ref<boolean>;
  loadedOnce: Ref<boolean>;
}

/**
 * Resultado de las funciones de carga.
 */
export interface LoadableDataActions<Args extends any[] = any[]> {
  load: (...args: Args) => Promise<void>;
  refresh: (...args: Args) => Promise<void>;
  reset: () => void;
}

/**
 * Opciones para useLoadableData.
 */
export interface LoadableDataOptions<T, Args extends any[] = any[]> {
  /** Valor inicial */
  initialValue?: T;
  /** Si true, carga automáticamente al montar */
  autoLoad?: boolean;
  /** Función para cargar datos */
  loadFn: (...args: Args) => Promise<T>;
  /** Mensaje de error por defecto */
  defaultErrorMessage?: string;
}

/**
 * Composable para manejar carga de datos con estados (loading, error, refreshing).
 * 
 * Unifica el patrón repetido de:
 * - Variable de datos
 * - Flag loading
 * - Flag error
 * - Flag refreshing
 * - Flag inFlight para evitar llamadas concurrentes
 * 
 * @example
 * ```vue
 * <script setup>
 * const { data: mods, loading: modsLoading, error: modsError, load: loadMods } = 
 *   useLoadableData({
 *     initialValue: [],
 *     autoLoad: true,
 *     loadFn: async () => await invoke<ModInfo[]>("list_installed_mods", { progress: channel })
 *   });
 * </script>
 * ```
 */
export function useLoadableData<T, Args extends any[] = any[]>(
  options: LoadableDataOptions<T, Args>
): LoadableDataState<T> & LoadableDataActions<Args> & { _setupAutoLoad: () => void } {
  const {
    initialValue,
    autoLoad = false,
    loadFn,
    defaultErrorMessage = "Failed to load data"
  } = options;

  const data = ref<T>(initialValue!) as Ref<T>;
  const loading = ref(false);
  const error = ref("");
  const refreshing = ref(false);
  const loadedOnce = ref(false);
  let inFlight = false;

  const doLoad = async (...args: Args): Promise<void> => {
    // Evitar llamadas concurrentes
    if (inFlight) return;
    inFlight = true;

    loading.value = true;
    error.value = "";

    try {
      data.value = await loadFn(...args);
      loadedOnce.value = true;
    } catch (e: any) {
      error.value = e?.message || defaultErrorMessage;
      console.error("LoadableData error:", e);
    } finally {
      loading.value = false;
      inFlight = false;
    }
  };

  const load = async (...args: Args): Promise<void> => {
    await doLoad(...args);
  };

  const refresh = async (...args: Args): Promise<void> => {
    // Evitar refresh si ya está cargando o refrescando
    if (loading.value || refreshing.value) return;
    refreshing.value = true;
    try {
      await doLoad(...args);
    } finally {
      refreshing.value = false;
    }
  };

  const reset = (): void => {
    data.value = initialValue!;
    loading.value = false;
    error.value = "";
    refreshing.value = false;
    loadedOnce.value = false;
    inFlight = false;
  };

  // Auto-load al montar si está configurado
  // Note: onMounted no está disponible aquí, debe ser llamado desde el componente
  const setupAutoLoad = () => {
    if (autoLoad) {
      // autoLoad solo tiene sentido para loadFn sin argumentos obligatorios;
      // si Args exige parámetros, es responsabilidad de quien llama no usar autoLoad.
      load(...([] as unknown as Args));
    }
  };

  return {
    data,
    loading,
    error,
    refreshing,
    loadedOnce,
    load,
    refresh,
    reset,
    // Exponemos setupAutoLoad para que el componente pueda llamarlo en onMounted
    _setupAutoLoad: setupAutoLoad
  };
}

/**
 * Versión simplificada para arrays con loading/refreshing.
 * Útil para listas que se cargan y refrescan.
 */
export function useLoadableList<T, Args extends any[] = any[]>(
  options: Omit<LoadableDataOptions<T[], Args>, 'initialValue'> & { initialValue?: T[] }
): LoadableDataState<T[]> & LoadableDataActions<Args> & { _setupAutoLoad: () => void } {
  return useLoadableData<T[], Args>({
    initialValue: [],
    ...options
  });
}