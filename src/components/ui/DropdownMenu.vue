<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";

// Envoltorio genérico para dropdowns: junta la mecánica que antes vivía
// duplicada en el dropdown de logs, el menú ⋮ de colores, y (con matices)
// AccountDropdown — abrir/cerrar, cerrar al clickear afuera (con
// composedPath, inmune a que el propio clic remueva el elemento del DOM),
// cerrar con Escape, y opcionalmente posicionamiento "fixed" calculado con
// getBoundingClientRect + teleport a <body> para no depender del
// overflow/margen de un contenedor con scroll.
//
// Uso básico (inline, como el dropdown de logs):
//   <DropdownMenu>
//     <template #trigger="{ toggle, isOpen }">
//       <button @click="toggle">...</button>
//     </template>
//     <template #default="{ close }">
//       <button @click="selectThing(); close()">Item</button>
//     </template>
//   </DropdownMenu>
//
// Uso con posicionamiento dinámico (como el menú ⋮ de colores):
//   <DropdownMenu use-fixed menu-width="180" placement="top-right">...

const props = withDefaults(
  defineProps<{
    /** Si es true, el menú se teletransporta a <body> con position:fixed y
     * se posiciona dinámicamente contra el trigger — para casos dentro de
     * contenedores con overflow/scroll que recortarían un dropdown normal. */
    useFixed?: boolean;
    /** Ancho estimado del menú en modo fixed, usado para no salirse de pantalla. */
    menuWidth?: number;
    /** Alto estimado del menú en modo fixed, usado para decidir si abre hacia arriba. */
    menuHeight?: number;
    /** Alineación del menú contra el trigger (ambos modos). */
    align?: "left" | "right" | "center";
    /** El trigger y el menú ocupan el 100% del ancho del contenedor padre
     * (ej. el selector de Java), en vez de ajustarse al contenido. */
    fullWidth?: boolean;
  }>(),
  { useFixed: false, menuWidth: 220, menuHeight: 260, align: "left", fullWidth: false }
);

const isOpen = ref(false);
const triggerRef = ref<HTMLElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const fixedStyle = ref<Record<string, string>>({});

function getTriggerEl(): HTMLElement | null {
  // Medimos el elemento que realmente se ve (el primer hijo renderizado por
  // el slot #trigger), no el wrapper. Si el trigger usa position:absolute
  // para su propio posicionamiento (ej. el botón ⋮ de un preset, anclado a
  // la esquina de su tarjeta), el wrapper (inline-flex) queda con tamaño
  // 0x0 porque su único hijo está fuera del flujo — medir el wrapper daría
  // una posición completamente equivocada para el menú.
  return (triggerRef.value?.firstElementChild as HTMLElement | null) ?? triggerRef.value;
}

async function computeFixedPosition() {
  await nextTick();

  const trigger = getTriggerEl();
  const menu = menuRef.value;

  if (!trigger || !menu) return;

  const rect = trigger.getBoundingClientRect();

  const gap = 6;
  const margin = 8;

  const menuRect = menu.getBoundingClientRect();
  const menuHeight = menuRect.height || props.menuHeight;
  const menuWidth = menuRect.width || props.menuWidth;

  const spaceBelow = window.innerHeight - rect.bottom;
  const spaceAbove = rect.top;

  const style: Record<string, string> = {
    position: "fixed",
    zIndex: "9999",
    minWidth: `${props.menuWidth}px`,
  };

  // Horizontal: alineamos el borde correspondiente del menú con el mismo
  // borde del trigger, según `align` (por defecto "right", como el menú ⋮
  // de colores; AccountDropdown en modo sidebar usa "left" para calzar con
  // su comportamiento original).
  let left: number;
  if (props.align === "left") {
    left = rect.left;
  } else if (props.align === "center") {
    left = rect.left + rect.width / 2 - menuWidth / 2;
  } else {
    left = rect.right - menuWidth;
  }

  left = Math.max(margin, left);
  left = Math.min(left, window.innerWidth - menuWidth - margin);

  style.left = `${left}px`;

  // Vertical
  if (spaceBelow >= menuHeight + gap || spaceBelow >= spaceAbove) {
    // Abrir abajo
    style.top = `${rect.bottom + gap}px`;
  } else {
    // Abrir arriba
    style.top = `${rect.top - menuHeight - gap}px`;
  }

  fixedStyle.value = style;
}   

