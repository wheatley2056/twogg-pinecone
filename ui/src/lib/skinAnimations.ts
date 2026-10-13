// Eigene Animationen für die Skin-Vorschau (skinview3d). Jede Animation berechnet aus der Zeit eine
// Pose; der Treiber blendet beim Wechsel weich über, lässt den Kopf dem Mauszeiger folgen, lässt
// Umhang und Arme über Federn nachschwingen und schreibt die Pose ins Modell. So bleiben keine
// Reste der vorigen Animation stehen, und Übergänge springen nicht.
//
// Gestaltung: Bewegungen mit Ausholen und Nachfedern statt reiner Sinuswellen, Gehen und Laufen
// leicht schräg von vorn (sonst schwingen Arme und Beine nur auf die Kamera zu), dazu Schleichen
// und Abbauen wie im Spiel.
//
// Achtung: Bei skinview3d schwingt ein positiver X-Winkel Arme und Beine nach hinten; nach vorn
// heben heißt negativer Winkel.

import type { PlayerObject } from "skinview3d";

export type AnimationId = "idle" | "walk" | "run" | "sneak" | "mine" | "wave" | "cheer" | "dance";

export const ANIMATIONS: AnimationId[] = ["idle", "walk", "run", "sneak", "mine", "wave", "cheer", "dance"];

interface Pose {
  headX: number;
  headY: number;
  headZ: number;
  rArmX: number;
  rArmY: number;
  rArmZ: number;
  lArmX: number;
  lArmY: number;
  lArmZ: number;
  rLegX: number;
  rLegZ: number;
  lLegX: number;
  lLegZ: number;
  /** 0 … 1: Schleich-Haltung wie im Spiel (Oberkörper vor, Kopf und Arme tiefer). */
  crouch: number;
  capeX: number;
  /** Ganze Figur: Versatz (Pixel), Neigung und Blickrichtung zur Kamera. */
  x: number;
  y: number;
  rotX: number;
  rotY: number;
  rotZ: number;
}

const REST: Pose = {
  headX: 0, headY: 0, headZ: 0,
  rArmX: 0, rArmY: 0, rArmZ: -0.06,
  lArmX: 0, lArmY: 0, lArmZ: 0.06,
  rLegX: 0, rLegZ: 0, lLegX: 0, lLegZ: 0,
  crouch: 0, capeX: 0.18,
  x: 0, y: 0, rotX: 0, rotY: 0, rotZ: 0,
};

const { sin, cos, abs, max, min, PI } = Math;

/** Weiche, nicht periodisch wirkende Bewegung aus überlagerten Sinuswellen (−1 … 1). */
function drift(t: number, seed = 0): number {
  return (sin(t + seed) + 0.6 * sin(t * 1.618 + 1.3 + seed) + 0.3 * sin(t * 2.71 + 0.4 + seed)) / 1.9;
}

const clamp01 = (x: number) => min(1, max(0, x));
const smooth = (x: number) => {
  const c = clamp01(x);
  return c * c * (3 - 2 * c);
};
/** Anteil von x im Abschnitt [a, b], geglättet. */
const span = (x: number, a: number, b: number) => smooth((x - a) / (b - a));
/** Schnell hin, langsam zurück: für Schläge und Gesten mit Wucht. */
const snap = (x: number) => 1 - (1 - clamp01(x)) ** 3;

/** Ruhe-Haltung mit Atmen, Gewichtsverlagerung und gelegentlichem Umschauen. */
function idle(t: number): Pose {
  const breathe = sin(t * 1.6);
  // Alle ~7 s einmal deutlich zur Seite schauen.
  const glanceCycle = (t % 7.2) / 7.2;
  const glance = span(glanceCycle, 0.55, 0.62) - span(glanceCycle, 0.8, 0.88);
  const shift = drift(t * 0.3, 3);
  return {
    ...REST,
    headX: 0.05 * sin(t * 0.9) + 0.04 * drift(t * 0.4, 2) - 0.04 * breathe,
    headY: 0.18 * drift(t * 0.33) + 0.45 * glance * (sin(t * 0.11) > 0 ? 1 : -1),
    headZ: 0.03 * sin(t * 0.7) - 0.03 * shift,
    rArmX: 0.04 * sin(t * 0.8),
    lArmX: 0.04 * sin(t * 0.8 + PI),
    rArmZ: -0.07 - 0.025 * breathe + 0.02 * shift,
    lArmZ: 0.07 + 0.025 * breathe + 0.02 * shift,
    rLegZ: 0.02 * max(0, shift),
    lLegZ: 0.02 * min(0, shift),
    y: 0.15 * breathe,
    x: 0.25 * shift,
    rotZ: 0.025 * shift,
    rotY: 0.05 * drift(t * 0.2, 5),
    capeX: 0.14 + 0.02 * sin(t * 1.1),
  };
}

