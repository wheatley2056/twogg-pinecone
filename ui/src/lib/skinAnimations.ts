// Eigene Animationen für die Skin-Vorschau (skinview3d). Statt der fertigen Animationen von
// skinview3d berechnet jede Animation eine Pose aus der Zeit; der Treiber blendet beim Wechsel
// weich über, lässt den Kopf dem Mauszeiger folgen und schreibt die Pose ins Modell. So bleiben
// keine Reste der vorigen Animation stehen und Übergänge springen nicht.

import type { PlayerObject } from "skinview3d";

export type AnimationId = "idle" | "walk" | "run" | "wave" | "cheer" | "dance";

export const ANIMATIONS: AnimationId[] = ["idle", "walk", "run", "wave", "cheer", "dance"];

interface Pose {
  headX: number;
  headY: number;
  headZ: number;
  rArmX: number;
  rArmZ: number;
  lArmX: number;
  lArmZ: number;
  rLegX: number;
  lLegX: number;
  capeX: number;
  /** Ganze Figur: Versatz (Pixel) und Neigung. */
  x: number;
  y: number;
  rotX: number;
  rotY: number;
  rotZ: number;
}

const REST: Pose = {
  headX: 0, headY: 0, headZ: 0,
  rArmX: 0, rArmZ: -0.06, lArmX: 0, lArmZ: 0.06,
  rLegX: 0, lLegX: 0, capeX: 0.18,
  x: 0, y: 0, rotX: 0, rotY: 0, rotZ: 0,
};

/** Weiche, nicht periodisch wirkende Bewegung aus überlagerten Sinuswellen (−1 … 1). */
function drift(t: number, seed = 0): number {
  return (Math.sin(t + seed) + 0.6 * Math.sin(t * 1.618 + 1.3 + seed) + 0.3 * Math.sin(t * 2.71 + 0.4 + seed)) / 1.9;
}

const { sin, cos, abs, max, PI } = Math;

const POSES: Record<AnimationId, (t: number) => Pose> = {
  // Atmen, Gewicht verlagern, sich umschauen.
  idle: (t) => {
    const breathe = sin(t * 1.7);
    return {
      ...REST,
      headX: 0.05 * sin(t * 0.9) + 0.04 * drift(t * 0.4, 2),
      headY: 0.32 * drift(t * 0.35),
      headZ: 0.04 * sin(t * 0.7),
      rArmX: 0.05 * sin(t * 0.8),
      lArmX: 0.05 * sin(t * 0.8 + PI),
      rArmZ: -0.07 - 0.03 * breathe,
      lArmZ: 0.07 + 0.03 * breathe,
      y: 0.18 * breathe,
      rotZ: 0.02 * sin(t * 0.5),
      rotY: 0.06 * drift(t * 0.25, 5),
      capeX: 0.16 + 0.03 * sin(t * 1.3),
    };
  },
  // Gehen mit Wippen, Hüftdrehung und ruhigem Kopf.
  walk: (t) => {
    const w = t * 5.6;
    const s = sin(w);
    return {
      ...REST,
      lLegX: 0.62 * s,
      rLegX: -0.62 * s,
      lArmX: -0.58 * s,
      rArmX: 0.58 * s,
      lArmZ: 0.07 + 0.03 * abs(s),
      rArmZ: -0.07 - 0.03 * abs(s),
      y: 0.35 - 0.6 * abs(s),
      rotY: 0.06 * s,
      rotZ: 0.025 * cos(w),
      headY: -0.05 * s + 0.12 * drift(t * 0.3),
      headX: 0.03 * cos(2 * w),
      capeX: 0.32 + 0.07 * sin(2 * w),
    };
  },
  // Laufen: nach vorn geneigt, große Schritte, Sprung bei jedem Schritt.
  run: (t) => {
    const w = t * 9.5;
    const s = sin(w);
    return {
      ...REST,
      lLegX: 1.1 * s,
      rLegX: -1.1 * s,
      lArmX: -1.15 * s,
      rArmX: 1.15 * s,
      lArmZ: 0.16,
      rArmZ: -0.16,
      y: 1.1 * abs(cos(w)),
      x: 0.12 * s,
      rotX: 0.14,
      rotY: 0.09 * s,
      headX: -0.16 + 0.04 * cos(2 * w),
      capeX: 0.85 + 0.12 * sin(2 * w),
    };
  },
  // Winken mit leichtem Wiegen.
  wave: (t) => {
    const idle = POSES.idle(t);
    return {
      ...idle,
      rArmX: PI * 0.92,
      rArmZ: -0.2 + 0.42 * sin(t * 7.5),
      headY: 0.18 + 0.05 * sin(t * 1.5),
      headZ: 0.08 + 0.03 * sin(t * 3.75),
      rotY: 0.1 + 0.04 * sin(t * 1.5),
      rotZ: -0.02 + 0.02 * sin(t * 3.75),
    };
  },
  // Jubeln: Arme hoch, kleine Sprünge.
  cheer: (t) => {
    const hop = max(0, sin(t * 6.5));
    return {
      ...REST,
      rArmX: PI * 0.86 + 0.08 * sin(t * 13),
      lArmX: PI * 0.86 + 0.08 * sin(t * 13 + 1),
      rArmZ: -0.42 - 0.18 * sin(t * 6.5),
      lArmZ: 0.42 + 0.18 * sin(t * 6.5),
      lLegX: 0.15 * hop,
      rLegX: -0.15 * hop,
      y: 2.4 * hop * hop,
      headX: -0.22 + 0.05 * sin(t * 6.5),
      headZ: 0.05 * sin(t * 3.25),
      capeX: 0.2 + 0.25 * hop,
    };
  },
  // Tanzen: Hüftschwung, Arme im Wechsel, im Takt wippen.
  dance: (t) => {
    const beat = t * 6;
    const sway = sin(t * 3);
    return {
      ...REST,
      rotY: 0.38 * sway,
      rotZ: 0.06 * sin(t * 6),
      x: 0.6 * sway,
      y: 0.7 * abs(sin(beat)),
      rArmX: 0.7 + 0.6 * sin(beat),
      lArmX: 0.7 + 0.6 * sin(beat + PI),
      rArmZ: -0.35 - 0.25 * max(0, sin(t * 3)),
      lArmZ: 0.35 + 0.25 * max(0, -sin(t * 3)),
      lLegX: 0.3 * sin(beat),
      rLegX: -0.3 * sin(beat),
      headZ: 0.14 * sway,
      headY: -0.15 * sway,
      headX: 0.06 * sin(beat * 2),
      capeX: 0.3 + 0.15 * abs(sin(beat)),
    };
  },
};

