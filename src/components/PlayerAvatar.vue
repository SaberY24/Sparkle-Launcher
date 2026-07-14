<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { useSkinRenderer, getMcHeadsUrl } from "../composables/useSkinRenderer";

const props = defineProps<{
  skinUrl: string | null;
  textureKey?: string | null;
  uuid?: string | null;
  username: string;
  size?: "xs" | "sm" | "md" | "lg";
  isPremium?: boolean;
}>();

const { headUrl, isLoading, renderHead } = useSkinRenderer();
const hasError = ref(false);
const fallbackUrl = ref<string | null>(null);

const sizeClass = computed(() => props.size || "md");

const steveUrl = "https://mc-heads.net/avatar/Steve/64";

const displayUrl = computed(() => {
  if (!props.isPremium && !props.skinUrl) {
    return steveUrl;
  }
  if (headUrl.value && !hasError.value) return headUrl.value;
  if (fallbackUrl.value && !hasError.value) return fallbackUrl.value;
  if (!props.isPremium) return steveUrl;
  return null;
});

const initial = computed(() => {
  const char = props.username?.charAt(0) || "?";
  return char.toUpperCase();
});

const fallbackGradient = computed(() => {
  const hash = props.username.split("").reduce((acc, char) => {
    return char.charCodeAt(0) + ((acc << 5) - acc);
  }, 0);
  const h = Math.abs(hash % 360);
  return `linear-gradient(135deg, hsl(${h}, 70%, 60%), hsl(${(h + 40) % 360}, 70%, 50%))`;
});

watch(
  () => props.skinUrl,
  async (newUrl) => {
    hasError.value = false;
    fallbackUrl.value = null;
    if (newUrl) {
      try {
        await renderHead(newUrl, props.textureKey || undefined);
      } catch {
        if (props.uuid && props.isPremium) {
          fallbackUrl.value = getMcHeadsUrl(props.uuid, 64);
        }
      }
    }
  },
  { immediate: true }
);

function onError() {
  hasError.value = true;
  if (props.uuid && props.isPremium && !fallbackUrl.value) {
    fallbackUrl.value = getMcHeadsUrl(props.uuid, 64);
  }
}
</script>

<template>
  <div 
    class="avatar-container" 
    :class="[sizeClass, { loading: isLoading }]"
    :style="!displayUrl ? { background: fallbackGradient } : undefined"
    :aria-label="`Avatar for ${username}`"
  >
    <img
      v-if="displayUrl && !hasError"
      :src="displayUrl"
      :alt="username"
      class="skin-image"
      @error="onError"
      loading="eager"
      decoding="async"
      draggable="false"
    />
    <span v-else class="fallback">{{ initial }}</span>
    
    <div v-if="isLoading" class="loading-overlay" aria-hidden="true">
      <div class="spinner"></div>
    </div>
  </div>
</template>

<style scoped>
.avatar-container {
  position: relative;
  border-radius: 12px;
  overflow: hidden;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 0 20px var(--accent-glow);
  transition: all 0.2s ease;
  aspect-ratio: 1;
}

.avatar-container.xs { width: 28px; height: 28px; border-radius: 8px; }
.avatar-container.sm { width: 36px; height: 36px; }
.avatar-container.md { width: 48px; height: 48px; }
.avatar-container.lg { width: 64px; height: 64px; }

.avatar-container.loading {
  opacity: 0.8;
}

.skin-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
  image-rendering: pixelated;
  image-rendering: crisp-edges;
  display: block;
}

.avatar-container.xs .fallback {
  font-size: 14px;
  font-weight: 800;
  color: #0c0c14;
  line-height: 1;
  user-select: none;
}

.avatar-container.sm .fallback,
.avatar-container.md .fallback,
.avatar-container.lg .fallback {
  font-weight: 800;
  color: #0c0c14;
  line-height: 1;
  font-size: 55%;
  user-select: none;
}

.loading-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.2);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}
</style>