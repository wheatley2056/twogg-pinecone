import { Channel, convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import type {
  Account,
  AppEvent,
  AppFriend,
  AppUser,
  FriendActivity,
  FriendsState,
  JoinInfo,
  Recommendation,
  SavedServer,
  World,
  WorldBackup,
  ServerInfo,
  Participation,
  AppInfo,
  JavaInstallation,
  LaunchEvent,
  LoaderKind,
  LoaderVersion,
  LoginStart,
  Profile,
  ProfileInput,
  ServerStatus,
  JvmPreset,
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
  InstanceExportOptions,
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
  /** Art wird aus Endung und Inhalt erkannt (Drag & Drop). */
  contentAddDropped: (id: string, path: string) => invoke<string>("content_add_dropped", { id, path }),
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
  packFillIcons: () => invoke<number>("pack_fill_icons"),
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
  syncpackProfiles: (id: string) => invoke<string[]>("syncpack_profiles", { id }),
  syncpackProfileOptions: (profileId: string) => invoke<Record<string, string>>("syncpack_profile_options", { profileId }),
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
  jvmBuiltinPresets: () => invoke<JvmPreset[]>("jvm_builtin_presets"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  exportTheme: (path: string) => invoke<void>("export_theme", { path }),
  importTheme: (path: string) => invoke<Settings>("import_theme", { path }),
  stopGame: (id: string) => invoke<void>("stop_game", { id }),

  datapackInstall: (id: string, world: string, project: string, version: string | null = null) =>
    invoke<string>("datapack_install", { id, world, project, version }),
  worlds: (id: string) => invoke<World[]>("worlds_list", { id }),
  worldBackup: (id: string, folder: string) => invoke<string>("world_backup", { id, folder }),
  worldDelete: (id: string, folder: string) => invoke<void>("world_delete", { id, folder }),
  instanceExport: (id: string, path: string, options: InstanceExportOptions) =>
    invoke<void>("instance_export", { id, path, options }),
  instanceExportSize: (id: string, options: InstanceExportOptions) => invoke<number>("instance_export_size", { id, options }),
  logSharePreview: (id: string) => invoke<string>("log_share_preview", { id }),
  logShareUpload: (text: string) => invoke<string>("log_share_upload", { text }),
  setProfileGroup: (id: string, group: string | null) => invoke<void>("set_profile_group", { id, group }),
  setProfileHooks: (id: string, preLaunch: string, wrapper: string, postExit: string) =>
    invoke<void>("set_profile_hooks", { id, preLaunch, wrapper, postExit }),
  setProfileAccount: (id: string, account: string | null) => invoke<void>("set_profile_account", { id, account }),
  duplicateProfile: (id: string, name: string, options: { worlds: boolean; screenshots: boolean }) =>
    invoke<Profile>("duplicate_profile", { id, name, options }),
  worldBackups: (id: string) => invoke<WorldBackup[]>("world_backups", { id }),
  /** Gibt den Pfad der Sicherung des vorherigen Stands zurück, falls die Welt noch existierte. */
  worldRestore: (id: string, file: string) => invoke<string | null>("world_restore", { id, file }),
  worldBackupDelete: (id: string, file: string) => invoke<void>("world_backup_delete", { id, file }),
  setBackupOnLaunch: (id: string, enabled: boolean, keep: number) =>
    invoke<void>("set_backup_on_launch", { id, enabled, keep }),
  worldOpenFolder: (id: string, folder: string) => invoke<void>("world_open_folder", { id, folder }),
  savedServers: (id: string) => invoke<SavedServer[]>("servers_list", { id }),
  recommendations: (projectType: ProjectType) => invoke<Recommendation[]>("twogg_recommendations", { projectType }),
  serverInfo: () => invoke<ServerInfo>("twogg_server_info"),
  friendsDeleteData: () => invoke<void>("friends_delete_data"),
  friends: () => invoke<FriendsState>("friends_list"),
  friendsPresence: (activity: FriendActivity) => invoke<void>("friends_presence", { activity }),
  friendRequest: (name: string) => invoke<"requested" | "friends">("friends_request", { name }),
  friendCode: (fresh = false) => invoke<string>("friends_code", { fresh }),
  friendRequestCode: (code: string) => invoke<{ status: "requested" | "friends"; name: string }>("friends_request_code", { code }),
  friendInvite: (to: string, server: string, pack: string | null, instance: string | null) =>
    invoke<void>("friends_invite", { to, server, pack, instance }),
  friendInviteAnswer: (id: string, accept: boolean) => invoke<void>("friends_invite_answer", { id, accept }),
  friendAccept: (uuid: string) => invoke<void>("friends_accept", { uuid }),
  friendDecline: (uuid: string) => invoke<void>("friends_decline", { uuid }),
  friendRemove: (uuid: string) => invoke<void>("friends_remove", { uuid }),
  appEvents: () => invoke<AppEvent[]>("twogg_app_events"),
  appMe: () => invoke<AppUser | null>("twogg_app_me"),
  appLoginDiscord: () => invoke<void>("twogg_app_login_discord"),
  appLoginPassword: (email: string, password: string) => invoke<AppUser>("twogg_app_login_password", { email, password }),
  appLogout: () => invoke<void>("twogg_app_logout"),
  appParticipations: () => invoke<Participation[]>("twogg_app_participations"),
  appJoin: (eventId: string) => invoke<JoinInfo>("twogg_app_join", { eventId }),
  appFriends: () => invoke<AppFriend[]>("twogg_app_friends"),
  /** Dateien, die ins Fenster gezogen werden: `over` beim Hineinziehen, `drop` beim Loslassen. */
  onFileDrop: (handler: (e: { type: "over" | "drop" | "leave"; paths: string[] }) => void) =>
    getCurrentWebview().onDragDropEvent((ev) => {
      const p = ev.payload;
      if (p.type === "enter") handler({ type: "over", paths: p.paths });
      else if (p.type === "over") handler({ type: "over", paths: [] });
      else if (p.type === "drop") handler({ type: "drop", paths: p.paths });
      else handler({ type: "leave", paths: [] });
    }),
  onAppLogin: (handler: (result: { user?: AppUser; error?: string }) => void) =>
    listen<{ user?: AppUser; error?: string }>("twogg-app-login", (e) => handler(e.payload)),
};

/** Lokale Datei für `<img src>`; `data:`-URLs (Mock-Modus) bleiben unverändert. */
export function fileSrc(path: string): string {
  return path.startsWith("data:") ? path : convertFileSrc(path);
}

/** Fehler aus Tauri-Befehlen kommen als Text; alles andere lesbar machen. */
export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : JSON.stringify(e);
}
