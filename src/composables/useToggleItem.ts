import { type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Opciones para useToggleItem.
 */
export interface ToggleItemOptions<T> {
  /** Ref al array de items */
  items: Ref<T[]>;
  /** Ref al mensaje de error */
  error: Ref<string>;
  /** Comando a invocar en el backend */
  invokeCommand: string;
  /** Nombre de la propiedad que contiene el fileName */
  fileNameProp?: keyof T;
  /** Nombre de la propiedad que contiene el nombre para mostrar en errores */
  nameProp?: keyof T;
  /** Función personalizada para obtener el fileName */
  getFileName?: (item: T) => string;
  /** Función personalizada para obtener el nombre */
  getName?: (item: T) => string;
  /** Parámetros adicionales para el comando invoke */
  invokeParams?: Record<string, any>;
}

/**
 * Item genérico con propiedades comunes.
 */
export interface TogglableItem {
  enabled: boolean;
  fileName: string;
  name: string;
}

/**
 * Composable para manejar toggle de items (enable/disable).
 * 
 * Unifica la lógica repetida de:
 * - Cambiar estado local
 * - Intentar invocar backend
 * - Revertir si falla
 * - Mostrar error
 * 
 * @example
 * ```vue
 * <script setup>
 * const mods = ref<ModInfo[]>([]);
 * const modToggleError = ref("");
 * 
 * const toggleMod = useToggleItem({
 *   items: mods,
 *   error: modToggleError,
 *   invokeCommand: "set_mod_enabled"
 * });
 * </script>
 * 
 * <template>
 * <ToggleSwitch 
 *   :model-value="mod.enabled" 
 *   @update:model-value="() => toggleMod(mod)" 
 * />
 * </template>
 * ```
 */
export function useToggleItem<T extends { enabled: boolean }>(
  options: ToggleItemOptions<T>
) {
  const {
    items,
    error,
    invokeCommand,
    fileNameProp = 'fileName' as keyof T,
    nameProp = 'name' as keyof T,
    getFileName,
    getName,
    // Parámetros adicionales para el comando invoke
    invokeParams = {}
  } = options;

  const toggle = async (item: T): Promise<void> => {
    const nextEnabled = !item.enabled;
    const previous = item.enabled;

    // Actualizar estado local
    (item as any).enabled = nextEnabled;
    items.value = [...items.value];
    error.value = "";

    const fileName = getFileName ? getFileName(item) : (item as any)[fileNameProp] as string;
    const name = getName ? getName(item) : (item as any)[nameProp] as string;

    try {
      await invoke(invokeCommand, { fileName, enabled: nextEnabled, ...invokeParams });
    } catch (e: any) {
      // Revertir cambios
      (item as any).enabled = previous;
      items.value = [...items.value];
      error.value = e?.message || `Could not ${nextEnabled ? "enable" : "disable"} "${name}".`;
    }
  };

  return toggle;
}



/**
 * Versión especializada para ModInfo.
 */
export function useToggleMod(
  mods: Ref<{ fileName: string; name: string; enabled: boolean }[]>, 
  error: Ref<string>
) {
  return useToggleItem({
    items: mods,
    error,
    invokeCommand: "set_mod_enabled",
    fileNameProp: 'fileName',
    nameProp: 'name'
  });
}

/**
 * Versión especializada para PackItem (shaders, resource packs).
 */
export function useTogglePackItem(
  items: Ref<{ fileName: string; name: string; enabled: boolean }[]>, 
  error: Ref<string>,
  kind: "shaderpacks" | "resourcepacks"
) {
  return useToggleItem({
    items,
    error,
    invokeCommand: "set_content_item_enabled",
    fileNameProp: 'fileName',
    nameProp: 'name',
    getFileName: (item) => item.fileName,
    getName: (item) => item.name,
    invokeParams: { kind }
  });
}

/**
 * Factory para crear funciones de toggle con kind específico.
 */
export function createTogglePackItem(
  kind: "shaderpacks" | "resourcepacks"
) {
  return function (
    items: Ref<{ fileName: string; name: string; enabled: boolean }[]>, 
    error: Ref<string>
  ) {
    return useToggleItem({
      items,
      error,
      invokeCommand: "set_content_item_enabled",
      fileNameProp: 'fileName',
      nameProp: 'name',
      invokeParams: { kind }
    });
  };
}