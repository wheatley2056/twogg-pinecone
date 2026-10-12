// Spiegel der DTOs aus src-tauri/src/dto.rs.

export type LoaderKind = "fabric" | "quilt" | "forge" | "neoforge";
export type AccountKind = "offline" | "microsoft";

export interface AppInfo {
  version: string;
  dataDir: string;
  msaConfigured: boolean;
  updatesConfigured: boolean;
  curseforgeConfigured: boolean;
}

export interface VersionInfo {
  id: string;
  kind: string;
  releaseTime: string;
}

export interface LoaderVersion {
  version: string;
  stable: boolean;
}

export interface LoaderSpec {
  kind: LoaderKind;
  version: string;
}

export interface Profile {
  id: string;
  name: string;
  minecraft: string;
  loader: LoaderSpec | null;
  memoryMb: number | null;
  jvmArgs: string[];
  /** JVM-Preset der Instanz; `null` = globaler Standard. */
  jvmPreset?: string | null;
  java: string | null;
  color: string | null;
  icon: string | null;
  running: boolean;
  /** Herkunft, falls aus einem Modpack; nur in der Profil-Liste gesetzt. */
  packSource?: "twogg" | "modrinth" | "curseforge" | "file" | null;
  /** Pack-ID bei TwoGG-Packs. */
  packId?: string | null;
  lastPlayed?: number | null;
  /** Gesamte Spielzeit in Sekunden; nur in der Liste. */
  playtime?: number | null;
  /** Eigene Gruppe in der Bibliothek; `null` = automatisch. */
  group?: string | null;
  /** Geänderte Welten vor jedem Start sichern. */
  backupOnLaunch: boolean;
  /** Automatische Backups je Welt, die behalten werden. */
  backupKeep: number;
  /** UUID des festen Kontos; `null` = aktives Konto. */
  account?: string | null;
  /** Start-Befehle (F-55). */
  preLaunch?: string | null;
  wrapper?: string | null;
  postExit?: string | null;
}

export interface ProfileInput {
  name: string;
  minecraft: string;
  loader: LoaderSpec | null;
  memoryMb: number | null;
  jvmArgs: string[];
  jvmPreset: string | null;
  java: string | null;
  color: string | null;
  icon: string | null;
}

export interface Account {
  uuid: string;
  name: string;
  kind: AccountKind;
  active: boolean;
}

export interface LoginStart {
  userCode: string;
  verificationUri: string;
  expiresIn: number;
}

export interface JavaInstallation {
  path: string;
  version: string;
  major: number;
  managed: boolean;
}

export type Stage = "backup" | "prelaunch" | "sync" | "loader" | "version" | "java" | "assets";

export type LaunchEvent =
  | { type: "stage"; stage: Stage }
  | { type: "progress"; done: number; files: number; bytesDone: number; bytes: number }
  | { type: "java"; version: string; path: string }
  | { type: "started"; pid: number | null }
  | { type: "log"; lines: string[] }
  | { type: "exited"; code: number | null; success: boolean };

export interface ProfileStats {
  playtimeSecs: number;
  sessions: number;
  lastPlayed: number | null;
  lastServer: { host: string; port: number | null } | null;
}

export interface ServerEntry {
  profileId: string;
  profileName: string;
  name: string;
  address: string;
  icon: string | null;
  last: boolean;
}

export interface Screenshot {
  profileId: string;
  path: string;
  modified: number;
}

export interface Dashboard {
  recent: { profile: Profile; stats: ProfileStats }[];
  servers: ServerEntry[];
  screenshots: Screenshot[];
  totalPlaytimeSecs: number;
}

export interface ServerStatus {
  motd: string;
  version: string;
  protocol: number;
  online: number;
  max: number;
  pingMs: number;
  favicon: string | null;
}

export type ThemeName = "rinde" | "kiefer" | "winterland";
export type WidgetId = "recent" | "news" | "packs" | "servers" | "screenshots" | "playtime";

