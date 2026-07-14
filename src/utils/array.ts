import { computed, type Ref } from "vue";

/**
 * Cuenta elementos deshabilitados en un array.
 * 
 * @param items - Ref al array de items con propiedad enabled
 * @returns Ref computada con el conteo
 * 
 * @example
 * ```vue
 * <script setup>
 * const mods = ref<ModInfo[]>([]);
 * const disabledModsCount = countDisabled(mods);
 * </script>
 * ```
 */
export function countDisabled<T extends { enabled: boolean }>(
  items: Ref<T[]>
): Ref<number> {
  return computed(() => items.value.filter((item) => !item.enabled).length);
}

/**
 * Cuenta elementos habilitados en un array.
 */
export function countEnabled<T extends { enabled: boolean }>(
  items: Ref<T[]>
): Ref<number> {
  return computed(() => items.value.filter((item) => item.enabled).length);
}

/**
 * Verifica si un array está vacío.
 */
export function isEmpty<T>(
  items: Ref<T[]>
): Ref<boolean> {
  return computed(() => items.value.length === 0);
}

/**
 * Obtiene el primer elemento de un array.
 */
export function first<T>(
  items: Ref<T[]>
): Ref<T | undefined> {
  return computed(() => items.value[0]);
}

/**
 * Obtiene el último elemento de un array.
 */
export function last<T>(
  items: Ref<T[]>
): Ref<T | undefined> {
  return computed(() => items.value[items.value.length - 1]);
}

/**
 * Verifica si todos los elementos están habilitados.
 */
export function allEnabled<T extends { enabled: boolean }>(
  items: Ref<T[]>
): Ref<boolean> {
  return computed(() => items.value.every((item) => item.enabled));
}

/**
 * Verifica si alguno está deshabilitado.
 */
export function someDisabled<T extends { enabled: boolean }>(
  items: Ref<T[]>
): Ref<boolean> {
  return computed(() => items.value.some((item) => !item.enabled));
}
