<script setup lang="ts">
// Toggle compartido entre Settings (native titlebar, etc.) y la lista de
// mods/shaders/resourcepacks (enable/disable). Antes cada uno tenía su
// propia copia casi idéntica del mismo switch (.toggle/.toggle-slider vs
// .toggle-modern/.toggle-track/.toggle-thumb) — mismo comportamiento visual,
// solo con tamaños y easing ligeramente distintos. Unificado acá.
withDefaults(
  defineProps<{
    modelValue: boolean;
    size?: "sm" | "md";
  }>(),
  { size: "md" }
);

defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <label class="toggle-switch" :class="size">
    <input
      type="checkbox"
      :checked="modelValue"
      @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    >
    <span class="toggle-track">
      <span class="toggle-thumb"></span>
    </span>
  </label>
</template>

<style scoped>
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  cursor: pointer;
  flex-shrink: 0;
}

.toggle-switch.sm {
  width: 42px;
  height: 22px;
}

.toggle-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-track {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg-hover) 80%, transparent);
  border-radius: 24px;
  transition: background-color 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), border-color 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  border: 1px solid color-mix(in srgb, var(--border-color) 60%, transparent);
  display: flex;
  align-items: center;
  padding: 0 2px;
  will-change: background-color, border-color;
}

.toggle-thumb {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
  display: block;
  will-change: transform, background-color;
}

.toggle-switch.sm .toggle-thumb {
  width: 16px;
  height: 16px;
}

.toggle-switch input:checked + .toggle-track {
  background: var(--accent-glow);
  border-color: var(--accent-primary);
}

.toggle-switch input:checked + .toggle-track .toggle-thumb {
  transform: translateX(20px);
  background: var(--accent-primary);
  box-shadow: 0 0 12px var(--accent-glow);
}

.toggle-switch.sm input:checked + .toggle-track .toggle-thumb {
  transform: translateX(22px);
}
</style>