/**
 * Opciones para useOptimizedScroll.
 */
export interface OptimizedScrollOptions {
  /** Intervalo de frames para sincronizar scrollTop (default: 3) */
  frameSyncInterval?: number;
  /** Tiempo de settle en ms (default: 150) */
  settleDelay?: number;
  /** Umbral mínimo de cambio de scroll para procesar (default: 0.5) */
  minScrollDelta?: number;
}

/**
 * Resultado de useOptimizedScroll.
 */
export interface OptimizedScrollResult {
  /** Handler de scroll a adjuntar al elemento */
  onScroll: (e: Event) => void;
  /** Resetear el estado interno */
  reset: () => void;
}

/**
 * Composable para manejar scroll de forma optimizada.
 * 
 * Incluye:
 * - Throttling por diferencia mínima de scroll
 * - Bloqueo de pointer-events durante scroll
 * - Sincronización de scrollTop cada N frames
 * - Settle timeout para limpieza
 * 
 * @example
 * ```vue
 * <script setup>
 * const { onScroll } = useOptimizedScroll();
 * </script>
 * 
 * <template>
 * <div @scroll="onScroll">
 *   <!-- content -->
 * </div>
 * </template>
 * ```
 */
export function useOptimizedScroll(
  options: OptimizedScrollOptions = {}
): OptimizedScrollResult {
  const {
    frameSyncInterval = 3,
    settleDelay = 150,
    minScrollDelta = 0.5
  } = options;

  // Estado interno
  let lastProcessedScrollTop = 0;
  let ticking = false;
  let frameCounter = 0;
  let scrollSettleTimer: number | null = null;

  const reset = (): void => {
    lastProcessedScrollTop = 0;
    ticking = false;
    frameCounter = 0;
    if (scrollSettleTimer) {
      window.clearTimeout(scrollSettleTimer);
      scrollSettleTimer = null;
    }
  };

  const onScroll = (e: Event): void => {
    const target = e.target as HTMLElement;
    const currentScrollTop = target.scrollTop;

    // Saltar si el scroll no ha cambiado lo suficiente
    if (Math.abs(currentScrollTop - lastProcessedScrollTop) < minScrollDelta) {
      return;
    }
    lastProcessedScrollTop = currentScrollTop;

    // Bloquear pointer-events directamente por DOM
    target.classList.add("is-scrolling");
    target.style.pointerEvents = "none";

    if (!ticking) {
      window.requestAnimationFrame(() => {
        frameCounter++;

        // Solo sincronizar scrollTop cada frameSyncInterval frames
        if (frameCounter >= frameSyncInterval) {
          frameCounter = 0;
        }

        ticking = false;
        target.style.pointerEvents = "";
      });
      ticking = true;
    }

    if (scrollSettleTimer) window.clearTimeout(scrollSettleTimer);
    scrollSettleTimer = window.setTimeout(() => {
      target.classList.remove("is-scrolling");
      frameCounter = 0;
    }, settleDelay);
  };

  return { onScroll, reset };
}

/**
 * Versión para logs (con intervalo de frames más largo para mejor rendimiento).
 */
export function useLogScroll(
  options: Partial<OptimizedScrollOptions> = {}
): OptimizedScrollResult {
  return useOptimizedScroll({
    frameSyncInterval: 3,
    settleDelay: 150,
    minScrollDelta: 0.5,
    ...options
  });
}

/**
 * Versión para changelog.
 */
export function useChangelogScroll(
  options: Partial<OptimizedScrollOptions> = {}
): OptimizedScrollResult {
  return useOptimizedScroll({
    frameSyncInterval: 3,
    settleDelay: 150,
    minScrollDelta: 0.5,
    ...options
  });
}
