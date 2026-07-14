<script setup lang="ts">
// Usa el icono de la app
const iconPath = "/icon.png";
</script>

<template>
  <div class="app-icon" aria-hidden="true">
    <img :src="iconPath" alt="Sparkle App Icon" />
  </div>
</template>

<style scoped>
.app-icon {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.85;
  filter: drop-shadow(0 0 6px rgba(0, 0, 0, 0.3));
}

.app-icon img {
  width: 85%;
  height: 85%;
  object-fit: contain;
}
</style>