/** Schritt-Zyklus: Beine pendeln, Körper hebt sich, wenn ein Bein senkrecht steht. */
function stride(w: number, legSwing: number, armSwing: number, lift: number) {
  const s = sin(w);
  return {
    lLegX: legSwing * s,
    rLegX: -legSwing * s,
    // Arme leicht verzögert: wirkt wie Nachschwingen aus der Schulter.
    lArmX: -armSwing * sin(w - 0.25),
    rArmX: armSwing * sin(w - 0.25),
    y: lift * (abs(cos(w)) - 0.5),
  };
}

const POSES: Record<AnimationId, (t: number) => Pose> = {
  idle,

  walk: (t) => {
    const w = t * 5.2;
    const st = stride(w, 0.62, 0.55, 0.7);
    return {
      ...REST,
      ...st,
      lArmZ: 0.08,
      rArmZ: -0.08,
      rotY: -0.5 + 0.05 * sin(w),
      rotZ: 0.02 * cos(w),
      headX: 0.04 + 0.025 * cos(2 * w),
      headY: 0.32 + 0.1 * drift(t * 0.3),
      capeX: 0.34,
    };
  },

  run: (t) => {
    const w = t * 9.6;
    const st = stride(w, 1.1, 0.95, 1.6);
    return {
      ...REST,
      ...st,
      lArmZ: 0.18,
      rArmZ: -0.18,
      y: st.y + 0.5,
      rotX: 0.2,
      rotY: -0.55 + 0.08 * sin(w),
      rotZ: 0.03 * cos(w),
      headX: -0.22 + 0.04 * cos(2 * w),
      headY: 0.35,
      capeX: 0.95,
    };
  },

  sneak: (t) => {
    const w = t * 2.6;
    const s = sin(w);
    return {
      ...REST,
      crouch: 1,
      lLegX: 0.42 * s,
      rLegX: -0.42 * s,
      lArmX: -0.3 * sin(w - 0.3),
      rArmX: 0.3 * sin(w - 0.3),
      lArmZ: 0.1,
      rArmZ: -0.1,
      y: 0.2 * (abs(cos(w)) - 0.5),
      rotY: -0.6 + 0.03 * s,
      headX: 0.12,
      headY: 0.4 + 0.25 * drift(t * 0.5, 1),
      capeX: 0.3,
    };
  },

  // Abbauen: ausholen, kräftig nach unten schlagen, kurz nachfedern.
  mine: (t) => {
    const p = (t * 2.4) % 1;
    const windUp = span(p, 0, 0.35);
    const strike = snap((p - 0.35) / 0.15);
    const settle = span(p, 0.5, 1);
    // Armwinkel: aus der Haltung hoch ausholen, schnell nach unten, zurück in die Haltung.
    const swing = p < 0.35 ? 0.6 + 1.7 * windUp : p < 0.5 ? 2.3 - 2.0 * strike : 0.3 + 0.3 * settle;
    const impact = p > 0.48 && p < 0.6 ? sin(((p - 0.48) / 0.12) * PI) : 0;
    return {
      ...REST,
      rArmX: -swing,
      rArmY: 0.25 * windUp * (1 - strike),
      rArmZ: -0.1,
      lArmX: -0.25 - 0.1 * impact,
      lArmZ: 0.1,
      lLegX: -0.25,
      rLegX: 0.15,
      headX: 0.22 + 0.08 * impact,
      headY: 0.25,
      y: -0.25 * impact,
      rotX: 0.05 + 0.06 * strike * (1 - settle),
      rotY: -0.35 - 0.08 * windUp * (1 - strike),
      capeX: 0.2 + 0.1 * strike * (1 - settle),
    };
  },

  // Winken: Arm mit etwas Schwung heben, dreimal winken, kurz absetzen.
  wave: (t) => {
    const base = idle(t);
    const p = (t % 3.4) / 3.4;
    const up = span(p, 0, 0.12) - span(p, 0.82, 0.97);
    const waving = p > 0.1 && p < 0.84 ? sin((p - 0.1) * PI * 8.1) : 0;
    return {
      ...base,
      rArmX: base.rArmX * (1 - up) - up * (PI * 0.92),
      rArmZ: base.rArmZ * (1 - up) + up * (-0.3 + 0.38 * waving),
      rArmY: up * 0.1,
      headY: base.headY * (1 - up) + up * 0.12,
      headZ: base.headZ + up * (0.1 + 0.03 * waving),
      rotY: base.rotY + up * 0.12,
      rotZ: base.rotZ - up * 0.03,
    };
  },

  // Jubeln: in die Knie, hochspringen mit Armen nach oben, weich landen.
  cheer: (t) => {
    const p = (t % 1.25) / 1.25;
    const dip = span(p, 0, 0.18) - span(p, 0.18, 0.28);
    const air = p > 0.24 && p < 0.72 ? sin(((p - 0.24) / 0.48) * PI) : 0;
    const land = p > 0.7 ? sin(((p - 0.7) / 0.3) * PI) : 0;
    const arms = smooth((p - 0.12) / 0.15) - span(p, 0.85, 1) * 0.35;
    const armUp = -(PI * 0.88 * arms + 0.2 * (1 - arms));
    return {
      ...REST,
      rArmX: armUp - 0.1 * air,
      lArmX: armUp - 0.1 * air,
      rArmZ: -0.15 - 0.35 * arms - 0.15 * air,
      lArmZ: 0.15 + 0.35 * arms + 0.15 * air,
      lLegX: 0.35 * dip + 0.2 * air,
      rLegX: -0.1 * dip - 0.25 * air,
      crouch: 0.45 * dip + 0.3 * land,
      y: 4.2 * air - 0.6 * dip - 0.4 * land,
      headX: -0.28 * arms + 0.1 * dip,
      headZ: 0.06 * sin(t * 2.5),
      rotY: 0.08 * sin(t * 1.3),
      capeX: 0.18 + 0.7 * air,
    };
  },

  // Tanzen: drei Figuren im Wechsel (Seitschritt, Disco-Zeigen, Arme schwingen), im Takt.
  dance: (t) => {
    const bpm = 118 / 60;
    const beat = t * bpm * PI;
    const bounce = abs(sin(beat));
    const bar = (t * bpm) / 8;
    const figure = Math.floor(bar) % 3;
    const blendIn = span(bar % 1, 0, 0.12);
    const step = sin(beat);
    const poses: Pose[] = [
      // Seitschritt mit Armpumpen
      {
        ...REST,
        x: 1.1 * step,
        y: 0.8 * bounce,
        rotZ: -0.05 * step,
        rotY: 0.2 * step,
        rArmX: -1.2 - 0.5 * sin(beat * 2),
        lArmX: -1.2 - 0.5 * sin(beat * 2 + PI),
        rArmZ: -0.25,
        lArmZ: 0.25,
        lLegZ: 0.12 * max(0, step),
        rLegZ: 0.12 * min(0, step),
        headZ: 0.12 * step,
        headX: 0.08 * sin(beat * 2),
        capeX: 0.25 + 0.15 * bounce,
      },
      // Disco: rechter Arm zeigt hoch und runter
      {
        ...REST,
        y: 0.7 * bounce,
        rotY: -0.25 + 0.1 * step,
        rArmX: -(PI * 0.5 + 0.8 * step),
        rArmZ: -0.55 + 0.25 * step,
        lArmX: -0.1,
        lArmZ: 0.65,
        lLegX: 0.2 * max(0, step),
        rLegX: -0.2 * max(0, -step),
        headY: -0.2 - 0.15 * step,
        headX: -0.15 * step,
        x: 0.4 * step,
        capeX: 0.25 + 0.1 * bounce,
      },
      // Arme seitlich vor und zurück schwingen, Hüfte gegenläufig
      {
        ...REST,
        y: 0.6 * bounce,
        x: -0.8 * step,
        rotZ: 0.06 * step,
        rArmY: 0.9 * step,
        lArmY: 0.9 * step,
        rArmZ: -0.45,
        lArmZ: 0.45,
        rArmX: -0.25,
        lArmX: -0.25,
        headZ: -0.1 * step,
        capeX: 0.25 + 0.12 * bounce,
      },
    ];
    const prev = poses[(figure + 2) % 3];
    return mix(prev, poses[figure], blendIn);
  },
};

