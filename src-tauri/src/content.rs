//! Befehle für Inhalte eines Profils (Mods, Ressourcenpakete, Shader) und den Modrinth-Browser.

use std::path::{Path, PathBuf};

use pinecone_core::content::{
    ContentKind, InstallReport, InstalledContent, UpdateInfo, loaders_for,
};
use pinecone_core::curseforge::{CfSearch, ClassKind, CurseForge};
use pinecone_core::modrinth::{
    Category, ProjectType, SearchHit, SearchQuery, SearchResult, SortIndex, Version,
};
use pinecone_core::profiles::Profile;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};

fn profile_and_dir(state: &AppState, id: &str) -> CmdResult<(Profile, PathBuf)> {
    let profile = state.launcher.profiles().get(id).map_err(err)?;
    let dir = state.launcher.game_dir(&profile);
    Ok((profile, dir))
}

/// Schnell und ohne Netz; unbekannte Dateien sind markiert.
#[tauri::command]
pub fn content_list(state: State<'_, AppState>, id: String) -> CmdResult<Vec<InstalledContent>> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    state.launcher.content().list(&id, &dir).map_err(err)
}

/// Unbekannte Dateien per Hash bei Modrinth nachschlagen.
#[tauri::command]
pub async fn content_identify(
    state: State<'_, AppState>,
    id: String,
) -> CmdResult<Vec<InstalledContent>> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .identify(&id, &dir)
        .await
        .map_err(err)
}

#[tauri::command]
pub fn content_set_enabled(
    state: State<'_, AppState>,
    id: String,
    key: String,
    enabled: bool,
) -> CmdResult<()> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .set_enabled(&dir, &key, enabled)
        .map_err(err)
}

#[tauri::command]
pub fn content_remove(state: State<'_, AppState>, id: String, key: String) -> CmdResult<()> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .remove(&id, &dir, &key)
        .map_err(err)
}

#[tauri::command]
pub async fn content_add_local(
    state: State<'_, AppState>,
    id: String,
    kind: ContentKind,
    path: String,
) -> CmdResult<String> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .add_local(&id, &dir, kind, Path::new(&path))
        .await
        .map_err(err)
}

