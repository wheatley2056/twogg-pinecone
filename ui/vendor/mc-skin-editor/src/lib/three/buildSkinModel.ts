// Builds the 3D player model as a Group of per-part, per-layer meshes,
// each with hand-built UVs pointing at the correct region of the skin
// texture atlas (see ../skin/uv.ts). Because every mesh's UV already points
// at the right atlas pixels — including the legacy left-limb mirroring — a
// raycast hit's interpolated UV converts directly to a texture pixel with no
// further part/face bookkeeping needed.

import {
  BufferGeometry,
  CanvasTexture,
  DoubleSide,
  EdgesGeometry,
  Float32BufferAttribute,
  Group,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  NearestFilter,
  SRGBColorSpace
} from 'three'
import type { FaceName, LayerName, ModelType, PartName, ResolutionId } from '../skin/types'
import { PART_NAMES, partDims, renderFaceSource, type FaceRect } from '../skin/uv'
import { partTransform, OVERLAY_INSET } from '../skin/layout'
import { RESOLUTIONS } from '../skin/resolutions'

export interface PartMeshes {
  part: PartName
  base: Mesh | null
  overlay: Mesh | null
  outline: LineSegments | null
  baseGrid: LineSegments | null
  overlayGrid: LineSegments | null
  /** Invisible depth-only twin of `base`, so unpainted (fully transparent) skin
   *  pixels still block whatever is behind them — the opposite wall of the same
   *  box, other parts, or their grid/outline lines — instead of acting as a
   *  see-through hole into the model's interior. */
  baseOccluder: Mesh | null
  /** Same trick as `baseOccluder`, but for the overlay layer. Only meant to be
   *  shown when the base layer isn't backstopping it (i.e. the base layer is
   *  toggled off) — otherwise it would block the base layer from showing
   *  through the overlay's legitimate transparent areas (hair through a gap
   *  in a hat, skin through a sleeve, etc). */
  overlayOccluder: Mesh | null
}

export interface SkinModel {
  group: Group
  parts: PartMeshes[]
  texture: CanvasTexture
  /** All meshes, for raycasting. */
  meshes: Mesh[]
  dispose: () => void
}

type Corner = [number, number, number]

function addFace (
  positions: number[],
  normals: number[],
  uvs: number[],
  indices: number[],
  corners: { tl: Corner, tr: Corner, bl: Corner, br: Corner },
  normal: Corner,
  rect: FaceRect,
  atlasW: number,
  atlasH: number,
  flipX: boolean
): void {
  const base = positions.length / 3
  const u0 = rect.x / atlasW
  const u1 = (rect.x + rect.w) / atlasW
  const v0 = rect.y / atlasH
  const v1 = (rect.y + rect.h) / atlasH
  const uLeft = flipX ? u1 : u0
  const uRight = flipX ? u0 : u1

  positions.push(...corners.tl, ...corners.tr, ...corners.bl, ...corners.br)
  for (let i = 0; i < 4; i++) normals.push(...normal)
  uvs.push(uLeft, v0, uRight, v0, uLeft, v1, uRight, v1)
  indices.push(base, base + 2, base + 1, base + 1, base + 2, base + 3)
}