export interface Settings {
  theme: { name: ThemeName; accent: string; pattern: boolean; pixelTitles: boolean; reduceMotion: boolean };
  systemFrame: boolean;
  splash: boolean;
  updateChannel: "release" | "beta";
  widgets: { id: WidgetId; visible: boolean }[];
  language: "de" | "en";
  jvm: JvmSettings;
  /** Fenster nach dem Spielstart. */
  launchBehavior: "keep" | "minimize" | "hide" | "close";
  /** Nach Spielende wieder anzeigen (bei „minimieren“). */
  showAfterExit: boolean;
  librarySort: "recent" | "name" | "playtime";
  /** Eingeklappte Gruppen der Bibliothek. */
  collapsedGroups: string[];
}

/** JVM-Presets in den Einstellungen: eigene und geänderte eingebaute (gleiche ID). */
export interface StoredPreset {
  id: string;
  name: string;
  args: string[];
  minJava: number;
}

export interface JvmSettings {
  /** `none` oder die ID eines Presets. */
  defaultPreset: string;
  presets: StoredPreset[];
}

export interface JvmPreset {
  id: string;
  name: string;
  description: string;
  args: string[];
  minJava: number;
  builtin: boolean;
  modified: boolean;
}

export type ContentKind = "mod" | "resourcepack" | "shader";
export type ProjectType = ContentKind | "modpack" | "datapack";
export type SortIndex = "relevance" | "downloads" | "follows" | "newest" | "updated";

export type ContentSource =
  | { type: "modrinth"; project_id: string; version_id: string }
  | { type: "curseforge"; project_id: number; file_id: number }
  | { type: "local" };

export interface InstalledContent {
  key: string;
  fileName: string;
  kind: ContentKind;
  enabled: boolean;
  name: string;
  version: string | null;
  source: ContentSource;
  iconUrl: string | null;
  fromPack: boolean;
  /** ID des Sync-Packs, aus dem der Inhalt stammt. */
  sync: string | null;
  size: number;
  unidentified: boolean;
}

export interface InstallReport {
  installed: [string, string][];
  alreadyPresent: string[];
  warnings: string[];
}

export interface UpdateInfo {
  key: string;
  name: string;
  current: string | null;
  latest: string;
  projectId: string;
  versionId: string;
  kind: ContentKind;
}

export interface SearchHit {
  projectId: string;
  slug: string;
  projectType: string;
  title: string;
  description: string;
  author: string;
  iconUrl: string | null;
  downloads: number;
  follows: number;
  displayCategories: string[];
  versions: string[];
  dateModified: string;
}

export interface SearchResult {
  hits: SearchHit[];
  offset: number;
  limit: number;
  totalHits: number;
}

export interface ModrinthVersion {
  id: string;
  projectId: string;
  name: string;
  versionNumber: string;
  versionType: "release" | "beta" | "alpha";
  loaders: string[];
  gameVersions: string[];
  datePublished: string;
}

export interface Category {
  name: string;
  projectType: string;
  header: string;
}

export interface SearchArgs {
  query: string;
  projectType: ProjectType;
  profileId: string | null;
  loaders: string[];
  gameVersion: string | null;
  categories: string[];
  sort: SortIndex;
  offset: number;
  limit: number;
}

export interface PackInfo {
  name: string;
  version: string;
  source: "modrinth" | "curseforge" | "twogg" | "file";
}

export interface PackUpdate {
  current: string;
  latest: string;
  versionId: string;
}

export type ChangeAction = "add" | "replace" | "remove";
export type ConflictReason = "bothChanged" | "userFileExists" | "changedButRemoved" | "userRemoved";
export type Resolution = "keepMine" | "takePack";

export interface UpdatePlan {
  packName: string;
  fromVersion: string;
  toVersion: string;
  minecraft: string;
  loader: LoaderSpec | null;
  changes: { path: string; action: ChangeAction }[];
  conflicts: { path: string; reason: ConflictReason; default: Resolution }[];
  kept: string[];
}

export interface SnapshotInfo {
  id: string;
  created: number;
  packVersion: string;
  files: number;
}

export interface ExportReport {
  path: string;
  linked: number;
  included: number;
}

export interface ManualDownload {
  name: string;
  path: string;
  url: string;
}

export interface ImportResult {
  profile: Profile;
  missing: ManualDownload[];
}

export type Platform = "modrinth" | "curseforge" | "twogg";

