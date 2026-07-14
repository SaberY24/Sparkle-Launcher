import type { Directive, DirectiveBinding } from "vue";

/**
 * v-tooltip="'Texto del tooltip'"
 *
 * Reemplaza el tooltip nativo del navegador (title="...") por uno propio
 * que usa la clase .custom-tooltip ya definida en styles/main.css, así
 * hereda automáticamente colores, blur y radios de la app (modo claro/oscuro).
 *
 * - El texto vacío o undefined desactiva el tooltip (no se agregan listeners).
 * - Se posiciona arriba del elemento por defecto y se voltea abajo si no cabe.
 * - Un solo <div class="custom-tooltip"> se reutiliza para toda la app.
 */

// --- Estado compartido para bloquear tooltips durante el scroll virtual ---
// Lo sellamos para evitar que se añadan propiedades accidentales en otros archivos.
export const uiState = Object.seal({
  isVirtualScrolling: false as boolean,
});

let tooltipEl: HTMLDivElement | null = null;
let showTimeout: number | null = null;
let currentTarget: HTMLElement | null = null;

// --- Detección de hover sintético causado por scroll ---
// Al hacer scroll, el contenido se mueve bajo un cursor que sigue quieto y
// el navegador dispara mouseenter/mouseover igualmente. Antes cada elemento
// registraba su propio listener de scroll (con listas largas, cientos de
// listeners en window por cada scroll). Ahora usamos UN listener global y
// distinguimos "hover real" de "hover sintético" comparando si el puntero
// realmente se movió desde el último scroll, en vez de un timeout fijo
// (un timeout no sirve si las pausas entre "ticks" de rueda/trackpad son
// más largas que el timeout).
let lastMouseX = -1;
let lastMouseY = -1;
let suppressUntilRealMove = false;
let globalListenersInstalled = false;

// Con mouses gaming de alto polling rate (500-1000Hz), un listener de
// mousemove sin throttle puede dispararse cientos de veces por segundo,
// justo el mismo momento en que se hace scroll (p.ej. arrastrando el
// scrollbar). Se limita a como mucho una vez por frame, que es la
// resolucion real que necesitamos para distinguir hover real de sintetico.
let mouseMoveTicking = false;
let pendingMouseX = -1;
let pendingMouseY = -1;

function trackMouseMove(e: MouseEvent) {
  pendingMouseX = e.clientX;
  pendingMouseY = e.clientY;
  if (mouseMoveTicking) return;
  mouseMoveTicking = true;
  requestAnimationFrame(() => {
    mouseMoveTicking = false;
    if (pendingMouseX !== lastMouseX || pendingMouseY !== lastMouseY) {
      lastMouseX = pendingMouseX;
      lastMouseY = pendingMouseY;
      suppressUntilRealMove = false;
    }
  });
}

function handleGlobalScroll() {
  suppressUntilRealMove = true;
  if (showTimeout) {
    window.clearTimeout(showTimeout);
    showTimeout = null;
  }
  if (currentTarget) hide(currentTarget);
}

function ensureGlobalListeners() {
  if (globalListenersInstalled) return;
  window.addEventListener("scroll", handleGlobalScroll, { capture: true, passive: true });
  window.addEventListener("mousemove", trackMouseMove, { passive: true });
  globalListenersInstalled = true;
}

function getTooltipEl(): HTMLDivElement {
  if (!tooltipEl) {
    tooltipEl = document.createElement("div");
    tooltipEl.className = "custom-tooltip";
    tooltipEl.setAttribute("role", "tooltip");
    document.body.appendChild(tooltipEl);
  }
  return tooltipEl;
}