function lerpCorner (a: Corner, b: Corner, t: number): Corner {
  return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/** Appends one grid line per pixel boundary on a face, so individual skin pixels read as a grid on the 3D model (mirroring the 2D texture map's per-pixel grid). */
function addFaceGrid (
  gridPositions: number[],
  corners: { tl: Corner, tr: Corner, bl: Corner, br: Corner },
  cols: number,
  rows: number
): void {
  for (let i = 0; i <= cols; i++) {
    const t = i / cols
    gridPositions.push(...lerpCorner(corners.tl, corners.tr, t), ...lerpCorner(corners.bl, corners.br, t))
  }
  for (let j = 0; j <= rows; j++) {
    const t = j / rows
    gridPositions.push(...lerpCorner(corners.tl, corners.bl, t), ...lerpCorner(corners.tr, corners.br, t))
  }
}

interface PartGeometry {
  geometry: BufferGeometry
  gridPositions: number[]
}

/** Builds one part's box geometry (base or inflated overlay), or null if this part/layer has no texture to show. */
function buildPartGeometry (
  part: PartName,
  layer: LayerName,
  model: ModelType,
  resolution: ResolutionId,
  atlasW: number,
  atlasH: number
): PartGeometry | null {
  const dims = partDims(part, model)
  const inset = layer === 'overlay' ? OVERLAY_INSET : 0
  const hx = dims.dx / 2 + inset
  const hy = dims.dy / 2 + inset
  const hz = dims.dz / 2 + inset

  const positions: number[] = []
  const normals: number[] = []
  const uvs: number[] = []
  const indices: number[] = []
  let any = false

  const faces: Array<{ face: FaceName, normal: Corner, corners: { tl: Corner, tr: Corner, bl: Corner, br: Corner } }> = [
    {
      face: 'front',
      normal: [0, 0, 1],
      corners: { tl: [-hx, hy, hz], tr: [hx, hy, hz], bl: [-hx, -hy, hz], br: [hx, -hy, hz] }
    },
    {
      face: 'back',
      normal: [0, 0, -1],
      corners: { tl: [hx, hy, -hz], tr: [-hx, hy, -hz], bl: [hx, -hy, -hz], br: [-hx, -hy, -hz] }
    },
    {
      face: 'left',
      normal: [1, 0, 0],
      corners: { tl: [hx, hy, hz], tr: [hx, hy, -hz], bl: [hx, -hy, hz], br: [hx, -hy, -hz] }
    },
    {
      face: 'right',
      normal: [-1, 0, 0],
      corners: { tl: [-hx, hy, -hz], tr: [-hx, hy, hz], bl: [-hx, -hy, -hz], br: [-hx, -hy, hz] }
    },
    {
      face: 'top',
      normal: [0, 1, 0],
      corners: { tl: [-hx, hy, -hz], tr: [hx, hy, -hz], bl: [-hx, hy, hz], br: [hx, hy, hz] }
    },
    {
      face: 'bottom',
      normal: [0, -1, 0],
      corners: { tl: [-hx, -hy, hz], tr: [hx, -hy, hz], bl: [-hx, -hy, -hz], br: [hx, -hy, -hz] }
    }
  ]

  const gridPositions: number[] = []

  for (const f of faces) {
    const source = renderFaceSource(part, layer, f.face, model, resolution)
    if (source == null) continue
    any = true
    addFace(positions, normals, uvs, indices, f.corners, f.normal, source.rect, atlasW, atlasH, source.flipX)
    addFaceGrid(gridPositions, f.corners, source.rect.w, source.rect.h)
  }

  if (!any) return null

  const geometry = new BufferGeometry()
  geometry.setAttribute('position', new Float32BufferAttribute(positions, 3))
  geometry.setAttribute('normal', new Float32BufferAttribute(normals, 3))
  geometry.setAttribute('uv', new Float32BufferAttribute(uvs, 2))
  geometry.setIndex(indices)
  return { geometry, gridPositions }
}

export function buildSkinModel (canvas: HTMLCanvasElement, model: ModelType, resolution: ResolutionId): SkinModel {
  const texture = new CanvasTexture(canvas)
  texture.flipY = false
  texture.magFilter = NearestFilter
  texture.minFilter = NearestFilter
  texture.generateMipmaps = false
  texture.colorSpace = SRGBColorSpace

  const material = new MeshBasicMaterial({
    map: texture,
    side: DoubleSide,
    alphaTest: 0.05,
    transparent: false
  })
  const overlayMaterial = material.clone()
  overlayMaterial.alphaTest = 0.05
  const outlineMaterial = new LineBasicMaterial({ color: 0x8a8f99, transparent: true, opacity: 0.55 })
  // Writes depth for every fragment of the base box regardless of the skin
  // texture's alpha, so unpainted (fully transparent) areas still block
  // whatever is behind them instead of turning into a see-through hole into
  // the model's interior. Never paints color, so it doesn't affect what the
  // textured meshes actually show.
  const occluderMaterial = new MeshBasicMaterial({ colorWrite: false, depthWrite: true })

  const group = new Group()
  const parts: PartMeshes[] = []
  const meshes: Mesh[] = []
  const outlines: LineSegments[] = []
  const grids: LineSegments[] = []
  const info = RESOLUTIONS[resolution]

  for (const part of PART_NAMES) {
    const transform = partTransform(part, model)
    const entry: PartMeshes = { part, base: null, overlay: null, outline: null, baseGrid: null, overlayGrid: null, baseOccluder: null, overlayOccluder: null }

    const baseGeom = buildPartGeometry(part, 'base', model, resolution, info.width, info.height)
    if (baseGeom != null) {
      const mesh = new Mesh(baseGeom.geometry, material)
      mesh.position.set(transform.center.x, transform.center.y, transform.center.z)
      mesh.userData = { part, layer: 'base' as LayerName }
      group.add(mesh)
      entry.base = mesh
      meshes.push(mesh)

      const occluder = new Mesh(baseGeom.geometry, occluderMaterial)
      occluder.position.copy(mesh.position)
      group.add(occluder)
      entry.baseOccluder = occluder

      // A faint wireframe so the body shape reads even before anything
      // has been painted (a fresh skin is fully transparent and the
      // textured mesh is invisible via alphaTest). Hidden along with the
      // rest of the base layer when that layer is toggled off.
      const outline = new LineSegments(new EdgesGeometry(baseGeom.geometry), outlineMaterial)
      outline.position.copy(mesh.position)
      group.add(outline)
      entry.outline = outline
      outlines.push(outline)

      // Per-pixel grid on the base layer's surface, so individual skin
      // pixels read as a grid — mirrors the 2D texture map's pixel grid.
      const gridGeom = new BufferGeometry()
      gridGeom.setAttribute('position', new Float32BufferAttribute(baseGeom.gridPositions, 3))
      const baseGrid = new LineSegments(gridGeom, outlineMaterial)
      baseGrid.position.copy(mesh.position)
      group.add(baseGrid)
      entry.baseGrid = baseGrid
      grids.push(baseGrid)
    }

    const overlayGeom = buildPartGeometry(part, 'overlay', model, resolution, info.width, info.height)
    if (overlayGeom != null) {
      const mesh = new Mesh(overlayGeom.geometry, overlayMaterial)
      mesh.position.set(transform.center.x, transform.center.y, transform.center.z)
      mesh.userData = { part, layer: 'overlay' as LayerName }
      group.add(mesh)
      entry.overlay = mesh
      meshes.push(mesh)

      const overlayOccluder = new Mesh(overlayGeom.geometry, occluderMaterial)
      overlayOccluder.position.copy(mesh.position)
      group.add(overlayOccluder)
      entry.overlayOccluder = overlayOccluder

      const gridGeom = new BufferGeometry()
      gridGeom.setAttribute('position', new Float32BufferAttribute(overlayGeom.gridPositions, 3))
      const overlayGrid = new LineSegments(gridGeom, outlineMaterial)
      overlayGrid.position.copy(mesh.position)
      group.add(overlayGrid)
      entry.overlayGrid = overlayGrid
      grids.push(overlayGrid)
    }

    parts.push(entry)
  }

  return {
    group,
    parts,
    texture,
    meshes,
    dispose () {
      for (const mesh of meshes) {
        mesh.geometry.dispose()
      }
      for (const outline of outlines) {
        outline.geometry.dispose()
      }
      for (const grid of grids) {
        grid.geometry.dispose()
      }
      material.dispose()
      overlayMaterial.dispose()
      outlineMaterial.dispose()
      occluderMaterial.dispose()
      texture.dispose()
    }
  }
}
