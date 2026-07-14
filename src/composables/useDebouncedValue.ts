import { ref, watch, onUnmounted, type Ref } from "vue";

/**
 * Antes cada lista (mods, shaders, resourcepacks) tenía su propia copia de
 * este mismo patrón: un ref, un `let ...Timeout`, y un `watch()` que
 * debounceaba y normalizaba el valor a mano. Unificado acá — y de paso se
 * limpia el timeout al desmontar, cosa que ninguna de las 3 copias hacía.
 */
export function useDebouncedValue(source: Ref<string>, delay = 200) {
  const debounced = ref(source.value.trim().toLowerCase());
  let timeout: number | null = null;

  watch(source, (value) => {
    if (timeout) window.clearTimeout(timeout);
    timeout = window.setTimeout(() => {
      debounced.value = value.trim().toLowerCase();
      timeout = null;
    }, delay);
  });

  onUnmounted(() => {
    if (timeout) window.clearTimeout(timeout);
  });

  return debounced;
}