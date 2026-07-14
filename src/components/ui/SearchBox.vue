<script setup lang="ts">
// Buscador compartido entre mods/shaders/resourcepacks (antes cada lista
// repetía el mismo icono + input + botón de limpiar). El debounce sigue
// viviendo en el componente padre (vía useDebouncedValue), acá solo se
// unifica la UI del input en sí.
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
  }>(),
  { placeholder: "Search..." }
);

const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

function onInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}

function clear() {
  emit("update:modelValue", "");
}
</script>

<template>
  <div class="content-search">
    <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" width="14" height="14">
      <circle cx="11" cy="11" r="8" />
      <line x1="21" y1="21" x2="16.65" y2="16.65" />
    </svg>
    <input :value="modelValue" @input="onInput" type="text" :placeholder="placeholder" />
    <button v-if="modelValue" class="search-clear" v-tooltip="'Clear search'" @click="clear">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" width="12" height="12">
        <line x1="18" y1="6" x2="6" y2="18" />
        <line x1="6" y1="6" x2="18" y2="18" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.content-search {
  flex: 1;
  min-width: 160px;
  max-width: 280px;
  height: 34px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  border-radius: 10px;
  background: color-mix(in srgb, var(--bg-input) 60%, transparent);
  border: 1px solid color-mix(in srgb, var(--border-color) 50%, transparent);
  color: var(--text-muted);
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.content-search:focus-within {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.content-search input {
  flex: 1;
  min-width: 0;
  height: 100%;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  caret-color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  padding: 0;
}

.content-search input::placeholder {
  color: var(--text-dim);
}

.search-icon {
  flex-shrink: 0;
  opacity: 0.6;
}

.search-clear {
  flex-shrink: 0;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: none;
  background: color-mix(in srgb, var(--bg-hover) 70%, transparent);
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s ease;
  padding: 0;
}

.search-clear:hover {
  background: color-mix(in srgb, var(--danger) 20%, transparent);
  color: var(--danger);
}

@container content-area (max-width: 720px) {
  .content-search {
    max-width: none;
  }
}

@container content-area (max-width: 520px) {
  .content-search {
    max-width: none;
  }
}
</style>