import { computed, type Ref } from "vue";
import { hashJavaList } from "./hash";

/**
 * Tipos para Java installations.
 */
export interface JavaInstallation {
  path: string;
  version: string;
  major: number;
  vendor: string;
}

/**
 * Opciones para seleccionar Java.
 */
export interface JavaSelectorOptions {
  javaInstallations: Ref<JavaInstallation[]>;
  selectedJavaPath: Ref<string>;
  bundledJavaPath: Ref<string>;
}

/**
 * Obtiene el label del Java seleccionado con caching.
 * 
 * @param options - Opciones del selector
 * @returns Ref computada con el label
 */
export function useSelectedJavaLabel(
  options: JavaSelectorOptions
): Ref<string> {
  const { javaInstallations, selectedJavaPath, bundledJavaPath } = options;
  
  // Cache manual para mejor rendimiento
  let lastPath = "";
  let lastBundled = "";
  let lastHash = 0;
  let cachedValue = "";
  
  return computed(() => {
    const currentPath = selectedJavaPath.value;
    const currentBundled = bundledJavaPath.value;
    const currentHash = hashJavaList(javaInstallations.value);
    
    if (currentPath === lastPath && currentBundled === lastBundled && currentHash === lastHash) {
      return cachedValue;
    }
    
    lastPath = currentPath;
    lastBundled = currentBundled;
    lastHash = currentHash;
    
    const isLauncher = currentPath === currentBundled && currentBundled !== "";
    if (isLauncher || !currentPath) {
      cachedValue = "Use Launcher Java";
    } else {
      const j = javaInstallations.value.find(x => x.path === currentPath);
      cachedValue = j ? `${j.vendor} ${j.major}` : "Custom Java";
    }
    return cachedValue;
  });
}

/**
 * Obtiene el meta del Java seleccionado con caching.
 */
export function useSelectedJavaMeta(
  options: JavaSelectorOptions
): Ref<string> {
  const { javaInstallations, selectedJavaPath, bundledJavaPath } = options;
  
  // Cache manual para mejor rendimiento
  let lastPath = "";
  let lastBundled = "";
  let lastHash = 0;
  let cachedValue = "";
  
  return computed(() => {
    const currentPath = selectedJavaPath.value;
    const currentBundled = bundledJavaPath.value;
    const currentHash = hashJavaList(javaInstallations.value);
    
    if (currentPath === lastPath && currentBundled === lastBundled && currentHash === lastHash) {
      return cachedValue;
    }
    
    lastPath = currentPath;
    lastBundled = currentBundled;
    lastHash = currentHash;
    
    const isLauncher = currentPath === currentBundled && currentBundled !== "";
    if (isLauncher || !currentPath) {
      cachedValue = currentBundled ? "Bundled runtime — managed by Sparkle" : "Will be downloaded automatically";
    } else {
      const j = javaInstallations.value.find(x => x.path === currentPath);
      cachedValue = j ? j.path : "Custom Java path";
    }
    return cachedValue;
  });
}

/**
 * Obtiene el vendor del Java seleccionado con caching.
 */
export function useSelectedJavaVendor(
  options: JavaSelectorOptions
): Ref<string> {
  const { javaInstallations, selectedJavaPath, bundledJavaPath } = options;
  
  // Cache manual para mejor rendimiento
  let lastPath = "";
  let lastBundled = "";
  let lastHash = 0;
  let cachedValue = "";
  
  return computed(() => {
    const currentPath = selectedJavaPath.value;
    const currentBundled = bundledJavaPath.value;
    const currentHash = hashJavaList(javaInstallations.value);
    
    if (currentPath === lastPath && currentBundled === lastBundled && currentHash === lastHash) {
      return cachedValue;
    }
    
    lastPath = currentPath;
    lastBundled = currentBundled;
    lastHash = currentHash;
    
    const isLauncher = currentPath === currentBundled && currentBundled !== "";
    if (isLauncher || !currentPath) {
      cachedValue = "Sparkle Bundled";
    } else {
      const j = javaInstallations.value.find(x => x.path === currentPath);
      cachedValue = j?.vendor || "Unknown";
    }
    return cachedValue;
  });
}

/**
 * Verifica si el Java del launcher está seleccionado con caching.
 */
export function useIsLauncherJavaSelected(
  selectedJavaPath: Ref<string>,
  bundledJavaPath: Ref<string>
): Ref<boolean> {
  // Cache manual para mejor rendimiento
  let lastSelected = "";
  let lastBundled = "";
  let cachedValue = false;
  
  return computed(() => {
    const currentSelected = selectedJavaPath.value;
    const currentBundled = bundledJavaPath.value;
    
    if (currentSelected === lastSelected && currentBundled === lastBundled) {
      return cachedValue;
    }
    
    lastSelected = currentSelected;
    lastBundled = currentBundled;
    cachedValue = currentSelected === currentBundled && currentBundled !== "";
    return cachedValue;
  });
}
