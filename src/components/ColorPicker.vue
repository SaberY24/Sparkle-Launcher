<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";

const props = defineProps<{ modelValue: string }>();
const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

function hexToRgb(hex: string) {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? { r: parseInt(result[1], 16), g: parseInt(result[2], 16), b: parseInt(result[3], 16) }
    : { r: 192, g: 132, b: 252 };
}

function rgbToHex(r: number, g: number, b: number) {
  const toHex = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, "0");
  return "#" + toHex(r) + toHex(g) + toHex(b);
}

function rgbToHsv(r: number, g: number, b: number) {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const d = max - min;
  let h = 0;
  const s = max === 0 ? 0 : d / max;
  const v = max;

  if (d !== 0) {
    switch (max) {
      case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
      case g: h = ((b - r) / d + 2) / 6; break;
      case b: h = ((r - g) / d + 4) / 6; break;
    }
  }
  return { h: h * 360, s: s * 100, v: v * 100 };
}

function hsvToRgb(h: number, s: number, v: number) {
  h = (h % 360) / 360;
  s = Math.max(0, Math.min(100, s)) / 100;
  v = Math.max(0, Math.min(100, v)) / 100;

  const i = Math.floor(h * 6);
  const f = h * 6 - i;
  const p = v * (1 - s);
  const q = v * (1 - f * s);
  const t = v * (1 - (1 - f) * s);

  let r = 0, g = 0, b = 0;
  switch (i % 6) {
    case 0: r = v; g = t; b = p; break;
    case 1: r = q; g = v; b = p; break;
    case 2: r = p; g = v; b = t; break;
    case 3: r = p; g = q; b = v; break;
    case 4: r = t; g = p; b = v; break;
    case 5: r = v; g = p; b = q; break;
  }
  return { r: Math.round(r * 255), g: Math.round(g * 255), b: Math.round(b * 255) };
}

const hue = ref(270);
const saturation = ref(100);
const value = ref(75);

const hexInput = ref("#000000");
const rgbR = ref(0);
const rgbG = ref(0);
const rgbB = ref(0);

const gradientArea = ref<HTMLDivElement | null>(null);
const hueSlider = ref<HTMLDivElement | null>(null);
const isDraggingGradient = ref(false);
const isDraggingHue = ref(false);

const currentColor = computed(() => {
  const rgb = hsvToRgb(hue.value, saturation.value, value.value);
  return rgbToHex(rgb.r, rgb.g, rgb.b);
});

const gradientPointerStyle = computed(() => ({
  left: `${saturation.value}%`,
  top: `${100 - value.value}%`,
}));

const huePointerStyle = computed(() => ({
  left: `${(hue.value / 360) * 100}%`,
}));

function updateFromHsv() {
  const rgb = hsvToRgb(hue.value, saturation.value, value.value);
  hexInput.value = rgbToHex(rgb.r, rgb.g, rgb.b);
  rgbR.value = rgb.r;
  rgbG.value = rgb.g;
  rgbB.value = rgb.b;
  emit("update:modelValue", hexInput.value);
}

function updateFromHex(hex: string) {
  const rgb = hexToRgb(hex);
  const hsv = rgbToHsv(rgb.r, rgb.g, rgb.b);
  hue.value = hsv.h;
  saturation.value = hsv.s;
  value.value = hsv.v;
  rgbR.value = rgb.r;
  rgbG.value = rgb.g;
  rgbB.value = rgb.b;
  hexInput.value = hex;
  emit("update:modelValue", hex);
}

function updateFromRgb() {
  const hex = rgbToHex(rgbR.value, rgbG.value, rgbB.value);
  const hsv = rgbToHsv(rgbR.value, rgbG.value, rgbB.value);
  hue.value = hsv.h;
  saturation.value = hsv.s;
  value.value = hsv.v;
  hexInput.value = hex;
  emit("update:modelValue", hex);
}

function onGradientMouseDown(e: MouseEvent) {
  isDraggingGradient.value = true;
  updateGradientPosition(e);
}

function onHueMouseDown(e: MouseEvent) {
  isDraggingHue.value = true;
  updateHuePosition(e);
}

function updateGradientPosition(e: MouseEvent) {
  if (!gradientArea.value) return;
  const rect = gradientArea.value.getBoundingClientRect();
  const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  const y = Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height));
  saturation.value = Math.round(x * 100);
  value.value = Math.round((1 - y) * 100);
  updateFromHsv();
}

function updateHuePosition(e: MouseEvent) {
  if (!hueSlider.value) return;
  const rect = hueSlider.value.getBoundingClientRect();
  const x = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  hue.value = Math.round(x * 360);
  updateFromHsv();
}

function onMouseMove(e: MouseEvent) {
  if (isDraggingGradient.value) updateGradientPosition(e);
  if (isDraggingHue.value) updateHuePosition(e);
}

function onMouseUp() {
  isDraggingGradient.value = false;
  isDraggingHue.value = false;
}

function onHexInput(e: Event) {
  const target = e.target as HTMLInputElement;
  let val = target.value.trim();
  if (!val.startsWith("#")) val = "#" + val;
  if (/^#[a-f\d]{6}$/i.test(val)) {
    updateFromHex(val.toLowerCase());
  }
}