/** Was beim Export als Datei mitkommt (Mods immer; Welten, Serverliste und Accounts nie). */
export interface ExportOptions {
  version: string;
  summary: string | null;
  resourcepacks: boolean;
  shaders: boolean;
  config: boolean;
  /** options.txt und Shader-/OptiFine-Einstellungen. */
  options: boolean;
  /** Inhalte aus eingeschalteten Sync-Packs. */
  syncContent: boolean;
}

export interface PackPreview {
  name: string;
  version: string;
  summary: string | null;
  minecraft: string;
  loader: LoaderSpec | null;
  mods: number;
  resourcepacks: number;
  shaders: number;
  otherFiles: number;
  downloadSize: number;
  archiveSize: number;
  missing: ManualDownload[];
  /** Nicht erlaubte Download-Quellen: Installation würde scheitern. */
  blockedHosts: string[];
}

export interface PreviewResult {
  token: string;
  preview: PackPreview;
  /** `instance` = vollständiger Instanz-Export (`.pinecone.zip`). */
  source: "modrinth" | "curseforge" | "twogg" | "file" | "instance";
}

/** Vollständiger Export (F-54): was zusätzlich zu Mods, Configs und Einstellungen mitkommt. */
export interface InstanceExportOptions {
  worlds: boolean;
  screenshots: boolean;
  resourcepacks: boolean;
  shaderpacks: boolean;
}

/** Von außen geöffnet (Link, Datei) oder in der Bibliothek gewählt; leerer Link = Eingabefeld. */
export type OpenRequest = { kind: "file"; path: string } | { kind: "link"; link: string };

export interface MigrationTarget {
  minecraft: string;
  /** null = Vanilla. */
  loader: LoaderKind | null;
}

export type ItemStatus = "compatible" | "update" | "substitute" | "kept" | "missing";
export type MissingChoice = "disable" | "drop";

export interface MigrationItem {
  key: string;
  name: string;
  kind: ContentKind;
  status: ItemStatus;
  fromVersion: string | null;
  toVersion: string | null;
  substitute: string | null;
}

export interface MigrationPlan {
  fromMinecraft: string;
  fromLoader: LoaderSpec | null;
  minecraft: string;
  loader: LoaderSpec | null;
  items: MigrationItem[];
  packOffer: { name: string; platform: Platform; projectId: string; versionId: string; version: string } | null;
  worlds: number;
  downgrade: boolean;
}

export interface MigrateOptions {
  target: MigrationTarget;
  name: string | null;
  includeWorlds: boolean;
  missing: Record<string, MissingChoice>;
}

export type SyncMode = "sync" | "template";

export type SyncEntry =
  | { type: "modrinth"; projectId: string; kind: ContentKind; name: string }
  | { type: "curseforge"; projectId: number; kind: ContentKind; name: string }
  | { type: "file"; path: string; sha1: string };

export interface SyncPack {
  id: string;
  name: string;
  mode: SyncMode;
  entries: SyncEntry[];
  /** Standardwerte für options.txt. */
  options: Record<string, string>;
  /** loaders: null = Vanilla; leer = alle. */
  auto: { enabled: boolean; loaders: (LoaderKind | null)[] };
}

export interface ProfileSyncPack {
  pack: SyncPack;
  enabled: boolean;
}

export interface SyncReport {
  installed: string[];
  removed: string[];
  present: string[];
  unavailable: string[];
  warnings: string[];
}

export type SkinModel = "classic" | "slim";

export interface SavedSkin {
  id: string;
  name: string;
  model: SkinModel;
  created: number;
  updated: number;
}

export interface AccountSkin {
  name: string;
  skinUrl: string | null;
  skinDataUrl: string | null;
  model: SkinModel;
  capes: { id: string; alias: string | null; active: boolean; dataUrl: string | null }[];
}

export interface ScreenshotItem {
  profileId: string;
  profileName: string;
  path: string;
  modified: number;
  size: number;
}

export interface AppUpdate {
  version: string;
  currentVersion: string;
  date: number | null;
  notes: string | null;
}

export type UpdateEvent = { type: "progress"; done: number; total: number | null } | { type: "installing" };

export interface TwoggVersion {
  id: string;
  version: string;
  minecraft: string;
  loader: string | null;
  loaderVersion: string | null;
  channel: string;
  changelogHtml: string;
  size: number;
  sha1: string;
  publishedAt: number;
}

