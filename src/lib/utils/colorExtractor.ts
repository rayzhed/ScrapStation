export async function extractDominantColor(imageSrc: string): Promise<string> {
    return new Promise((resolve) => {
        const img = new Image();
        img.crossOrigin = 'anonymous';
        img.src = imageSrc;

        img.onload = () => {
            try {
                const canvas = document.createElement('canvas');
                const ctx = canvas.getContext('2d', { willReadFrequently: true });

                if (!ctx) {
                    resolve('0, 229, 255');
                    return;
                }

                const size = 100;
                canvas.width = size;
                canvas.height = size;
                ctx.drawImage(img, 0, 0, size, size);

                const imageData = ctx.getImageData(0, 0, size, size);
                const data = imageData.data;

                const centerX = size / 2;
                const centerY = size / 2;
                const radius = size / 3;

                const colors: number[][] = [];

                for (let y = 0; y < size; y++) {
                    for (let x = 0; x < size; x++) {
                        const dx = x - centerX;
                        const dy = y - centerY;
                        const distance = Math.sqrt(dx * dx + dy * dy);

                        if (distance > radius) continue;

                        const i = (y * size + x) * 4;
                        const r = data[i];
                        const g = data[i + 1];
                        const b = data[i + 2];
                        const a = data[i + 3];

                        if (a < 125) continue;

                        const brightness = r + g + b;
                        if (brightness < 100 || brightness > 650) continue;

                        colors.push([r, g, b]);
                    }
                }

                if (colors.length === 0) {
                    resolve('0, 229, 255');
                    return;
                }

                let totalR = 0, totalG = 0, totalB = 0;

                for (const [r, g, b] of colors) {
                    const vibrancy = Math.max(r, g, b) - Math.min(r, g, b);
                    const weight = 1 + (vibrancy / 255);

                    totalR += r * weight;
                    totalG += g * weight;
                    totalB += b * weight;
                }

                let avgR = Math.floor(totalR / colors.length);
                let avgG = Math.floor(totalG / colors.length);
                let avgB = Math.floor(totalB / colors.length);

                const saturation = 1.6;
                avgR = Math.min(255, Math.floor(avgR * saturation));
                avgG = Math.min(255, Math.floor(avgG * saturation));
                avgB = Math.min(255, Math.floor(avgB * saturation));

                resolve(`${avgR}, ${avgG}, ${avgB}`);

            } catch {
                resolve('0, 229, 255');
            }
        };

        img.onerror = () => {
            resolve('0, 229, 255');
        };
    });
}