const FADE_S = 0.45;

function ease(x: number): number {
  return x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2;
}

function mix(a: Pose, b: Pose, w: number): Pose {
  const out = { ...a };
  for (const k of Object.keys(out) as (keyof Pose)[]) out[k] = a[k] + (b[k] - a[k]) * w;
  return out;
}

/** Treiber: hält Zeit, Überblendung und Blickrichtung; `apply` läuft in jedem Frame. */
export class SkinAnimator {
  private time = 0;
  private current: AnimationId;
  private previous: AnimationId | null = null;
  private fade = 1;
  private look = { x: 0, y: 0 };
  private lookTarget = { x: 0, y: 0 };
  /** 0 … 1: Ausschlag der Bewegungen (für „Animationen reduzieren“). */
  intensity = 1;

  constructor(initial: AnimationId = "idle") {
    this.current = initial;
  }

  play(id: AnimationId) {
    if (id === this.current) return;
    this.previous = this.current;
    this.current = id;
    this.fade = 0;
  }

  /** Mauszeiger relativ zur Vorschau (−1 … 1), `null` wenn er sie verlässt. */
  lookAt(pointer: { x: number; y: number } | null) {
    this.lookTarget = pointer ? { x: pointer.x, y: pointer.y } : { x: 0, y: 0 };
  }

  apply(player: PlayerObject, delta: number) {
    const dt = Math.min(delta, 0.1);
    this.time += dt;
    this.fade = Math.min(1, this.fade + dt / FADE_S);

    let pose = POSES[this.current](this.time);
    if (this.previous && this.fade < 1) pose = mix(POSES[this.previous](this.time), pose, ease(this.fade));
    if (this.intensity < 1) pose = mix(REST, pose, this.intensity);

    // Blick folgt dem Zeiger mit etwas Trägheit.
    const k = 1 - Math.exp(-dt * 7);
    this.look.x += (this.lookTarget.x - this.look.x) * k;
    this.look.y += (this.lookTarget.y - this.look.y) * k;
    pose.headY += this.look.x * 0.75;
    pose.headX += this.look.y * 0.45;

    const s = player.skin;
    s.head.rotation.set(pose.headX, pose.headY, pose.headZ);
    s.body.rotation.set(0, 0, 0);
    s.rightArm.rotation.set(pose.rArmX, 0, pose.rArmZ);
    s.leftArm.rotation.set(pose.lArmX, 0, pose.lArmZ);
    s.rightLeg.rotation.set(pose.rLegX, 0, 0);
    s.leftLeg.rotation.set(pose.lLegX, 0, 0);
    player.cape.rotation.x = pose.capeX;
    player.position.set(pose.x, pose.y, 0);
    player.rotation.set(pose.rotX, pose.rotY, pose.rotZ);
  }
}
