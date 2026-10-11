// Spiegel der DTOs aus src-tauri/src/dto.rs.

export type LoaderKind = "fabric" | "quilt";
export type AccountKind = "offline" | "microsoft";

export interface AppInfo {
  version: string;
  dataDir: string;
  msaConfigured: boolean;
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
  java: string | null;
  color: string | null;
  icon: string | null;
  running: boolean;
}

export interface ProfileInput {
  name: string;
  minecraft: string;
  loader: LoaderSpec | null;
  memoryMb: number | null;
  jvmArgs: string[];
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

export type Stage = "sync" | "loader" | "version" | "java" | "assets";

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

export type ThemeMode = "system" | "light" | "dark";
export type WidgetId = "recent" | "packs" | "servers" | "screenshots" | "playtime";

export interface Settings {
  theme: { mode: ThemeMode; accent: string; backgroundDim: number };
  background: string | null;
  backgroundPath: string | null;
  widgets: { id: WidgetId; visible: boolean }[];
  language: "de" | "en";
}

export type ContentKind = "mod" | "resourcepack" | "shader";
export type ProjectType = ContentKind | "modpack";
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
  source: "modrinth" | "curseforge" | "file";
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

export type Platform = "modrinth" | "curseforge";

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
  source: "modrinth" | "curseforge" | "file";
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