const FADE_S = 0.5;

function ease(x: number): number {
  return x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2;
}

function mix(a: Pose, b: Pose, w: number): Pose {
  const out = { ...a };
  for (const k of Object.keys(out) as (keyof Pose)[]) out[k] = a[k] + (b[k] - a[k]) * w;
  return out;
}

/** Gedämpfte Feder: folgt dem Ziel mit etwas Überschwingen. */
class Spring {
  value: number;
  private velocity = 0;
  constructor(
    initial: number,
    private stiffness: number,
    private damping: number,
  ) {
    this.value = initial;
  }
  step(target: number, dt: number): number {
    const accel = this.stiffness * (target - this.value) - this.damping * this.velocity;
    this.velocity += accel * dt;
    this.value += this.velocity * dt;
    return this.value;
  }
}

/** Treiber: hält Zeit, Überblendung, Blickrichtung und Federn; `apply` läuft in jedem Frame. */
export class SkinAnimator {
  private time = 0;
  private current: AnimationId;
  private previous: AnimationId | null = null;
  private fade = 1;
  private look = { x: 0, y: 0 };
  private lookTarget = { x: 0, y: 0 };
  private cape = new Spring(REST.capeX, 60, 9);
  private lastY = 0;
  private lastX = 0;
  private sway = new Spring(0, 90, 12);
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
    const dt = Math.min(delta, 0.05);
    this.time += dt;
    this.fade = Math.min(1, this.fade + dt / FADE_S);

