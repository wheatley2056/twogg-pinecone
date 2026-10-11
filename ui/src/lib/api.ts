import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Account,
  AppInfo,
  JavaInstallation,
  LaunchEvent,
  LoaderKind,
  LoaderVersion,
  LoginStart,
  Profile,
  ProfileInput,
  ServerStatus,
  Settings,
  Dashboard,
  VersionInfo,
  Category,
  ContentKind,
  InstallReport,
  InstalledContent,
  ModrinthVersion,
  ProjectType,
  SearchArgs,
  SearchResult,
  UpdateInfo,
  ExportOptions,
  ExportReport,
  ImportResult,
  PackInfo,
  PackUpdate,
  Resolution,
  SnapshotInfo,
  UpdatePlan,
  PreviewResult,
  MigrateOptions,
  MigrationPlan,
  MigrationTarget,
  ProfileSyncPack,
  SyncMode,
  SyncPack,
  SyncReport,
  AccountSkin,
  SavedSkin,
  SkinModel,
  ScreenshotItem,
  UpdateEvent,
  AppUpdate,
  NewsItem,
  TwoggPack,
} from "./types";

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  versions: (snapshots: boolean) => invoke<VersionInfo[]>("list_versions", { snapshots }),
  loaderVersions: (kind: LoaderKind, minecraft: string) =>
    invoke<LoaderVersion[]>("loader_versions", { kind, minecraft }),

  profiles: () => invoke<Profile[]>("list_profiles"),
  createProfile: (input: ProfileInput) => invoke<Profile>("create_profile", { input }),
  updateProfile: (id: string, input: ProfileInput) => invoke<Profile>("update_profile", { id, input }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
  openProfileFolder: (id: string) => invoke<void>("open_profile_folder", { id }),
  lastLog: (id: string, maxLines: number) => invoke<string[]>("last_log", { id, maxLines }),

  accounts: () => invoke<Account[]>("list_accounts"),
  addOfflineAccount: (name: string) => invoke<Account>("add_offline_account", { name }),
  setActiveAccount: (uuid: string) => invoke<void>("set_active_account", { uuid }),
  removeAccount: (uuid: string) => invoke<void>("remove_account", { uuid }),
  loginStart: () => invoke<LoginStart>("login_start"),
  loginFinish: () => invoke<Account>("login_finish"),
  loginCancel: () => invoke<void>("login_cancel"),

  javaList: () => invoke<JavaInstallation[]>("java_list"),

  launch: (id: string, server: string | null, onEvent: (e: LaunchEvent) => void) => {
    const channel = new Channel<LaunchEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("launch", { id, server, onEvent: channel });
  },

  dashboard: () => invoke<Dashboard>("dashboard"),
  pingServer: (address: string) => invoke<ServerStatus>("ping_server", { address }),
  openScreenshot: (path: string) => invoke<void>("open_screenshot", { path }),

  contentList: (id: string) => invoke<InstalledContent[]>("content_list", { id }),
  contentIdentify: (id: string) => invoke<InstalledContent[]>("content_identify", { id }),
  contentSetEnabled: (id: string, key: string, enabled: boolean) => invoke<void>("content_set_enabled", { id, key, enabled }),
  contentRemove: (id: string, key: string) => invoke<void>("content_remove", { id, key }),
  contentAddLocal: (id: string, kind: ContentKind, path: string) => invoke<string>("content_add_local", { id, kind, path }),
  contentInstall: (id: string, kind: ContentKind, project: string, version: string | null = null) =>
    invoke<InstallReport>("content_install", { id, kind, project, version }),
  contentUpdates: (id: string) => invoke<UpdateInfo[]>("content_updates", { id }),
  contentUpdate: (id: string, update: UpdateInfo) => invoke<InstallReport>("content_update", { id, update }),
  modrinthSearch: (args: SearchArgs) => invoke<SearchResult>("modrinth_search", { args }),
  modrinthVersions: (project: string, profileId: string | null, projectType: ProjectType) =>
    invoke<ModrinthVersion[]>("modrinth_versions", { project, profileId, projectType }),
  modrinthCategories: () => invoke<Category[]>("modrinth_categories"),

  packInstallModrinth: (project: string, version: string | null = null) => invoke<ImportResult>("pack_install_modrinth", { project, version }),
  packInstallTwogg: (pack: string, version: string | null = null) => invoke<ImportResult>("pack_install_twogg", { pack, version }),
  twoggPacks: () => invoke<TwoggPack[]>("twogg_packs"),
  twoggPack: (id: string) => invoke<TwoggPack>("twogg_pack", { id }),
  twoggNews: (limit = 5) => invoke<NewsItem[]>("twogg_news", { limit }),
  packInstallCurseforge: (project: number, file: number | null = null) => invoke<ImportResult>("pack_install_curseforge", { project, file }),
  packPreviewFile: (path: string) => invoke<PreviewResult>("pack_preview_file", { path }),
  packPreviewLink: (link: string) => invoke<PreviewResult>("pack_preview_link", { link }),
  packPreviewCancel: (token: string) => invoke<void>("pack_preview_cancel", { token }),
  packImportPending: (token: string, name: string | null) => invoke<ImportResult>("pack_import_pending", { token, name }),
  packShareLink: (id: string) => invoke<string | null>("pack_share_link", { id }),
  curseforgeSearch: (args: SearchArgs) => invoke<SearchResult>("curseforge_search", { args }),
  curseforgeFiles: (project: number, profileId: string | null, projectType: ProjectType) =>
    invoke<ModrinthVersion[]>("curseforge_files", { project, profileId, projectType }),
  contentInstallCurseforge: (id: string, kind: ContentKind, project: number, file: number | null = null) =>
    invoke<InstallReport>("content_install_curseforge", { id, kind, project, file }),
  packInfo: (id: string) => invoke<PackInfo | null>("pack_info", { id }),
  packCheckUpdate: (id: string) => invoke<PackUpdate | null>("pack_check_update", { id }),
  packUpdatesAll: () => invoke<{ profile: Profile; update: PackUpdate }[]>("pack_updates_all"),
  packPlanUpdate: (id: string, version: string | null = null, file: string | null = null) =>
    invoke<UpdatePlan>("pack_plan_update", { id, version, file }),
  packApplyUpdate: (id: string, resolutions: Record<string, Resolution>) => invoke<SnapshotInfo>("pack_apply_update", { id, resolutions }),
  packSnapshots: (id: string) => invoke<SnapshotInfo[]>("pack_snapshots", { id }),
  packRollback: (id: string, snapshot: string) => invoke<void>("pack_rollback", { id, snapshot }),
  packExport: (id: string, path: string, options: ExportOptions) => invoke<ExportReport>("pack_export", { id, path, options }),
  migratePlan: (id: string, target: MigrationTarget) => invoke<MigrationPlan>("migrate_plan", { id, target }),
  migrateApply: (id: string, options: MigrateOptions) =>
    invoke<{ profile: Profile; warnings: string[] }>("migrate_apply", { id, options }),
  syncpackList: () => invoke<SyncPack[]>("syncpack_list"),
  syncpackCreate: (name: string, mode: SyncMode) => invoke<SyncPack>("syncpack_create", { name, mode }),
  syncpackSave: (pack: SyncPack) => invoke<SyncPack>("syncpack_save", { id: pack.id, pack }),
  syncpackDelete: (id: string) => invoke<void>("syncpack_delete", { id }),
  syncpackAddFile: (id: string, path: string, target: string) => invoke<SyncPack>("syncpack_add_file", { id, path, target }),
  syncpackAddFromProfile: (id: string, profileId: string, keys: string[]) =>
    invoke<SyncPack>("syncpack_add_from_profile", { id, profileId, keys }),
  syncpackForProfile: (profileId: string) => invoke<ProfileSyncPack[]>("syncpack_for_profile", { profileId }),
  syncpackSetEnabled: (profileId: string, id: string, enabled: boolean) =>
    invoke<SyncReport>("syncpack_set_enabled", { profileId, id, enabled }),
  syncpackSync: (profileId: string) => invoke<SyncReport>("syncpack_sync", { profileId }),
  updateCheck: () => invoke<AppUpdate | null>("update_check"),
  updateInstall: (onEvent: (e: UpdateEvent) => void) => {
    const channel = new Channel<UpdateEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("update_install", { onEvent: channel });
  },
  diagnosticsExport: (path: string, profileId: string | null = null) =>
    invoke<{ path: string; files: number; bytes: number }>("diagnostics_export", { path, profileId }),
  openLogsFolder: () => invoke<void>("open_logs_folder"),
  screenshotsList: () => invoke<ScreenshotItem[]>("screenshots_list"),
  screenshotThumb: (path: string) => invoke<string>("screenshot_thumb", { path }),
  screenshotDelete: (path: string) => invoke<void>("screenshot_delete", { path }),
  screenshotReveal: (path: string) => invoke<void>("screenshot_reveal", { path }),
  screenshotSaveAs: (path: string, target: string) => invoke<void>("screenshot_save_as", { path, target }),
  onScreenshotsChanged: (handler: () => void) => listen("screenshots-changed", () => handler()),
  skinsList: () => invoke<SavedSkin[]>("skins_list"),
  skinData: (id: string) => invoke<string>("skin_data", { id }),
  skinImportFile: (path: string, model: SkinModel) => invoke<SavedSkin>("skin_import_file", { path, model }),
  skinImportPlayer: (player: string) => invoke<SavedSkin>("skin_import_player", { player }),
  skinSave: (id: string | null, name: string, model: SkinModel, dataUrl: string) =>
    invoke<SavedSkin>("skin_save", { id, name, model, dataUrl }),
  skinRename: (id: string, name: string) => invoke<SavedSkin>("skin_rename", { id, name }),
  skinSetModel: (id: string, model: SkinModel) => invoke<SavedSkin>("skin_set_model", { id, model }),
  skinDelete: (id: string) => invoke<void>("skin_delete", { id }),
  skinExport: (id: string, path: string) => invoke<void>("skin_export", { id, path }),
  skinAccount: () => invoke<AccountSkin>("skin_account"),
  skinApply: (id: string) => invoke<AccountSkin>("skin_apply", { id }),
  skinSetCape: (capeId: string | null) => invoke<AccountSkin>("skin_set_cape", { capeId }),
  skinImportAccount: () => invoke<SavedSkin>("skin_import_account"),
  /** Beim Start übergebene Links/Dateien (einmalig) und später geöffnete. */
  takeOpenRequests: () => invoke<string[]>("take_open_requests"),
  onOpenPack: (handler: (request: string) => void) => listen<string>("open-pack", (e) => handler(e.payload)),

  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  exportTheme: (path: string) => invoke<void>("export_theme", { path }),
  importTheme: (path: string) => invoke<Settings>("import_theme", { path }),
  stopGame: (id: string) => invoke<void>("stop_game", { id }),
};

/** Lokale Datei für `<img src>`; `data:`-URLs (Mock-Modus) bleiben unverändert. */
export function fileSrc(path: string): string {
  return path.startsWith("data:") ? path : convertFileSrc(path);
}

/** Fehler aus Tauri-Befehlen kommen als Text; alles andere lesbar machen. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
