import { ref } from "vue";

// Non-reactive cache
const headUrlCache = new Map<string, string>();

export async function generatePlayerHeadBlob(
  skinUrl: string,
  size: number = 64
): Promise<Blob> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.decoding = "async";

    img.onload = () => {
      try {
        const sourceCanvas = document.createElement("canvas");
        const sourceCtx = sourceCanvas.getContext("2d", { willReadFrequently: true });
        if (!sourceCtx) throw new Error("No 2D context");

        const w = img.width;
        const h = img.height;
        sourceCanvas.width = w;
        sourceCanvas.height = h;
        sourceCtx.drawImage(img, 0, 0);

        const outputCanvas = document.createElement("canvas");
        const outputCtx = outputCanvas.getContext("2d");
        if (!outputCtx) throw new Error("No 2D context");

        outputCanvas.width = size;
        outputCanvas.height = size;

        // Head layer (8,8 -> 8x8)
        const headData = sourceCtx.getImageData(8, 8, 8, 8);
        const headCanvas = document.createElement("canvas");
        headCanvas.width = 8;
        headCanvas.height = 8;
        headCanvas.getContext("2d")!.putImageData(headData, 0, 0);

        outputCtx.imageSmoothingEnabled = false;
        outputCtx.drawImage(headCanvas, 0, 0, 8, 8, 0, 0, size, size);

        // Hat layer (only for 64x64 skins at 40,8)
        if (h >= 64) {
          const hatData = sourceCtx.getImageData(40, 8, 8, 8);
          const hatPixels = hatData.data;
          let hasHat = false;
          for (let i = 3; i < hatPixels.length; i += 4) {
            if (hatPixels[i] > 10) { // Not fully transparent
              hasHat = true;
              break;
            }
          }

          if (hasHat) {
            const hatCanvas = document.createElement("canvas");
            hatCanvas.width = 8;
            hatCanvas.height = 8;
            hatCanvas.getContext("2d")!.putImageData(hatData, 0, 0);
            outputCtx.drawImage(hatCanvas, 0, 0, 8, 8, 0, 0, size, size);
          }
        }

        outputCanvas.toBlob((blob) => {
          if (blob) resolve(blob);
          else reject(new Error("Failed to create blob"));
        }, "image/png", 1.0);
      } catch (error) {
        reject(error);
      }
    };

    img.onerror = () => reject(new Error("Failed to load skin image"));
    img.src = skinUrl;
  });
}

export async function getPlayerHeadUrl(
  skinUrl: string,
  textureKey?: string
): Promise<string> {
  const cacheKey = textureKey || skinUrl;
  const cached = headUrlCache.get(cacheKey);
  if (cached) return cached;

  const blob = await generatePlayerHeadBlob(skinUrl, 64);
  const blobUrl = URL.createObjectURL(blob);
  headUrlCache.set(cacheKey, blobUrl);
  return blobUrl;
}

export function getMcHeadsUrl(uuid: string, size: number = 64): string {
  return `https://mc-heads.net/avatar/${uuid}/${size}`;
}

export function revokeHeadUrl(key: string) {
  const url = headUrlCache.get(key);
  if (url) {
    URL.revokeObjectURL(url);
    headUrlCache.delete(key);
  }
}

export function useSkinRenderer() {
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  const headUrl = ref<string | null>(null);

  async function renderHead(skinUrl: string, textureKey?: string) {
    isLoading.value = true;
    error.value = null;

    try {
      headUrl.value = await getPlayerHeadUrl(skinUrl, textureKey);
    } catch (err) {
      error.value = err instanceof Error ? err.message : "Unknown error";
      headUrl.value = null;
    } finally {
      isLoading.value = false;
    }
  }

  return {
    isLoading,
    error,
    headUrl,
    renderHead,
  };
}