    let pose = POSES[this.current](this.time);
    if (this.previous && this.fade < 1) pose = mix(POSES[this.previous](this.time), pose, ease(this.fade));
    if (this.intensity < 1) pose = mix(REST, pose, this.intensity);

    // Blick folgt dem Zeiger mit etwas Trägheit, relativ zur Blickrichtung der Figur.
    const k = 1 - Math.exp(-dt * 7);
    this.look.x += (this.lookTarget.x - this.look.x) * k;
    this.look.y += (this.lookTarget.y - this.look.y) * k;
    pose.headY += this.look.x * 0.7 - pose.rotY * 0.35 * (abs(this.look.x) > 0.05 ? 1 : 0);
    pose.headX += this.look.y * 0.4;

    // Umhang: Feder auf die Zielneigung, dazu Schwung aus Auf- und Seitwärtsbewegung.
    const vy = dt > 0 ? (pose.y - this.lastY) / dt : 0;
    const vx = dt > 0 ? (pose.x - this.lastX) / dt : 0;
    this.lastY = pose.y;
    this.lastX = pose.x;
    const capeX = this.cape.step(pose.capeX + clamp01(-vy * 0.04) * 0.6 + abs(vx) * 0.02, dt);
    // Leichtes Nachschwingen der Arme bei seitlichen Bewegungen.
    const armLag = this.sway.step(-vx * 0.012, dt);

    const s = player.skin;
    const c = pose.crouch;
    s.head.rotation.set(pose.headX, pose.headY, pose.headZ);
    s.head.position.set(0, -3.618 * c, 0);
    s.body.rotation.set(0.4538 * c, 0, 0);
    s.body.position.set(0, -6 - 2.104 * c, -2.124 * c);
    s.rightArm.rotation.set(pose.rArmX + 0.41 * c, pose.rArmY, pose.rArmZ + armLag);
    s.leftArm.rotation.set(pose.lArmX + 0.41 * c, pose.lArmY, pose.lArmZ + armLag);
    s.rightArm.position.set(-5, -2 - 2.539 * c, 0.168 * c);
    s.leftArm.position.set(5, -2 - 2.539 * c, 0.168 * c);
    s.rightLeg.rotation.set(pose.rLegX, 0, pose.rLegZ);
    s.leftLeg.rotation.set(pose.lLegX, 0, pose.lLegZ);
    s.rightLeg.position.set(-1.9, -12, -0.1 - 3.45 * c);
    s.leftLeg.position.set(1.9, -12, -0.1 - 3.45 * c);
    player.cape.position.set(0, 8 - 1.851 * c, -2 + 0.337 * c);
    player.cape.rotation.x = capeX + 0.294 * c;
    player.position.set(pose.x, pose.y, 0);
    player.rotation.set(pose.rotX, pose.rotY, pose.rotZ);
  }
}
