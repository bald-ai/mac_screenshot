import { invoke } from "@tauri-apps/api/core";

const DIVIDER_HEIGHT = 30;
const BACKGROUND_COLOR = "#ffffff";
const LINE_COLOR = "#333333";

export interface StitchResult {
  base64Data: string;
  width: number;
  height: number;
  maxSingleImageWidth: number;
  maxSingleImageHeight: number;
}

export async function stitchImages(imagePaths: string[]): Promise<StitchResult> {
  const imagePromises = imagePaths.map((path) =>
    invoke<string>("read_original_image_base64", { filepath: path })
  );
  const base64Images = await Promise.all(imagePromises);

  const images: HTMLImageElement[] = await Promise.all(
    base64Images.map((base64) => loadImage(base64))
  );

  const maxWidth = Math.max(...images.map((img) => img.width));
  const maxSingleImageWidth = maxWidth;
  const maxSingleImageHeight = Math.max(...images.map((img) => img.height));
  const totalHeight =
    images.reduce((sum, img) => sum + img.height, 0) +
    (images.length - 1) * DIVIDER_HEIGHT;

  const canvas = document.createElement("canvas");
  canvas.width = maxWidth;
  canvas.height = totalHeight;
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("Failed to get canvas 2D context");
  }

  ctx.fillStyle = BACKGROUND_COLOR;
  ctx.fillRect(0, 0, maxWidth, totalHeight);

  let currentY = 0;
  for (let i = 0; i < images.length; i++) {
    const img = images[i];
    const xOffset = (maxWidth - img.width) / 2;
    ctx.drawImage(img, xOffset, currentY);
    currentY += img.height;

    if (i < images.length - 1) {
      ctx.fillStyle = BACKGROUND_COLOR;
      ctx.fillRect(0, currentY, maxWidth, DIVIDER_HEIGHT);
      
      const rectHeight = 12;
      const rectY = currentY + Math.floor(DIVIDER_HEIGHT / 2) - rectHeight / 2;
      ctx.fillStyle = LINE_COLOR;
      ctx.fillRect(0, rectY, maxWidth, rectHeight);
      
      currentY += DIVIDER_HEIGHT;
    }
  }

  const dataUrl = canvas.toDataURL("image/png");
  const base64Data = dataUrl.replace(/^data:image\/png;base64,/, "");

  return {
    base64Data,
    width: maxWidth,
    height: totalHeight,
    maxSingleImageWidth,
    maxSingleImageHeight,
  };
}

function loadImage(base64: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = base64;
  });
}
