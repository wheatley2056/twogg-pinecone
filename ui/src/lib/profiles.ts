// Gemeinsame Anzeige-Helfer für Profile (Versionen immer in Mono: „Fabric 1.21.11“, „Vanilla 26.2“).

import type { LoaderKind, Profile } from "./types";

const LOADERS: Record<LoaderKind, string> = { fabric: "Fabric", quilt: "Quilt" };

export function loaderName(kind: LoaderKind | null | undefined): string {
  return kind ? LOADERS[kind] : "Vanilla";
}

/** „Fabric 1.21.11“ */
export function versionLabel(p: Pick<Profile, "minecraft" | "loader">): string {
  return `${loaderName(p.loader?.kind)} ${p.minecraft}`;
}

export function isTwogg(p: Pick<Profile, "packSource">): boolean {
  return p.packSource === "twogg";
}

/** Zuletzt gespielt zuerst, nie gespielte danach alphabetisch. */
export function byRecent(a: Profile, b: Profile): number {
  return (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0) || a.name.localeCompare(b.name);
}