export interface TwoggPack {
  id: string;
  title: string;
  summary: string;
  iconUrl: string | null;
  featured: boolean;
  updatedAt: number;
  downloads: number;
  latest: TwoggVersion | null;
  descriptionHtml?: string | null;
  versions?: TwoggVersion[] | null;
  /** Event auf twogg.app, zu dem das Pack gehört. */
  eventId?: string | null;
  /** Öffentliche Seite des Packs auf dem Pinecone-Server (fehlt bei älteren Servern). */
  pageUrl?: string | null;
}

/** Eintrag der TwoGG-Auswahl in „Entdecken“. */
export interface Recommendation {
  id: string;
  projectType: ProjectType;
  source: "twogg" | "modrinth" | "curseforge";
  projectId: string;
  title: string;
  subtitle: string;
  iconUrl: string | null;
  motif: string | null;
}

export interface NewsItem {
  id: string;
  title: string;
  /** Vom Server aus Markdown erzeugt (ohne rohes HTML). */
  bodyHtml: string;
  imageUrl: string | null;
  link: string | null;
  pinned: boolean;
  publishedAt: number;
}

/** Event von twogg.app (Titel ohne Emojis, Zusammenfassung als Klartext). */
export interface AppEvent {
  id: string;
  title: string;
  summary: string;
  eventType: "STANDARD" | "TEAM" | "ROLEPLAY" | string;
  eventDate: string | null;
  registrationDeadline: string | null;
  registrationClosed: boolean;
  requiresApplication: boolean;
  minecraftVersion: string | null;
  maxPlayers: number | null;
  teamSize: number | null;
  logoUrl: string | null;
  bannerUrl: string | null;
  ended: boolean;
  url: string;
}

export interface AppUser {
  id: string;
  username: string;
  avatarUrl: string | null;
  mcUsername: string | null;
  mcUuid: string | null;
}

export interface Participation {
  eventId: string;
  kind: "registered" | "team" | "pre" | "invite";
  status: string;
  teamName: string | null;
}

export interface JoinInfo {
  approved: boolean;
  serverAddress: string | null;
}

export interface AppFriend {
  id: string;
  username: string;
  mcUsername: string | null;
  avatarUrl: string | null;
}

/** Pinecone-Freundesliste (eigener Server, Minecraft-Account als Identität). */
export interface FriendActivity {
  instance: string | null;
  version: string | null;
  server: string | null;
  playing: boolean;
}

export interface Friend {
  uuid: string;
  name: string;
  online: boolean;
  lastSeen: number;
  activity: FriendActivity | null;
}

export interface FriendsState {
  /** false bei Offline-Accounts. */
  available: boolean;
  friends?: Friend[];
  incoming?: { uuid: string; name: string }[];
  outgoing?: { uuid: string; name: string }[];
  /** Offene Einladungen an mich (F-56). */
  invites?: Invite[];
  /** Eigene Einladungen der letzten 15 Minuten. */
  sentInvites?: Invite[];
}

/** Einladung auf einen Server; `uuid`/`name` = Absender bzw. Empfänger. */
export interface Invite {
  id: string;
  uuid: string;
  name: string;
  server: string;
  /** Teilen-Link des Packs (`pinecone://…`). */
  pack: string | null;
  instance: string | null;
  status: "open" | "accepted" | "declined";
  expiresAt: number;
}

export interface ServerInfo {
  gameServer: { name: string; address: string } | null;
  /** CurseForge über den Proxy des Pinecone-Servers verfügbar. */
  curseforge: boolean;
  legal: { imprintUrl: string | null; privacyUrl: string | null; contactEmail: string | null };
}

export interface World {
  folder: string;
  name: string;
  gameMode: "survival" | "creative" | "adventure" | "spectator" | null;
  hardcore: boolean;
  /** Unix-Zeit in Millisekunden. */
  lastPlayed: number | null;
  version: string | null;
  size: number;
  icon: string | null;
}

export interface WorldBackup {
  /** Dateiname im Backup-Ordner (ID für Aktionen). */
  file: string;
  /** Ordner der gesicherten Welt. */
  folder: string;
  /** Unix-Zeit in Millisekunden. */
  created: number;
  size: number;
  auto: boolean;
}

export interface SavedServer {
  name: string;
  address: string;
  /** PNG als Base64. */
  icon: string | null;
}