function positionTooltip(target: HTMLElement) {
  const el = getTooltipEl();
  // Leemos primero (layout) y escribimos después (style), evitando Layout Thrashing
  const rect = target.getBoundingClientRect();
  const tipRect = el.getBoundingClientRect();
  const gap = 8;

  let top = rect.top - tipRect.height - gap;
  let placement: "top" | "bottom" = "top";

  if (top < 4) {
    top = rect.bottom + gap;
    placement = "bottom";
  }

  let left = rect.left + rect.width / 2 - tipRect.width / 2;
  left = Math.max(4, Math.min(left, window.innerWidth - tipRect.width - 4));

  el.style.top = `${Math.round(top)}px`;
  el.style.left = `${Math.round(left)}px`;
  el.dataset.placement = placement;
}

function show(target: HTMLElement, text: string) {
  if (!text) return;
  currentTarget = target;
  const el = getTooltipEl();
  el.textContent = text;
  el.style.top = "-9999px";
  el.style.left = "-9999px";
  el.classList.remove("visible");

  // Usamos doble requestAnimationFrame para asegurar que el navegador pinte 
  // el estado inicial (sin .visible) antes de aplicar la clase que dispara la 
  // transición CSS, evitando que aparezca de golpe sin animar.
  requestAnimationFrame(() => {
    if (currentTarget !== target) return;
    positionTooltip(target);
    requestAnimationFrame(() => {
      if (currentTarget === target) {
        el.classList.add("visible");
      }
    });
  });
}

function hide(target: HTMLElement) {
  if (currentTarget !== target) return;
  currentTarget = null;
  if (tooltipEl) tooltipEl.classList.remove("visible");
}

interface TooltipHTMLElement extends HTMLElement {
  __tooltipText?: string;
  __tooltipHandlers?: {
    enter: () => void;
    leave: () => void;
  };
}

function bind(el: TooltipHTMLElement, binding: DirectiveBinding<string | null | undefined>) {
  const text = binding.value ?? "";
  el.__tooltipText = text;

  // Siempre registramos los listeners, pero onEnter verificará si hay texto
  // Esto permite que los tooltips se activen/désactiven dinámicamente

  ensureGlobalListeners();

  const onEnter = () => {
    // 1. Aborto temprano: si el scroll virtual está activo, ni siquiera iniciamos el timeout.
    if (uiState.isVirtualScrolling) return;
    
    // 2. Filtro de hover sintético (scroll de rueda/trackpad)
    if (suppressUntilRealMove) return;
    
    // 3. Si no hay texto, no mostrar tooltip
    if (!el.__tooltipText) return;
    
    if (showTimeout) window.clearTimeout(showTimeout);
    showTimeout = window.setTimeout(() => {
      // 4. Failsafe: el nodo pudo ser reciclado por el virtual scroller
      if (!el.isConnected) return;
      
      // 5. Failsafe: el usuario pudo empezar a scrollear durante los 250ms del timeout
      if (uiState.isVirtualScrolling) return;
      
      if (el.__tooltipText) show(el, el.__tooltipText);
    }, 250);
  };
  
  const onLeave = () => {
    if (showTimeout) {
      window.clearTimeout(showTimeout);
      showTimeout = null;
    }
    hide(el);
  };

  el.addEventListener("mouseenter", onEnter);
  el.addEventListener("mouseleave", onLeave);
  el.addEventListener("click", onLeave);

  el.__tooltipHandlers = { enter: onEnter, leave: onLeave };
}

function unbind(el: TooltipHTMLElement) {
  const handlers = el.__tooltipHandlers;
  if (handlers) {
    el.removeEventListener("mouseenter", handlers.enter);
    el.removeEventListener("mouseleave", handlers.leave);
    el.removeEventListener("click", handlers.leave);
  }
  hide(el);
}

export const tooltip: Directive<TooltipHTMLElement, string | null | undefined> = {
  mounted(el, binding) {
    bind(el, binding);
  },
  updated(el, binding) {
    if (binding.value === binding.oldValue) return;
    unbind(el);
    bind(el, binding);
  },
  unmounted(el) {
    unbind(el);
  },
};

export default tooltip;