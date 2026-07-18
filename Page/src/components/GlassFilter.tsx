import { useEffect, useState } from 'react'
import { LiquidGlassFilter } from '@sohumsuthar/liquid-glass'

const roundedRectDistance = (
  x: number,
  y: number,
  width: number,
  height: number,
  radius: number,
) => {
  const qx = Math.abs(x - width / 2) - width / 2 + radius
  const qy = Math.abs(y - height / 2) - height / 2 + radius
  return (
    Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) +
    Math.min(Math.max(qx, qy), 0) -
    radius
  )
}

const createDisplacementMap = () => {
  const size = 160
  const radius = 28
  const bezel = 22
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const context = canvas.getContext('2d')
  if (!context) return undefined

  const image = context.createImageData(size, size)
  for (let y = 0; y < size; y += 1) {
    for (let x = 0; x < size; x += 1) {
      const index = (y * size + x) * 4
      const distance = -roundedRectDistance(x + 0.5, y + 0.5, size, size, radius)
      let red = 128
      let green = 128

      if (distance > 0 && distance < bezel) {
        const delta = 0.8
        const dx =
          roundedRectDistance(x + delta, y, size, size, radius) -
          roundedRectDistance(x - delta, y, size, size, radius)
        const dy =
          roundedRectDistance(x, y + delta, size, size, radius) -
          roundedRectDistance(x, y - delta, size, size, radius)
        const length = Math.hypot(dx, dy) || 1
        const strength = Math.pow(1 - distance / bezel, 1.65)
        red = Math.round(128 - (dx / length) * strength * 118)
        green = Math.round(128 - (dy / length) * strength * 118)
      }

      image.data[index] = red
      image.data[index + 1] = green
      image.data[index + 2] = 128
      image.data[index + 3] = 255
    }
  }

  context.putImageData(image, 0, 0)
  return canvas.toDataURL('image/png')
}

export function GlassFilter() {
  const [map, setMap] = useState<string>()

  useEffect(() => {
    setMap(createDisplacementMap())
  }, [])

  return <LiquidGlassFilter displacementMap={map} />
}
