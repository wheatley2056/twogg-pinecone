// Nur für `npm run dev` im normalen Browser: ersetzt die Tauri-Befehle durch Beispieldaten,
// damit sich die Oberfläche ohne App-Fenster entwickeln und prüfen lässt.

import { api } from "./api";
import type { Account, AppEvent, AppUser, FriendsState, Participation, InstalledContent, LaunchEvent, Profile, ProfileInput, SavedSkin, SearchArgs, Settings, SyncMode, SyncPack, UpdateInfo } from "./types";

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const loginHandlers: ((r: { user?: AppUser; error?: string }) => void)[] = [];

let profiles: Profile[] = [
  { id: "fabric-test", name: "REVIVAL: Avalon", minecraft: "1.21.11", loader: { kind: "fabric", version: "0.19.5" }, memoryMb: 6144, jvmArgs: [], java: null, color: null, icon: "motif:shield", running: false, packSource: "twogg", lastPlayed: null },
  { id: "latest-2026", name: "LATEST 2026", minecraft: "1.21.11", loader: { kind: "fabric", version: "0.19.5" }, memoryMb: 4096, jvmArgs: [], java: null, color: null, icon: "motif:waves", running: false, packSource: "twogg", lastPlayed: null },
  { id: "survival", name: "Survival", minecraft: "26.2", loader: null, memoryMb: null, jvmArgs: [], java: null, color: null, icon: "motif:block", running: false, packSource: null, lastPlayed: null },
  { id: "quilt-1-21-1", name: "Bauteam-Welt", minecraft: "26.2", loader: { kind: "quilt", version: "0.30.1" }, memoryMb: null, jvmArgs: [], java: null, color: null, icon: "motif:house", running: false, packSource: null, lastPlayed: null },
  { id: "qol", name: "QoL Mods", minecraft: "26.2", loader: { kind: "fabric", version: "0.19.5" }, memoryMb: null, jvmArgs: [], java: null, color: null, icon: "motif:sliders", running: false, packSource: "modrinth", lastPlayed: null },
  { id: "retro", name: "Retro", minecraft: "1.8.9", loader: null, memoryMb: 2048, jvmArgs: [], java: null, color: null, icon: null, running: false, packSource: null, lastPlayed: null },
];
let accounts: Account[] = [{ uuid: "1a70aea0-11fc-356b-8f09-325a0158a7d0", name: "Tobi", kind: "offline", active: true }];
const stopped = new Set<string>();
let settings: Settings = {
  theme: { name: "rinde", accent: "#e9a23b", pattern: true, pixelTitles: true, reduceMotion: false },
  systemFrame: false,
  widgets: (["recent", "news", "packs", "servers", "screenshots", "playtime"] as const).map((id) => ({ id, visible: true })),
  language: "de",
};
const hour = 3600;
const now = Math.floor(Date.now() / 1000);
const stats: Record<string, { playtimeSecs: number; sessions: number; lastPlayed: number | null; lastServer: null }> = {
  "fabric-test": { playtimeSecs: 14 * hour + 1200, sessions: 23, lastPlayed: now - 2 * hour, lastServer: null },
  survival: { playtimeSecs: 30 * hour, sessions: 40, lastPlayed: now - 26 * hour, lastServer: null },
  "quilt-1-21-1": { playtimeSecs: 9 * hour, sessions: 12, lastPlayed: now - 3 * 86400, lastServer: null },
  qol: { playtimeSecs: 2 * hour, sessions: 3, lastPlayed: now - 5 * 86400, lastServer: null },
  retro: { playtimeSecs: 3 * hour, sessions: 5, lastPlayed: now - 9 * 86400, lastServer: null },
};
const SODIUM_ICON = "https://cdn.modrinth.com/data/AANobbMI/295862f4724dc3f78df3447ad6072b2dcd3ef0c9_96.webp";
let content: InstalledContent[] = [
  { key: "mods/fabric-api.jar", fileName: "fabric-api.jar", kind: "mod", enabled: true, name: "Fabric API", version: "0.162.0+26.3",
    source: { type: "modrinth", project_id: "P7dR8mSH", version_id: "a" }, iconUrl: null, fromPack: false, sync: null, size: 1, unidentified: false },
  { key: "mods/sodium.jar", fileName: "sodium.jar", kind: "mod", enabled: true, name: "Sodium", version: "mc26.3-0.9.2-fabric",
    source: { type: "modrinth", project_id: "AANobbMI", version_id: "b" }, iconUrl: SODIUM_ICON, fromPack: false, sync: "basics", size: 1, unidentified: false },
  { key: "mods/eigene-mod.jar", fileName: "eigene-mod.jar.disabled", kind: "mod", enabled: false, name: "eigene-mod", version: null,
    source: { type: "local" }, iconUrl: null, fromPack: false, sync: null, size: 1, unidentified: false },
  { key: "shaderpacks/complementary.zip", fileName: "complementary.zip", kind: "shader", enabled: true, name: "Complementary Shaders - Reimagined",
    version: "r5.9.3", source: { type: "modrinth", project_id: "HVnmMxH1", version_id: "c" }, iconUrl: null, fromPack: false, sync: null, size: 1, unidentified: false },
];
const update: UpdateInfo = { key: "mods/sodium.jar", name: "Sodium", current: "mc26.3-0.9.2-fabric", latest: "mc26.3-0.9.4-fabric",
  projectId: "AANobbMI", versionId: "d", kind: "mod" };

