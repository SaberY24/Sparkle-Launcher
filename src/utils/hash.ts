/**
 * Funciones de hash para caching de computed properties.
 * Usado para detectar cambios en arrays y evitar recálculos redundantes.
 */

/**
 * Crea una función de hash genérica para arrays.
 * Por defecto, usa el largo del array y las longitudes de las propiedades 'name' 
 * de los elementos first, mid y last.
 * 
 * @param propWeights - Array de objetos que definen qué propiedades usar y su peso
 * @returns Función de hash
 */
export function createListHash<T extends Record<string, any>>(
  propWeights: { prop: keyof T; multiplier: number }[] = []
): (list: T[]) => number {
  return (list: T[]): number => {
    if (list.length === 0) return 0;
    
    const first = list[0];
    const mid = list[Math.floor(list.length / 2)];
    const last = list[list.length - 1];
    
    let hash = list.length;
    
    // Añadir pesos basados en propiedades
    for (const { prop, multiplier } of propWeights) {
      hash += ((first?.[prop]?.length || 0) * multiplier) +
              ((mid?.[prop]?.length || 0) * multiplier * 10) +
              ((last?.[prop]?.length || 0) * multiplier * 100);
    }
    
    // Añadir pesos basados en valores booleanos (enabled, etc.)
    if (first && 'enabled' in first) {
      hash += (first.enabled ? 1 : 0) * 1000 +
              (mid?.enabled ? 1 : 0) * 10000 +
              (last?.enabled ? 1 : 0) * 100000;
    }
    
    // Añadir pesos basados en valores numéricos (major, etc.)
    if (first && 'major' in first) {
      hash += (first.major || 0) * 1000;
    }
    
    return hash;
  };
}

/**
 * Función de hash específica para ModInfo/PackItem con enabled y name.
 */
export function hashEnabledList<T extends { enabled?: boolean; name: string }>(
  list: T[]
): number {
  if (list.length === 0) return 0;
  const first = list[0];
  const mid = list[Math.floor(list.length / 2)];
  const last = list[list.length - 1];
  return list.length +
         (first?.enabled ? 1 : 0) * 1000 +
         (mid?.enabled ? 1 : 0) * 10000 +
         (last?.enabled ? 1 : 0) * 100000 +
         (first?.name.length || 0) +
         (mid?.name.length || 0) * 10 +
         (last?.name.length || 0) * 100;
}

/**
 * Función de hash para arrays simples (solo largo y nombres).
 */
export function hashSimpleList<T extends { name: string }>(
  list: T[]
): number {
  if (list.length === 0) return 0;
  const first = list[0];
  const mid = list[Math.floor(list.length / 2)];
  const last = list[list.length - 1];
  return list.length +
         (first?.name.length || 0) +
         (mid?.name.length || 0) * 10 +
         (last?.name.length || 0) * 100;
}

/**
 * Función de hash para Java installations.
 */
export function hashJavaList(
  list: { path: string; version: string; major: number; vendor: string }[]
): number {
  if (list.length === 0) return 0;
  const first = list[0];
  const mid = list[Math.floor(list.length / 2)];
  const last = list[list.length - 1];
  return list.length +
         (first?.path.length || 0) +
         (mid?.path.length || 0) * 10 +
         (last?.path.length || 0) * 100 +
         (first?.major || 0) * 1000;
}