function onRgbInput() {
  rgbR.value = Math.max(0, Math.min(255, Math.round(rgbR.value || 0)));
  rgbG.value = Math.max(0, Math.min(255, Math.round(rgbG.value || 0)));
  rgbB.value = Math.max(0, Math.min(255, Math.round(rgbB.value || 0)));
  updateFromRgb();
}

watch(() => props.modelValue, (newVal) => {
  if (newVal && newVal !== currentColor.value) {
    updateFromHex(newVal);
  }
}, { immediate: true });

onMounted(() => {
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
});

onUnmounted(() => {
  document.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseup", onMouseUp);
});
</script>

<template>
  <div class="color-picker" role="application" aria-label="Color picker">
    <div
      ref="gradientArea"
      class="gradient-area"
      :style="{ backgroundColor: `hsl(${hue}, 100%, 50%)` }"
      @mousedown="onGradientMouseDown"
      aria-label="Color saturation and brightness"
    >
      <div class="gradient-white"></div>
      <div class="gradient-black"></div>
      <div class="gradient-pointer" :style="gradientPointerStyle" aria-hidden="true">
        <div class="pointer-ring"></div>
      </div>
    </div>

    <div
      ref="hueSlider"
      class="hue-slider"
      @mousedown="onHueMouseDown"
      aria-label="Hue selector"
    >
      <div class="hue-track"></div>
      <div class="hue-pointer" :style="huePointerStyle" aria-hidden="true">
        <div class="pointer-ring"></div>
      </div>
    </div>

    <div class="picker-inputs">
      <div class="preview-circle" :style="{ background: currentColor }" aria-hidden="true"></div>
      
      <div class="input-group hex-group">
        <label>HEX</label>
        <input
          type="text"
          :value="hexInput"
          @input="onHexInput"
          maxlength="7"
          spellcheck="false"
          aria-label="Hex color value"
        />
      </div>

      <div class="rgb-group">
        <div class="input-group">
          <label>R</label>
          <input
            type="number"
            v-model.number="rgbR"
            min="0"
            max="255"
            @change="onRgbInput"
            aria-label="Red value"
          />
        </div>
        <div class="input-group">
          <label>G</label>
          <input
            type="number"
            v-model.number="rgbG"
            min="0"
            max="255"
            @change="onRgbInput"
            aria-label="Green value"
          />
        </div>
        <div class="input-group">
          <label>B</label>
          <input
            type="number"
            v-model.number="rgbB"
            min="0"
            max="255"
            @change="onRgbInput"
            aria-label="Blue value"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.color-picker {
  display: flex;
  flex-direction: column;
  gap: 12px;
  user-select: none;
  touch-action: none;
}

.gradient-area {
  position: relative;
  width: 100%;
  height: 180px;
  border-radius: 12px;
  overflow: hidden;
  cursor: crosshair;
  box-shadow: inset 0 0 0 1px var(--border-color);
}

.gradient-white {
  position: absolute;
  inset: 0;
  background: linear-gradient(to right, #fff, rgba(255,255,255,0));
}

.gradient-black {
  position: absolute;
  inset: 0;
  background: linear-gradient(to top, #000, rgba(0,0,0,0));
}

.gradient-pointer {
  position: absolute;
  width: 16px;
  height: 16px;
  transform: translate(-50%, -50%);
  pointer-events: none;
}

.pointer-ring {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 2px solid #fff;
  box-shadow: 0 0 0 1px rgba(0,0,0,0.3), 0 2px 6px rgba(0,0,0,0.3);
}

.hue-slider {
  position: relative;
  width: 100%;
  height: 16px;
  border-radius: 8px;
  cursor: pointer;
  overflow: hidden;
  box-shadow: inset 0 0 0 1px var(--border-color);
}

.hue-track {
  width: 100%;
  height: 100%;
  background: linear-gradient(
    to right,
    #ff0000 0%, #ffff00 17%, #00ff00 33%, 
    #00ffff 50%, #0000ff 67%, #ff00ff 83%, #ff0000 100%
  );
}

.hue-pointer {
  position: absolute;
  top: 0;
  width: 16px;
  height: 100%;
  transform: translateX(-50%);
  pointer-events: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.hue-pointer .pointer-ring {
  width: 14px;
  height: 14px;
}

.picker-inputs {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  padding: 12px;
  background: var(--bg-input);
  border-radius: 12px;
  border: 1px solid var(--border-color);
}

.preview-circle {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  border: 2px solid var(--border-color);
  flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15);
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.input-group label {
  font-size: 10px;
  font-weight: 700;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.input-group input {
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  font-family: var(--font-mono);
  font-weight: 600;
  outline: none;
  text-align: center;
  transition: border-color 0.15s, box-shadow 0.15s;
}

.input-group input:focus {
  border-color: var(--accent-primary);
  box-shadow: 0 0 0 3px var(--accent-glow);
}

.hex-group {
  flex: 1;
  min-width: 0;
}

.hex-group input {
  text-align: left;
  text-transform: uppercase;
}

.rgb-group {
  display: flex;
  gap: 8px;
}

.rgb-group .input-group input {
  width: 52px;
  padding: 8px 6px;
}

input[type="number"]::-webkit-inner-spin-button,
input[type="number"]::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
input[type="number"] {
  -moz-appearance: textfield;
}

@media (max-width: 500px) {
  .picker-inputs {
    flex-wrap: wrap;
  }
  .rgb-group {
    width: 100%;
    justify-content: space-between;
  }
}
</style>