// Kleines grünes Quadrat als Server-Icon (PNG, 2×2).
const ICON = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAEklEQVR4nGPwWR/gsz6AAUIBACQyBS0TyzOPAAAAAElFTkSuQmCC";

let syncPacks: SyncPack[] = [
  {
    id: "basics", name: "Basics", mode: "sync",
    entries: [
      { type: "modrinth", projectId: "w7ThoJFB", kind: "mod", name: "Zoomify" },
      { type: "modrinth", projectId: "AANobbMI", kind: "mod", name: "Sodium" },
      { type: "modrinth", projectId: "50dA9Sha", kind: "resourcepack", name: "Fresh Animations" },
      { type: "file", path: "config/zoomify.json", sha1: "0" },
    ],
    options: { fov: "0.25" }, auto: { enabled: true, loaders: ["fabric", "quilt"] },
  },
  { id: "shader-setup", name: "Shader-Setup", mode: "template", entries: [{ type: "modrinth", projectId: "HVnmMxH1", kind: "shader", name: "Complementary Shaders" }], options: {}, auto: { enabled: false, loaders: [] } },
];
const syncEnabled = new Set(["fabric-test/basics"]);

// Einfarbiger 64×32-Platzhalter-Skin (Kopf blau, Gesicht hautfarben), als PNG erzeugt.
const STEVE = (() => {
  const c = document.createElement("canvas");
  c.width = 64;
  c.height = 32;
  const g = c.getContext("2d")!;
  g.fillStyle = "#3b6fb6"; g.fillRect(0, 16, 64, 16);
  g.fillStyle = "#c69c7a"; g.fillRect(0, 0, 32, 16);
  g.fillStyle = "#4a3222"; g.fillRect(8, 8, 8, 2); g.fillRect(0, 0, 32, 8);
  g.fillStyle = "#ffffff"; g.fillRect(9, 12, 2, 1); g.fillRect(13, 12, 2, 1);
  return c.toDataURL("image/png");
})();
// Beispiel-Screenshots: farbige Verläufe als data:-URL (im Browser ohne Tauri).
function shotImage(hue: number) {
  const c = document.createElement("canvas");
  c.width = 480;
  c.height = 270;
  const g = c.getContext("2d")!;
  const grad = g.createLinearGradient(0, 0, 0, 270);
  grad.addColorStop(0, `hsl(${hue} 70% 70%)`);
  grad.addColorStop(0.6, `hsl(${hue + 20} 50% 45%)`);
  grad.addColorStop(1, "hsl(110 40% 30%)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 480, 270);
  g.fillStyle = "hsl(110 45% 28%)";
  for (let x = 0; x < 480; x += 24) g.fillRect(x, 190 + ((x * 7) % 30), 24, 90);
  return c.toDataURL("image/png");
}
let mockShots = [200, 30, 260, 180, 320, 90].map((hue, i) => ({
  profileId: i % 2 ? "retro" : "fabric-test",
  profileName: i % 2 ? "Retro" : "Fabric Test",
  path: shotImage(hue),
  modified: now - i * 7000 - (i > 3 ? 86400 : 0),
  size: 1000,
}));
const TWOGG_PACK = {
  id: "horizons-unbound", title: "Horizons Unbound", summary: "Das Modpack für unseren Survival-Server", iconUrl: null, featured: true,
  updatedAt: now - 86400, downloads: 128,
  latest: { id: "v2", version: "2.0.0", minecraft: "1.20.1", loader: "fabric", loaderVersion: "0.19.5", channel: "release",
    changelogHtml: "<ul><li>Neue Farmwelt</li><li>Create 6</li></ul>", size: 2_400_000, sha1: "0", publishedAt: now - 86400 },
};
let mockSkins: SavedSkin[] = [
  { id: "1", name: "Tobi", model: "classic", created: now, updated: now },
  { id: "2", name: "Winter", model: "slim", created: now - 100, updated: now - 100 },
];
function addMockSkin(name: string, model: "classic" | "slim" = "classic") {
  const s = { id: String(Date.now()), name, model, created: now, updated: now };
  mockSkins = [s, ...mockSkins];
  return s;
}

function preview(token: string, source: "modrinth" | "curseforge" | "file") {
  return {
    token,
    source,
    preview: {
      name: "Fabulously Optimized", version: "14.1.0", summary: "Improve your graphics and performance with this simple modpack.",
      minecraft: "26.2", loader: { kind: "fabric" as const, version: "0.19.5" }, mods: 49, resourcepacks: 3, shaders: 0, otherFiles: 49,
      downloadSize: 44_900_000, archiveSize: 210_000, missing: [], blockedHosts: [],
    },
  };
}

function toProfile(id: string, input: ProfileInput): Profile {
  return { id, running: false, ...input, loader: input.loader && { ...input.loader, version: input.loader.version || "0.19.5" } };
}

// Echte Events von twogg.app (gekürzt) plus REVIVAL aus dem Design-System.
const APP_EVENTS: AppEvent[] = [
  { id: "revival-avalon", title: "REVIVAL: Avalon", summary: "Mittelalter-RP-Sandbox im Land Avalon. Schließ dich einer Seite an – Die Treue Hand oder der Phönixbund – und schreib die Geschichte mit.",
    eventType: "ROLEPLAY", eventDate: null, registrationDeadline: null, registrationClosed: false, requiresApplication: true, minecraftVersion: "1.21.11",
    maxPlayers: null, teamSize: null, logoUrl: null, bannerUrl: null, ended: false, url: "https://twogg.app/events" },
  { id: "ca7acfe0-fc10-44e4-a477-96b64c670631", title: "WINTERLAND: Icebreaker", summary: "", eventType: "ROLEPLAY", eventDate: "2026-11-28T17:00:00.000Z",
    registrationDeadline: null, registrationClosed: false, requiresApplication: true, minecraftVersion: "1.21.11", maxPlayers: 32, teamSize: 2,
    logoUrl: null, bannerUrl: null, ended: false, url: "https://twogg.app/events/ca7acfe0-fc10-44e4-a477-96b64c670631" },
  { id: "94181fb6-d005-45f2-80d2-0216826ffd65", title: "LATEST 2026 RISE OF ATLANTIS",
    summary: "LATEST ist das jährliche Aushängeschild-Event von TwoGG: ein Minecraft-PvP-Survival im Varo-Format, bei dem ihr euch zu zweit gegen alle anderen Teams behauptet.",
    eventType: "TEAM", eventDate: "2026-10-03T17:00:00.000Z", registrationDeadline: "2026-10-03T16:45:00.000Z", registrationClosed: false, requiresApplication: false,
    minecraftVersion: "1.21.11", maxPlayers: 50, teamSize: 2, logoUrl: null, bannerUrl: null, ended: true, url: "https://twogg.app/events/94181fb6-d005-45f2-80d2-0216826ffd65" },
];
let appUser: AppUser | null = null;

const now2 = Math.floor(Date.now() / 1000);
const mockFriends: FriendsState = {
  available: true,
  friends: [
    { uuid: "a1", name: "Kalle", online: true, lastSeen: now2, activity: { instance: "REVIVAL: Avalon", version: "Fabric 1.21.11", server: "mc.twogg.de", playing: true } },
    { uuid: "a2", name: "MaraBaut", online: true, lastSeen: now2, activity: { instance: null, version: null, server: null, playing: false } },
    { uuid: "a3", name: "Rex", online: false, lastSeen: now2 - 7200, activity: null },
  ],
  incoming: [{ uuid: "a4", name: "Lina" }],
  outgoing: [],
};

Object.assign(api, {
  worlds: async (id: string) =>
    id === "fabric-test"
      ? [
          { folder: "Avalon", name: "Avalon – Hauptwelt", gameMode: "survival", hardcore: false, lastPlayed: Date.now() - 2 * 3600_000, version: "1.21.11", size: 412_000_000, icon: null },
          { folder: "Bauplatz", name: "Bauplatz", gameMode: "creative", hardcore: false, lastPlayed: Date.now() - 5 * 86400_000, version: "1.21.11", size: 38_000_000, icon: null },
        ]
      : [],
  datapackInstall: async (_id: string, _world: string, project: string) => (await wait(600), `${project}.zip`),
  worldBackup: async (_id: string, folder: string) => `/home/beispiel/.local/share/twogg-pinecone/backups/worlds/${folder}.zip`,
  worldDelete: async () => {},
  worldOpenFolder: async () => {},
  savedServers: async () => [
    { name: "TwoGG Community", address: "mc.twogg.de", icon: null },
    { name: "Avalon (Event)", address: "avalon.twogg.de", icon: null },
  ],
  serverInfo: async () => ({ gameServer: { name: "TwoGG Server", address: "mc.twogg.de" } }),
  friends: async () => mockFriends,
  friendsPresence: async () => {},
  friendRequest: async (name: string) => (mockFriends.outgoing!.push({ uuid: name, name }), "requested"),
  friendAccept: async (uuid: string) => {
    const p = mockFriends.incoming!.find((x) => x.uuid === uuid);
    mockFriends.incoming = mockFriends.incoming!.filter((x) => x.uuid !== uuid);
    if (p) mockFriends.friends!.push({ ...p, online: false, lastSeen: now2 - 60, activity: null });
  },
  friendDecline: async (uuid: string) => {
    mockFriends.incoming = mockFriends.incoming!.filter((x) => x.uuid !== uuid);
    mockFriends.outgoing = mockFriends.outgoing!.filter((x) => x.uuid !== uuid);
  },
  friendRemove: async (uuid: string) => void (mockFriends.friends = mockFriends.friends!.filter((x) => x.uuid !== uuid)),
  appEvents: async () => (await wait(150), APP_EVENTS),
  appMe: async () => appUser,
  appLoginDiscord: async () => {
    appUser = { id: "u1", username: "wheatley", avatarUrl: null, mcUsername: "wheatley", mcUuid: null };
    loginHandlers.forEach((h) => h({ user: appUser! }));
  },
  appLoginPassword: async () => (appUser = { id: "u1", username: "wheatley", avatarUrl: null, mcUsername: "wheatley", mcUuid: null }),
  appLogout: async () => void (appUser = null),
  appParticipations: async (): Promise<Participation[]> => (appUser ? [{ eventId: "revival-avalon", kind: "pre", status: "pending", teamName: null }] : []),
  appJoin: async () => ({ approved: false, serverAddress: null }),
  appFriends: async () => [],
  onAppLogin: async (h: (r: { user?: AppUser; error?: string }) => void) => {
    loginHandlers.push(h);
    return () => {};
  },
  appInfo: async () => ({ version: "0.1.0", dataDir: "/home/beispiel/.local/share/twogg-pinecone", msaConfigured: false, curseforgeConfigured: true, updatesConfigured: true }),
  versions: async (snapshots: boolean) => [
    ...(snapshots ? [{ id: "26.4-snapshot-3", kind: "snapshot", releaseTime: "2026-10-08" }] : []),
    ...["26.3", "26.2", "26.1.2", "1.21.1", "1.20.1", "1.12.2", "1.8.9"].map((id) => ({ id, kind: "release", releaseTime: "" })),
  ],
  loaderVersions: async (kind: string, mc: string) => {
    await wait(300);
    if (mc === "1.8.9") return [];
    return kind === "fabric"
      ? [{ version: "0.19.5", stable: true }, { version: "0.19.4", stable: false }]
      : [{ version: "0.30.1", stable: true }, { version: "0.31.0-beta.4", stable: false }];
  },
  profiles: async () => profiles.map((p) => ({ ...p, lastPlayed: stats[p.id]?.lastPlayed ?? null })),
  createProfile: async (input: ProfileInput) => {
    const p = toProfile(input.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"), input);
    profiles = [...profiles, p];
    return p;
  },
  updateProfile: async (id: string, input: ProfileInput) => {
    const p = toProfile(id, input);
    profiles = profiles.map((x) => (x.id === id ? p : x));
    return p;
  },
  deleteProfile: async (id: string) => void (profiles = profiles.filter((p) => p.id !== id)),
  openProfileFolder: async () => {},
  lastLog: async () => [],
  accounts: async () => accounts,
  addOfflineAccount: async (name: string) => {
    const a: Account = { uuid: crypto.randomUUID(), name, kind: "offline", active: true };
    accounts = [...accounts.map((x) => ({ ...x, active: false })), a];
    return a;
  },
  setActiveAccount: async (uuid: string) => void (accounts = accounts.map((a) => ({ ...a, active: a.uuid === uuid }))),
  removeAccount: async (uuid: string) => void (accounts = accounts.filter((a) => a.uuid !== uuid)),
  javaList: async () => [
    { path: "/usr/lib/jvm/java-8-openjdk/jre/bin/java", version: "1.8.0_504", major: 8, managed: false },
    { path: "/usr/bin/java", version: "25.0.4.1", major: 25, managed: false },
  ],
  stopGame: async (id: string) => void stopped.add(id),
  dashboard: async () => ({
    recent: profiles
      .map((profile) => ({ profile, stats: stats[profile.id] ?? { playtimeSecs: 0, sessions: 0, lastPlayed: null, lastServer: null } }))
      .sort((a, b) => (b.stats.lastPlayed ?? 0) - (a.stats.lastPlayed ?? 0)),
    servers: [
      { profileId: "fabric-test", profileName: "REVIVAL: Avalon", name: "TwoGG Community", address: "mc.twogg.de", icon: ICON, last: true },
      { profileId: "fabric-test", profileName: "REVIVAL: Avalon", name: "Hypixel", address: "mc.hypixel.net", icon: null, last: false },
      { profileId: "retro", profileName: "Retro", name: "Alter Server", address: "old.example.org:25570", icon: null, last: false },
    ],
    screenshots: [],
    totalPlaytimeSecs: Object.values(stats).reduce((a, s) => a + s.playtimeSecs, 0),
  }),
  pingServer: async (address: string) => {
    await wait(400 + Math.random() * 600);
    if (address.startsWith("old.")) throw "Server nicht erreichbar";
    return address.includes("hypixel")
      ? { motd: "Hypixel Network [1.8/26.3]\nHALLOWEEN EVENT", version: "Requires MC 1.8 / 1.21", protocol: 47, online: 34858, max: 200000, pingMs: 106, favicon: null }
      : { motd: "TwoGG Community – Survival & Minigames", version: "Paper 26.3", protocol: 767, online: 7, max: 50, pingMs: 12, favicon: null };
  },
  openScreenshot: async () => {},
  contentList: async () => content,
  contentIdentify: async () => content,
  contentSetEnabled: async (_id: string, key: string, enabled: boolean) =>
    void (content = content.map((c) => (c.key === key ? { ...c, enabled } : c))),
  contentRemove: async (_id: string, key: string) => void (content = content.filter((c) => c.key !== key)),
  contentAddLocal: async () => "mods/neu.jar",
  contentInstall: async (_id: string, _kind: string, project: string) => {
    await wait(700);
    return { installed: [[project === "AANobbMI" ? "Sodium" : "Iris Shaders", "1.0"]], alreadyPresent: [], warnings: [] };
  },
  contentUpdates: async () => (await wait(500), content.some((c) => c.key === update.key && c.version !== update.latest) ? [update] : []),
  contentUpdate: async () => {
    await wait(600);
    content = content.map((c) => (c.key === update.key ? { ...c, version: update.latest } : c));
    return { installed: [["Sodium", update.latest]], alreadyPresent: [], warnings: [] };
  },
  modrinthSearch: async (args: SearchArgs) => {
    await wait(250);
    const all = [
      { projectId: "AANobbMI", slug: "sodium", projectType: "mod", title: "Sodium", author: "jellysquid3", iconUrl: SODIUM_ICON,
        description: "A high-performance rendering engine replacement for Minecraft, which greatly improves frame rates and reduces micro-stutter.",
        downloads: 240498978, follows: 30000, displayCategories: ["optimization", "fabric", "quilt"], versions: ["26.3"], dateModified: "2026-09-20T21:27:06Z" },
      { projectId: "YL57xq9U", slug: "iris", projectType: "mod", title: "Iris Shaders", author: "coderbot", iconUrl: null,
        description: "A modern shader pack loader for Minecraft intended to be compatible with existing OptiFine shader packs",
        downloads: 98000000, follows: 20000, displayCategories: ["decoration", "optimization"], versions: ["26.3"], dateModified: "2026-10-01T10:00:00Z" },
      { projectId: "P7dR8mSH", slug: "fabric-api", projectType: "mod", title: "Fabric API", author: "modmuss50", iconUrl: null,
        description: "Lightweight and modular API providing common hooks and intercompatibility measures utilized by mods using the Fabric toolchain.",
        downloads: 150000000, follows: 25000, displayCategories: ["library"], versions: ["26.3"], dateModified: "2026-10-05T10:00:00Z" },
    ];
    const q = args.query.toLowerCase();
    const hits = all.filter((h) => !q || h.title.toLowerCase().includes(q));
    return { hits, offset: 0, limit: 20, totalHits: hits.length };
  },
  modrinthVersions: async () => (await wait(300), [
    { id: "v1", projectId: "AANobbMI", name: "Sodium", versionNumber: "mc26.3-0.9.3-alpha.1-fabric", versionType: "alpha", loaders: ["fabric"], gameVersions: ["26.3"], datePublished: "" },
    { id: "v2", projectId: "AANobbMI", name: "Sodium", versionNumber: "mc26.3-0.9.2-fabric", versionType: "release", loaders: ["fabric"], gameVersions: ["26.3"], datePublished: "" },
  ]),
  modrinthCategories: async () => [
    { name: "optimization", projectType: "mod", header: "categories" },
    { name: "technology", projectType: "mod", header: "categories" },
    { name: "magic", projectType: "mod", header: "categories" },
    { name: "library", projectType: "mod", header: "categories" },
    { name: "realistic", projectType: "shader", header: "categories" },
    { name: "potato", projectType: "shader", header: "performance impact" },
  ],
  packInfo: async (id: string) =>
    id === "fabric-test" ? { name: "Avalon Event-Pack", version: "1.4.0", source: "twogg" } : id === "qol" ? { name: "Fabulously Optimized", version: "6.3.4", source: "modrinth" } : null,
  packCheckUpdate: async () => (await wait(400), { current: "6.3.4", latest: "6.5.0", versionId: "N276l2ON" }),
  packUpdatesAll: async () => (await wait(600), [{ profile: profiles[0], update: { current: "6.3.4", latest: "6.5.0", versionId: "N276l2ON" } }]),
  packPlanUpdate: async () => (await wait(500), {
    packName: "Fabulously Optimized", fromVersion: "6.3.4", toVersion: "6.5.0", minecraft: "1.21.1",
    loader: { kind: "fabric", version: "0.19.3" },
    changes: [
      { path: "mods/sodium-fabric-0.6.13.jar", action: "add" },
      { path: "config/yosbr/options.txt", action: "replace" },
      { path: "mods/ImmediatelyFast-Fabric-1.6.5+1.21.1.jar", action: "remove" },
    ],
    conflicts: [
      { path: "config/fabric_loader_dependencies.json", reason: "bothChanged", default: "keepMine" },
      { path: "mods/eigene-sodium.jar", reason: "userFileExists", default: "takePack" },
    ],
    kept: ["config/modmenu.json"],
  }),
  packApplyUpdate: async () => (await wait(800), { id: "1791667261", created: now, packVersion: "6.3.4", files: 93 }),
  packSnapshots: async () => [{ id: "1791660000", created: now - 86400, packVersion: "6.3.3", files: 12 }],
  packRollback: async () => {},
  packExport: async () => (await wait(400), { path: "/tmp/x.mrpack", linked: 48, included: 7 }),
  packPreviewFile: async (path: string) => (await wait(300), preview(path, "file")),
  packPreviewLink: async (link: string) => {
    await wait(600);
    if (!link.startsWith("pinecone://") && !link.startsWith("https://modrinth.com/")) throw `Kein gültiger Modpack-Link: ${link}`;
    return preview(link, link.includes("curseforge") ? "curseforge" : "modrinth");
  },
  packPreviewCancel: async () => {},
  packImportPending: async () => (await wait(800), { profile: profiles[0], missing: [] }),
  packShareLink: async (id: string) => (id === "fabric-test" ? "pinecone://modrinth/1KVo5zza/ssWn7YI0" : null),
  migratePlan: async (id: string, target: { minecraft: string; loader: "fabric" | "quilt" | null }) => {
    await wait(700);
    const p = profiles.find((x) => x.id === id)!;
    if (p.minecraft === target.minecraft && (p.loader?.kind ?? null) === target.loader) throw "Das Profil hat schon diese Version und diesen Loader";
    const mods = target.loader !== null;
    return {
      fromMinecraft: p.minecraft, fromLoader: p.loader, minecraft: target.minecraft,
      loader: target.loader ? { kind: target.loader, version: target.loader === "fabric" ? "0.19.5" : "0.30.1" } : null,
      items: [
        { key: "mods/fabric-api.jar", name: "Fabric API", kind: "mod", status: mods ? "update" : "missing", fromVersion: "0.162.0+26.3", toVersion: mods ? "0.116.17+1.21.1" : null, substitute: null },
        { key: "mods/sodium.jar", name: "Sodium", kind: "mod", status: mods ? "update" : "missing", fromVersion: "mc26.3-0.9.2-fabric", toVersion: mods ? "mc1.21.1-0.8.13-fabric" : null, substitute: null },
        { key: "mods/eigene-mod.jar", name: "eigene-mod", kind: "mod", status: "missing", fromVersion: null, toVersion: null, substitute: null },
        { key: "mods/qfapi.jar", name: "Quilted Fabric API", kind: "mod", status: mods ? "substitute" : "missing", fromVersion: "11.0.0", toVersion: null, substitute: "Fabric API" },
        { key: "shaderpacks/complementary.zip", name: "Complementary Shaders - Reimagined", kind: "shader", status: "compatible", fromVersion: "r5.9.3", toVersion: null, substitute: null },
        { key: "resourcepacks/eigenes.zip", name: "eigenes", kind: "resourcepack", status: "kept", fromVersion: null, toVersion: null, substitute: null },
      ],
      packOffer: id === "fabric-test" ? { name: "Fabulously Optimized", platform: "modrinth", projectId: "1KVo5zza", versionId: "N276l2ON", version: "6.5.0" } : null,
      worlds: 2,
      downgrade: target.minecraft < p.minecraft,
    };
  },
  migrateApply: async (id: string, options: { target: { minecraft: string } }) => {
    await wait(1200);
    const p = profiles.find((x) => x.id === id)!;
    const copy = { ...p, id: `${id}-neu`, name: `${p.name} (${options.target.minecraft})`, minecraft: options.target.minecraft };
    profiles = [...profiles, copy];
    return { profile: copy, warnings: [] };
  },
  syncpackList: async () => syncPacks,
  syncpackCreate: async (name: string, mode: SyncMode) => {
    const pack: SyncPack = { id: name.toLowerCase().replace(/[^a-z0-9]+/g, "-"), name, mode, entries: [], options: {}, auto: { enabled: false, loaders: [] } };
    syncPacks = [...syncPacks, pack];
    return pack;
  },
  syncpackSave: async (pack: SyncPack) => ((syncPacks = syncPacks.map((p) => (p.id === pack.id ? pack : p))), pack),
  syncpackDelete: async (id: string) => void (syncPacks = syncPacks.filter((p) => p.id !== id)),
  syncpackAddFile: async (id: string, _path: string, target: string) => {
    const pack = syncPacks.find((p) => p.id === id)!;
    pack.entries = [...pack.entries, { type: "file", path: target, sha1: "0" }];
    return pack;
  },
  syncpackAddFromProfile: async (id: string) => syncPacks.find((p) => p.id === id)!,
  syncpackForProfile: async (profileId: string) => syncPacks.map((pack) => ({ pack, enabled: syncEnabled.has(`${profileId}/${pack.id}`) })),
  syncpackSetEnabled: async (profileId: string, id: string, enabled: boolean) => {
    await wait(500);
    const template = syncPacks.find((p) => p.id === id)?.mode === "template";
    if (enabled && !template) syncEnabled.add(`${profileId}/${id}`);
    else syncEnabled.delete(`${profileId}/${id}`);
    return enabled
      ? { installed: ["Zoomify", "Fresh Animations"], removed: [], present: ["Sodium"], unavailable: [], warnings: [] }
      : { installed: [], removed: ["Zoomify", "Fresh Animations"], present: [], unavailable: [], warnings: [] };
  },
  syncpackSync: async () => ({ installed: [], removed: [], present: [], unavailable: [], warnings: [] }),
  updateCheck: async () => (await wait(500), { version: "0.2.0", currentVersion: "0.1.0", date: now - 3600, notes: "- Skins und Screenshots\n- Sync-Packs\n- Versionswechsel" }),
  updateInstall: async (onEvent: (e: { type: "progress"; done: number; total: number | null } | { type: "installing" }) => void) => {
    for (let done = 0; done <= 25_000_000; done += 2_500_000) {
      onEvent({ type: "progress", done, total: 25_000_000 });
      await wait(120);
    }
    onEvent({ type: "installing" });
    await wait(800);
    throw "Mock-Modus: kein echtes Update";
  },
  diagnosticsExport: async (path: string) => (await wait(600), { path, files: 12, bytes: 48_000 }),
  openLogsFolder: async () => {},
  screenshotsList: async () => mockShots,
  screenshotThumb: async (path: string) => path,
  screenshotDelete: async (path: string) => void (mockShots = mockShots.filter((s) => s.path !== path)),
  screenshotReveal: async () => {},
  screenshotSaveAs: async () => {},
  onScreenshotsChanged: async () => () => {},
  twoggPacks: async () => (await wait(300), [TWOGG_PACK]),
  twoggPack: async () => (await wait(300), { ...TWOGG_PACK, descriptionHtml: "<h2>Willkommen</h2><p>Alles für <strong>Horizons Unbound</strong>: Create, Farmwelt und mehr.</p>", versions: [TWOGG_PACK.latest, { ...TWOGG_PACK.latest, id: "v1", version: "1.9.0", channel: "release", publishedAt: now - 30 * 86400 }] }),
  twoggNews: async () => (await wait(300), [
    { id: "n1", title: "Pinecone ist da!", bodyHtml: "<p>Der neue <strong>TwoGG-Launcher</strong> ist da. Modpacks, Skins und mehr an einem Ort.</p>", imageUrl: null, link: "https://twogg.de", pinned: true, publishedAt: now - 3600 },
    { id: "n2", title: "Farmwelt-Reset am Sonntag", bodyHtml: "<p>Wie jede Woche um 4 Uhr.</p>", imageUrl: null, link: null, pinned: false, publishedAt: now - 3 * 86400 },
  ]),
  packInstallTwogg: async () => (await wait(800), { profile: profiles[0], missing: [] }),
  skinsList: async () => mockSkins,
  skinData: async () => STEVE,
  skinImportFile: async () => addMockSkin("Datei-Skin"),
  skinImportPlayer: async (player: string) => (await wait(500), addMockSkin(player)),
  skinSave: async (id: string | null, name: string, model: "classic" | "slim") =>
    id ? Object.assign(mockSkins.find((s) => s.id === id)!, { name, model }) : addMockSkin(name, model),
  skinRename: async (id: string, name: string) => Object.assign(mockSkins.find((s) => s.id === id)!, { name }),
  skinSetModel: async (id: string, model: "classic" | "slim") => Object.assign(mockSkins.find((s) => s.id === id)!, { model }),
  skinDelete: async (id: string) => void (mockSkins = mockSkins.filter((s) => s.id !== id)),
  skinExport: async () => {},
  skinAccount: async () => { throw "Skins und Capes lassen sich nur mit einem Microsoft-Account ändern."; },
  skinApply: async () => { throw "Microsoft-Login ist nicht eingerichtet"; },
  skinSetCape: async () => { throw "Microsoft-Login ist nicht eingerichtet"; },
  skinImportAccount: async () => addMockSkin("Account"),
  takeOpenRequests: async () => [],
  onOpenPack: async () => () => {},
  packInstallModrinth: async () => (await wait(800), { profile: profiles[0], missing: [] }),
  packInstallCurseforge: async () => (await wait(800), {
    profile: profiles[0],
    missing: [{ name: "OptiFine-Ersatz", path: "mods/beispiel.jar", url: "https://www.curseforge.com/minecraft/mc-mods/beispiel/files/1" }],
  }),
  curseforgeSearch: async () => (await wait(250), { offset: 0, limit: 20, totalHits: 1, hits: [
    { projectId: "238222", slug: "jei", projectType: "mod", title: "Just Enough Items (JEI)", author: "mezz", iconUrl: null,
      description: "View Items and Recipes", downloads: 400000000, follows: 0, displayCategories: ["API and Library"], versions: [], dateModified: "2026-10-01T00:00:00Z" },
  ] }),
  curseforgeFiles: async () => [],
  contentInstallCurseforge: async () => ({ installed: [["Just Enough Items (JEI)", "jei-1.0.jar"]], alreadyPresent: [], warnings: [] }),
  getSettings: async () => settings,
  saveSettings: async (s: Settings) => (settings = s),
  exportTheme: async () => {},
  importTheme: async () => settings,
  launch: async (id: string, onEvent: (e: LaunchEvent) => void) => {
    stopped.delete(id);
    for (const stage of ["loader", "version"] as const) {
      onEvent({ type: "stage", stage });
      await wait(400);
    }
    onEvent({ type: "progress", done: 0, files: 3812, bytesDone: 0, bytes: 812 * 1024 * 1024 });
    for (let done = 0; done <= 3812; done += 190) {
      onEvent({ type: "progress", done: Math.min(done, 3812), files: 3812, bytesDone: (done / 3812) * 812 * 1024 * 1024, bytes: 0 });
      await wait(80);
    }
    onEvent({ type: "stage", stage: "java" });
    onEvent({ type: "java", version: "25.0.4.1", path: "/usr/bin/java" });
    onEvent({ type: "stage", stage: "assets" });
    await wait(300);
    onEvent({ type: "started", pid: 4242 });
    (async () => {
      const sample = [
        "[22:11:28] [main/INFO]: Loading 49 mods:",
        "\t- fabric-api 0.162.0+26.3",
        "\t- fabricloader 0.19.5",
        "[22:11:32] [Render thread/INFO]: Setting user: Tobi",
        "[22:11:32] [Download-2/ERROR]: Failed to fetch user properties",
        "[22:11:32] [Render thread/WARN]: Error while loading the narrator",
        "[22:11:33] [Render thread/INFO]: Sound engine started",
        "[22:11:33] [Render thread/INFO]: Created: 2048x2048x4 minecraft:textures/atlas/blocks.png-atlas",
      ];
      for (const line of sample) {
        if (stopped.has(id)) break;
        onEvent({ type: "log", lines: [line] });
        await wait(250);
      }
      while (!stopped.has(id)) await wait(200);
      onEvent({ type: "exited", code: 143, success: false });
    })();
  },
});

console.info("TwoGG Pinecone: Mock-Modus (kein Tauri)");
