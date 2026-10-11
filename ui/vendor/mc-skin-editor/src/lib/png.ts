// PNG import/export helpers. Canvas PNG export always preserves the alpha
// channel and is pixel-lossless (PNG's DEFLATE compression is lossless, so
// "uncompressed" in practice just means: no format conversion, alpha kept).

function get2dContext (canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = canvas.getContext('2d')
  if (ctx == null) throw new Error('2D canvas context unavailable')
  return ctx
}

export function imageDataToCanvas (imageData: ImageData): HTMLCanvasElement {
  const canvas = document.createElement('canvas')
  canvas.width = imageData.width
  canvas.height = imageData.height
  const ctx = get2dContext(canvas)
  ctx.putImageData(imageData, 0, 0)
  return canvas
}

export function canvasToImageData (canvas: HTMLCanvasElement): ImageData {
  const ctx = get2dContext(canvas)
  return ctx.getImageData(0, 0, canvas.width, canvas.height)
}

export function imageDataToDataUrl (imageData: ImageData): string {
  return imageDataToCanvas(imageData).toDataURL('image/png')
}

export async function dataUrlToImageData (dataUrl: string): Promise<ImageData> {
  return await new Promise((resolve, reject) => {
    const img = new Image()
    img.onload = () => {
      const canvas = document.createElement('canvas')
      canvas.width = img.width
      canvas.height = img.height
      const ctx = get2dContext(canvas)
      ctx.drawImage(img, 0, 0)
      resolve(ctx.getImageData(0, 0, img.width, img.height))
    }
    img.onerror = () => reject(new Error('Failed to load image'))
    img.src = dataUrl
  })
}

export async function fileToImageData (file: File): Promise<ImageData> {
  return await new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      dataUrlToImageData(reader.result as string).then(resolve, reject)
    }
    reader.onerror = () => reject(new Error('Failed to read file'))
    reader.readAsDataURL(file)
  })
}

export function downloadImageData (imageData: ImageData, filename: string): void {
  const canvas = imageDataToCanvas(imageData)
  canvas.toBlob((blob) => {
    if (blob == null) return
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    document.body.appendChild(a)
    a.click()
    a.remove()
    URL.revokeObjectURL(url)
  }, 'image/png')
}
