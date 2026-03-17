import { invoke } from '@tauri-apps/api/core';

const imageCache = new Map<string, string>();

/** Returns true if the path points to a local file rather than an http(s) URL. */
function isLocalPath(path: string): boolean {
    return !path.startsWith('http://') && !path.startsWith('https://');
}

export async function getOptimizedImage(urlOrPath: string): Promise<string> {
    if (!urlOrPath) return '';
    if (imageCache.has(urlOrPath)) {
        return imageCache.get(urlOrPath)!;
    }

    try {
        // Use read_local_image for on-disk covers, fetch_image for remote URLs
        const imageBytes = isLocalPath(urlOrPath)
            ? await invoke<number[]>('read_local_image', { path: urlOrPath })
            : await invoke<number[]>('fetch_image', { url: urlOrPath });

        const url = urlOrPath; // alias for cache key below
        const uint8Array = new Uint8Array(imageBytes);

        const mimeType = getMimeType(urlOrPath);
        const blob = new Blob([uint8Array], { type: mimeType });
        const originalUrl = URL.createObjectURL(blob);

        const optimized = await upscaleAndEnhance(originalUrl, mimeType);
        imageCache.set(url, optimized);

        return optimized;
    } catch {
        return '';
    }
}

function getMimeType(url: string): string {
    const lower = url.toLowerCase();
    if (lower.endsWith('.png')) return 'image/png';
    if (lower.endsWith('.webp')) return 'image/webp';
    return 'image/jpeg';
}

async function upscaleAndEnhance(
    imageUrl: string,
    mimeType: string
): Promise<string> {
    return new Promise((resolve, reject) => {
        const img = new Image();

        img.onload = () => {
            const maxWidth = 600;
            const maxHeight = 800;

            let targetWidth = Math.min(img.width * 2, maxWidth);
            let targetHeight = Math.min(img.height * 2, maxHeight);

            const aspectRatio = img.width / img.height;
            if (targetWidth / targetHeight > aspectRatio) {
                targetWidth = targetHeight * aspectRatio;
            } else {
                targetHeight = targetWidth / aspectRatio;
            }

            const canvas = document.createElement('canvas');
            canvas.width = targetWidth;
            canvas.height = targetHeight;
            const ctx = canvas.getContext('2d');

            if (!ctx) {
                resolve(imageUrl);
                return;
            }

            ctx.imageSmoothingEnabled = true;
            ctx.imageSmoothingQuality = 'high';
            ctx.drawImage(img, 0, 0, targetWidth, targetHeight);

            // Appliquer sharpen
            const imageData = ctx.getImageData(0, 0, targetWidth, targetHeight);
            const sharpened = applySharpen(imageData);
            ctx.putImageData(sharpened, 0, 0);

            canvas.toBlob(
                (blob) => {
                    if (blob) {
                        const optimizedUrl = URL.createObjectURL(blob);
                        URL.revokeObjectURL(imageUrl);
                        resolve(optimizedUrl);
                    } else {
                        resolve(imageUrl);
                    }
                },
                mimeType,
                0.95
            );
        };

        img.onerror = () => {
            URL.revokeObjectURL(imageUrl);
            reject(new Error('Failed to load image'));
        };

        img.src = imageUrl;
    });
}

function applySharpen(imageData: ImageData): ImageData {
    const data = imageData.data;
    const width = imageData.width;
    const height = imageData.height;
    const output = new ImageData(width, height);

    const kernel = [0, -0.5, 0, -0.5, 3, -0.5, 0, -0.5, 0];

    for (let y = 1; y < height - 1; y++) {
        for (let x = 1; x < width - 1; x++) {
            for (let c = 0; c < 3; c++) {
                let sum = 0;
                for (let ky = -1; ky <= 1; ky++) {
                    for (let kx = -1; kx <= 1; kx++) {
                        const idx = ((y + ky) * width + (x + kx)) * 4 + c;
                        const kernelIdx = (ky + 1) * 3 + (kx + 1);
                        sum += data[idx] * kernel[kernelIdx];
                    }
                }
                const idx = (y * width + x) * 4 + c;
                output.data[idx] = Math.min(255, Math.max(0, sum));
            }
            const idx = (y * width + x) * 4 + 3;
            output.data[idx] = data[idx];
        }
    }

    return output;
}