/// Hineingezogene Datei hinzufügen; die Art wird aus Endung und Inhalt erkannt.
#[tauri::command]
pub async fn content_add_dropped(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> CmdResult<String> {
    let (_, dir) = profile_and_dir(&state, &id)?;
    let path = Path::new(&path);
    let kind = ContentKind::detect(path).map_err(err)?;
    state
        .launcher
        .content()
        .add_local(&id, &dir, kind, path)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn content_install(
    state: State<'_, AppState>,
    id: String,
    kind: ContentKind,
    project: String,
    version: Option<String>,
) -> CmdResult<InstallReport> {
    let (profile, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .install(&profile, &dir, kind, &project, version.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn content_updates(state: State<'_, AppState>, id: String) -> CmdResult<Vec<UpdateInfo>> {
    let (profile, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .identify(&id, &dir)
        .await
        .map_err(err)?;
    state
        .launcher
        .content()
        .check_updates(&profile, &dir)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn content_update(
    state: State<'_, AppState>,
    id: String,
    update: UpdateInfo,
) -> CmdResult<InstallReport> {
    let (profile, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .apply_update(&profile, &dir, &update)
        .await
        .map_err(err)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    query: String,
    project_type: ProjectType,
    /// Filter aus einem Profil übernehmen (Loader und Version).
    profile_id: Option<String>,
    #[serde(default)]
    loaders: Vec<String>,
    game_version: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    sort: SortIndex,
    #[serde(default)]
    offset: u32,
    limit: Option<u32>,
}

#[tauri::command]
pub async fn modrinth_search(
    state: State<'_, AppState>,
    args: SearchArgs,
) -> CmdResult<SearchResult> {
    let mut query = SearchQuery {
        query: args.query,
        project_type: Some(args.project_type),
        loaders: args.loaders,
        game_version: args.game_version,
        categories: args.categories,
        sort: args.sort,
        offset: args.offset,
        limit: args.limit.unwrap_or(20),
    };
    if let Some(id) = args.profile_id {
        let (profile, _) = profile_and_dir(&state, &id)?;
        if let Some(kind) = ContentKind::from_project_type(args.project_type) {
            query.loaders = loaders_for(&profile, kind).ok_or(
                "Vanilla-Profile haben keine Mods. Bitte im Profil Fabric oder Quilt wählen.",
            )?;
        }
        query.game_version = Some(profile.minecraft);
    }
    state
        .launcher
        .content()
        .modrinth()
        .search(&query)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn modrinth_versions(
    state: State<'_, AppState>,
    project: String,
    profile_id: Option<String>,
    project_type: ProjectType,
) -> CmdResult<Vec<Version>> {
    let (loaders, versions) = match profile_id {
        Some(id) => {
            let (profile, _) = profile_and_dir(&state, &id)?;
            let loaders = if project_type == ProjectType::Datapack {
                vec!["datapack".to_string()]
            } else {
                ContentKind::from_project_type(project_type)
                    .and_then(|k| loaders_for(&profile, k))
                    .unwrap_or_default()
            };
            (loaders, vec![profile.minecraft])
        }
        None => (Vec::new(), Vec::new()),
    };
    state
        .launcher
        .content()
        .modrinth()
        .versions(&project, &loaders, &versions)
        .await
        .map_err(err)
}

#[tauri::command]
pub async fn modrinth_categories(state: State<'_, AppState>) -> CmdResult<Vec<Category>> {
    state
        .launcher
        .content()
        .modrinth()
        .categories()
        .await
        .map_err(err)
}

fn cf(state: &AppState) -> CmdResult<&CurseForge> {
    state.launcher.content().curseforge().ok_or_else(|| {
        "CurseForge ist nicht eingerichtet (PINECONE_CURSEFORGE_API_KEY fehlt)".to_string()
    })
}

/// CurseForge-Klasse; Datenpakete führt Pinecone nur über Modrinth.
fn class_of(t: ProjectType) -> Option<ClassKind> {
    Some(match t {
        ProjectType::Mod => ClassKind::Mod,
        ProjectType::Resourcepack => ClassKind::Resourcepack,
        ProjectType::Shader => ClassKind::Shader,
        ProjectType::Modpack => ClassKind::Modpack,
        ProjectType::Datapack => return None,
    })
}

/// CurseForge-Suche, als Modrinth-artiges Ergebnis (gleiche Darstellung in der Oberfläche).
#[tauri::command]
pub async fn curseforge_search(
    state: State<'_, AppState>,
    args: SearchArgs,
) -> CmdResult<SearchResult> {
    let cf = cf(&state)?;
    let Some(class) = class_of(args.project_type) else {
        return Ok(SearchResult {
            offset: args.offset,
            limit: args.limit.unwrap_or(20),
            total_hits: 0,
            hits: Vec::new(),
        });
    };
    let mut q = CfSearch {
        query: args.query,
        class: Some(class),
        game_version: args.game_version,
        loader: args.loaders.first().and_then(|l| l.parse().ok()),
        sort_field: match args.sort {
            SortIndex::Relevance | SortIndex::Follows => 2,
            SortIndex::Downloads => 6,
            SortIndex::Newest | SortIndex::Updated => 3,
        },
        index: args.offset,
        page_size: args.limit.unwrap_or(20),
    };
    if let Some(id) = args.profile_id {
        let (profile, _) = profile_and_dir(&state, &id)?;
        if args.project_type == ProjectType::Mod {
            q.loader = Some(profile.loader.as_ref().map(|l| l.kind).ok_or(
                "Vanilla-Profile haben keine Mods. Bitte im Profil Fabric oder Quilt wählen.",
            )?);
        }
        q.game_version = Some(profile.minecraft);
    }
    let page = cf.search(&q).await.map_err(err)?;
    Ok(SearchResult {
        offset: page.pagination.index,
        limit: page.pagination.page_size,
        // CurseForge liefert höchstens 10 000 Treffer aus.
        total_hits: page.pagination.total_count.min(10_000),
        hits: page
            .data
            .into_iter()
            .map(|m| SearchHit {
                project_id: m.id.to_string(),
                slug: m.slug,
                project_type: args.project_type.as_str().into(),
                title: m.name,
                description: m.summary,
                author: m
                    .authors
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                icon_url: m.logo.and_then(|l| l.thumbnail_url),
                downloads: m.download_count as u64,
                follows: 0,
                display_categories: m.categories.into_iter().map(|c| c.name).collect(),
                versions: Vec::new(),
                date_modified: m.date_modified,
            })
            .collect(),
    })
}

/// Datei einer CurseForge-Projektliste im Format der Versionswahl.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CfVersionDto {
    id: String,
    project_id: String,
    name: String,
    version_number: String,
    version_type: &'static str,
    loaders: Vec<String>,
    game_versions: Vec<String>,
    date_published: String,
}

#[tauri::command]
pub async fn curseforge_files(
    state: State<'_, AppState>,
    project: u64,
    profile_id: Option<String>,
    project_type: ProjectType,
) -> CmdResult<Vec<CfVersionDto>> {
    let cf = cf(&state)?;
    let (version, loader) = match profile_id {
        Some(id) => {
            let (profile, _) = profile_and_dir(&state, &id)?;
            let loader = (project_type == ProjectType::Mod)
                .then(|| profile.loader.as_ref().map(|l| l.kind))
                .flatten();
            (Some(profile.minecraft), loader)
        }
        None => (None, None),
    };
    let mut files = cf
        .files(project, version.as_deref(), loader)
        .await
        .map_err(err)?;
    files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    Ok(files
        .into_iter()
        .map(|f| CfVersionDto {
            id: f.id.to_string(),
            project_id: f.mod_id.to_string(),
            name: f.display_name.clone(),
            version_number: f.display_name,
            version_type: match f.release_type {
                1 => "release",
                2 => "beta",
                _ => "alpha",
            },
            loaders: Vec::new(),
            game_versions: f.game_versions,
            date_published: f.file_date,
        })
        .collect())
}

#[tauri::command]
pub async fn content_install_curseforge(
    state: State<'_, AppState>,
    id: String,
    kind: ContentKind,
    project: u64,
    file: Option<u64>,
) -> CmdResult<InstallReport> {
    let (profile, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .install_curseforge(&profile, &dir, kind, project, file)
        .await
        .map_err(err)
}

/// Datenpaket in eine Welt der Instanz installieren.
#[tauri::command]
pub async fn datapack_install(
    state: State<'_, AppState>,
    id: String,
    world: String,
    project: String,
    version: Option<String>,
) -> CmdResult<String> {
    let (profile, dir) = profile_and_dir(&state, &id)?;
    state
        .launcher
        .content()
        .install_datapack(
            &dir,
            &world,
            &project,
            version.as_deref(),
            &profile.minecraft,
        )
        .await
        .map_err(err)
}
