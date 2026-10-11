import type { Mesh, Raycaster, Vector2 } from 'three'
import type { PartName } from '../skin/types'

/** Converts a raycast hit's interpolated UV (with flipY=false texture convention) into an integer atlas pixel coordinate. */
export function uvToPixel (uv: Vector2, atlasWidth: number, atlasHeight: number): { x: number, y: number } {
  const x = Math.min(atlasWidth - 1, Math.max(0, Math.floor(uv.x * atlasWidth)))
  const y = Math.min(atlasHeight - 1, Math.max(0, Math.floor(uv.y * atlasHeight)))
  return { x, y }
}

export interface PixelHit {
  x: number
  y: number
  /** Which body part's mesh was hit — the atlas regions for different parts
   *  aren't adjacent, so a straight texture-space line between hits on two
   *  different parts wouldn't correspond to anything the cursor actually
   *  crossed on the model. */
  part: PartName
}

export function pickPixel (
  raycaster: Raycaster,
  meshes: Mesh[],
  atlasWidth: number,
  atlasHeight: number
): PixelHit | null {
  const hits = raycaster.intersectObjects(meshes, false)
  for (const hit of hits) {
    if (hit.uv == null) continue
    const part = (hit.object as Mesh).userData?.part as PartName | undefined
    if (part == null) continue
    return { ...uvToPixel(hit.uv, atlasWidth, atlasHeight), part }
  }
  return null
}