async function toggle() {
  if (isOpen.value) {
    close();
    return;
  }

  isOpen.value = true;

  if (props.useFixed) {
    await computeFixedPosition();
  }
}

async function open() {
  if (isOpen.value) return;
  await toggle();
}

function close() {
  isOpen.value = false;
}

function onDocumentClick(e: MouseEvent) {
  if (!isOpen.value) return;
  const path = e.composedPath();
  if (triggerRef.value && path.includes(triggerRef.value)) return;
  if (menuRef.value && path.includes(menuRef.value)) return;
  close();
}

function onKeydown(e: KeyboardEvent) {
  if (e.key !== "Escape" || !isOpen.value) return;
  close();
  // Si un componente padre (ej. un modal) también escucha Escape a nivel de
  // document, esto evita que la MISMA pulsación cierre ambas cosas a la vez:
  // Escape debe cerrar primero el dropdown, y recién en la siguiente
  // pulsación cerrar lo que esté detrás. stopPropagation() no alcanza acá
  // porque ambos listeners están en el mismo nodo (document); hace falta
  // stopImmediatePropagation(). Esto funciona porque Vue monta los hijos
  // antes que el padre, así que este listener siempre queda registrado
  // antes que el del padre.
  e.stopImmediatePropagation();
}

function onResize() {
  if (isOpen.value && props.useFixed) computeFixedPosition();
}

onMounted(() => {
  document.addEventListener("click", onDocumentClick);
  document.addEventListener("keydown", onKeydown);
  window.addEventListener("resize", onResize);
  document.addEventListener("scroll", () => props.useFixed && close(), true);
});

onUnmounted(() => {
  document.removeEventListener("click", onDocumentClick);
  document.removeEventListener("keydown", onKeydown);
  window.removeEventListener("resize", onResize);
});

defineExpose({ close, open, toggle, isOpen });
</script>

<template>
  <div class="dropdown-menu-wrapper" :class="{ 'is-inline': !useFixed, 'full-width': fullWidth }" ref="triggerRef">
    <slot name="trigger" :toggle="toggle" :is-open="isOpen" />

    <template v-if="!useFixed">
      <transition name="dropdown">
        <div v-if="isOpen" ref="menuRef" class="dropdown-menu-panel" :class="[align, { 'full-width': fullWidth }]">
          <slot :close="close" />
        </div>
      </transition>
    </template>

    <teleport to="body" v-else>
      <transition name="dropdown">
        <div v-if="isOpen" ref="menuRef" class="dropdown-menu-panel fixed" :style="fixedStyle">
          <slot :close="close" />
        </div>
      </transition>
    </teleport>
  </div>
</template>

<style scoped>
.dropdown-menu-wrapper {
  display: inline-flex;
}

.dropdown-menu-wrapper.is-inline {
  position: relative;
}

/* Modo fixed: el wrapper no necesita generar caja propia (el menú se
   posiciona globalmente vía fixed+teleport), así no deja un hueco fantasma
   en el flujo cuando su único hijo (el trigger) usa position:absolute. */
.dropdown-menu-wrapper:not(.is-inline) {
  display: contents;
}

.dropdown-menu-wrapper.full-width {
  display: block;
  width: 100%;
}

.dropdown-menu-panel {
  position: absolute;
  top: calc(100% + 6px);
  z-index: 60;
}

.dropdown-menu-panel.left {
  left: 0;
}

.dropdown-menu-panel.right {
  right: 0;
}

.dropdown-menu-panel.center {
  left: 50%;
  transform: translateX(-50%);
}

.dropdown-menu-panel.full-width {
  left: 0;
  right: 0;
}

.dropdown-menu-panel.fixed {
  position: fixed;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s cubic-bezier(0.22, 1, 0.36, 1), transform 0.15s cubic-bezier(0.22, 1, 0.36, 1);
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>