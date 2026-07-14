import { type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

/**
 * Opciones para useShowInFolder.
 */
export interface ShowInFolderOptions<T> {
  /** Ref al mensaje de error */
  error: Ref<string>;
  /** Comando a invocar en el backend */
  invokeCommand: string;
  /** Función para obtener el fileName del item */
  getFileName?: (item: T) => string;
  /** Función para obtener el nombre del item para el mensaje de error */
  getName?: (item: T) => string;
  /** Nombre de la propiedad fileName (default: 'fileName') */
  fileNameProp?: keyof T;
  /** Nombre de la propiedad name (default: 'name') */
  nameProp?: keyof T;
  /** Parámetros adicionales para el comando invoke */
  invokeParams?: Record<string, any>;
}

/**
 * Composable para crear funciones "show in folder" genéricas.
 * 
 * Unifica la lógica repetida de:
 * - showModInFolder
 * - showShaderInFolder
 * - showResourcePackInFolder
 * 
 * @example
 * ```vue
 * <script setup>
 * const modToggleError = ref("");
 * 
 * const showModInFolder = useShowInFolder({
 *   error: modToggleError,
 *   invokeCommand: "show_mod_in_folder"
 * });
 * </script>
 * ```
 */
export function useShowInFolder<T extends { fileName: string; name: string }>(
  options: ShowInFolderOptions<T>
) {
  const {
    error,
    invokeCommand,
    getFileName,
    getName,
    fileNameProp = 'fileName' as keyof T,
    nameProp = 'name' as keyof T,
    invokeParams
  } = options;

  const showInFolder = async (item: T): Promise<void> => {
    error.value = "";
    
    const fileName = getFileName ? getFileName(item) : (item as any)[fileNameProp] as string;
    const name = getName ? getName(item) : (item as any)[nameProp] as string;

    try {
      await invoke(invokeCommand, { fileName, ...invokeParams });
    } catch (e: any) {
      error.value = e?.message || `Could not show "${name}" in the file explorer.`;
    }
  };

  return showInFolder;
}

/**
 * Versión especializada para ModInfo.
 */
export function useShowModInFolder(
  error: Ref<string>
) {
  return useShowInFolder({
    error,
    invokeCommand: "show_mod_in_folder",
    fileNameProp: 'fileName',
    nameProp: 'name'
  });
}

/**
 * Factory para crear funciones showInFolder para PackItem con kind específico.
 */
export function createShowInFolderForKind(
  kind: "shaderpacks" | "resourcepacks"
) {
  return function (error: Ref<string>) {
    return useShowInFolder({
      error,
      invokeCommand: "show_content_item_in_folder",
      fileNameProp: 'fileName',
      nameProp: 'name',
      invokeParams: { kind }
    });
  };
}

/**
 * Versión especializada para shaders.
 */
export function useShowShaderInFolder(
  error: Ref<string>
) {
  return useShowInFolder({
    error,
    invokeCommand: "show_content_item_in_folder",
    fileNameProp: 'fileName',
    nameProp: 'name'
  });
}

/**
 * Versión especializada para resource packs.
 */
export function useShowResourcePackInFolder(
  error: Ref<string>
) {
  return useShowInFolder({
    error,
    invokeCommand: "show_content_item_in_folder",
    fileNameProp: 'fileName',
    nameProp: 'name'
  });
}