//! Versionswechsel: ein Profil auf eine andere Minecraft-Version bzw. einen anderen Loader
//! umziehen. Für jeden Inhalt wird die passende Version gesucht (Modrinth per Hash, CurseForge
//! per Projekt), bekannte Loader-Gegenstücke werden ersetzt, der Rest wird gemeldet.
//!
//! Umgezogen wird immer in ein neues Profil: Das Original bleibt unverändert und ist damit
//! zugleich die Sicherung (Welten vertragen keinen Downgrade).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::content::{ContentKind, InstallReport, LockEntry, Source, loaders_for};
use crate::curseforge;
use crate::launcher::Launcher;
use crate::loader::{LoaderKind, LoaderSpec, compare_versions};
use crate::modpack::PackSource;
use crate::modrinth::{self, VersionType};
use crate::profiles::Profile;
use crate::{Error, Result};

/// Bekannte Gegenstücke, wenn ein Mod für den Ziel-Loader fehlt:
/// (Modrinth-Projekt, Ziel-Loader, Ersatzprojekt, Name des Ersatzes).
const SUBSTITUTES: &[(&str, LoaderKind, &str, &str)] = &[
    // Quilted Fabric API → Fabric API (umgekehrt nicht nötig: Quilt lädt Fabric API).
    ("qvIfYCYJ", LoaderKind::Fabric, "P7dR8mSH", "Fabric API"),
];

/// Ordner, die beim Kopieren nicht mitkommen.
const SKIP_DIRS: [&str; 2] = ["logs", "crash-reports"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationTarget {
    pub minecraft: String,
    /// `None`: Vanilla (Mods werden dann deaktiviert oder weggelassen).
    pub loader: Option<LoaderKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    /// Die installierte Datei passt schon.
    Compatible,
    /// Andere Version desselben Projekts.
    Update,
    /// Ersetzt durch ein Gegenstück für den Ziel-Loader.
    Substitute,
    /// Bleibt unverändert (Ressourcenpakete/Shader ohne passende Version, eigene Dateien).
    Kept,
    /// Nicht verfügbar: wird deaktiviert mitgenommen oder weggelassen.
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationItem {
    pub key: String,
    pub name: String,
    pub kind: ContentKind,
    pub status: ItemStatus,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    /// Name des Ersatzprojekts bei `Substitute`.
    pub substitute: Option<String>,
}

/// Das Modpack selbst gibt es für das Ziel: dann besser diese Version installieren.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackOffer {
    pub name: String,
    /// `modrinth` oder `curseforge`.
    pub platform: String,
    pub project_id: String,
    pub version_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationPlan {
    pub from_minecraft: String,
    pub from_loader: Option<LoaderSpec>,
    pub minecraft: String,
    /// Mit der empfohlenen Loader-Version für das Ziel.
    pub loader: Option<LoaderSpec>,
    pub items: Vec<MigrationItem>,
    pub pack_offer: Option<PackOffer>,
    /// Anzahl der Welten im Profil.
    pub worlds: usize,
    /// Ziel ist älter als die aktuelle Version: Welten können kaputtgehen.
    pub downgrade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingChoice {
    /// Datei bleibt, aber deaktiviert (lässt sich später wieder einschalten).
    #[default]
    Disable,
    /// Datei kommt nicht ins neue Profil.
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateOptions {
    pub target: MigrationTarget,
    /// Name des neuen Profils; ohne Angabe „<Name> (<Version>)“.
    #[serde(default)]
    pub name: Option<String>,
    /// Welten mitkopieren.
    #[serde(default = "yes")]
    pub include_worlds: bool,
    /// Entscheidung je fehlendem Inhalt (Schlüssel), sonst `Disable`.
    #[serde(default)]
    pub missing: HashMap<String, MissingChoice>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResult {
    pub profile: Profile,
    pub report: InstallReport,
}

/// Was beim Umzug mit einem Inhalt passiert (intern, wird beim Anwenden neu berechnet).
#[derive(Debug, Clone)]
enum Action {
    Nothing,
    Modrinth { project: String, version: String },
    Curseforge { project: u64, file: u64 },
    Substitute { project: String },
    Missing,
}

impl Launcher {
    /// Plant den Umzug, ohne etwas zu ändern.
    pub async fn plan_migration(
        &self,
        profile: &Profile,
        target: &MigrationTarget,
    ) -> Result<MigrationPlan> {
        Ok(self.compute_migration(profile, target).await?.0)
    }

    async fn compute_migration(
        &self,
        profile: &Profile,
        target: &MigrationTarget,
    ) -> Result<(MigrationPlan, Vec<Action>)> {
        let minecraft = target.minecraft.trim().to_string();
        if minecraft.is_empty() {
            return Err(Error::Invalid("Bitte eine Minecraft-Version wählen".into()));
        }
        if minecraft == profile.minecraft
            && target.loader == profile.loader.as_ref().map(|l| l.kind)
        {
            return Err(Error::Invalid(
                "Das Profil hat schon diese Version und diesen Loader".into(),
            ));
        }
        let loader = match target.loader {
            Some(kind) => Some(LoaderSpec {
                kind,
                version: self.loaders().recommended(kind, &minecraft).await?.version,
            }),
            None => None,
        };
        let mut new_profile = Profile::new(profile.name.clone(), minecraft.clone());
        new_profile.loader = loader.clone();

        let game_dir = self.game_dir(profile);
        let content = self.content();
        let installed = content.identify(&profile.id, &game_dir).await?;
        let lock = content.load_lock(&profile.id)?;
        let entry = |key: &str| lock.files.get(key);

        // Modrinth: je Inhaltsart eine Sammelanfrage per Hash.
        let mut found: HashMap<String, modrinth::Version> = HashMap::new();
        for kind in ContentKind::ALL {
            let Some(loaders) = loaders_for(&new_profile, kind) else {
                continue;
            };
            let hashes: Vec<String> = installed
                .iter()
                .filter(|c| c.kind == kind && matches!(c.source, Source::Modrinth { .. }))
                .filter_map(|c| entry(&c.key).map(|e| e.sha1.clone()))
                .collect();
            let latest = content
                .modrinth()
                .latest_by_hash(&hashes, &loaders, std::slice::from_ref(&minecraft))
                .await?;
            found.extend(latest);
        }

        let mut items = Vec::new();
        let mut actions = Vec::new();
        for c in &installed {
            let e = entry(&c.key);
            let mut item = MigrationItem {
                key: c.key.clone(),
                name: c.name.clone(),
                kind: c.kind,
                status: ItemStatus::Kept,
                from_version: c.version.clone(),
                to_version: None,
                substitute: None,
            };
            let moddable = loaders_for(&new_profile, c.kind).is_some();
            // `confirmed`: Es gibt nachweislich eine passende Version für das Ziel.
            let (action, confirmed) = match (&c.source, e) {
                _ if !moddable => (Action::Missing, false),
                (
                    Source::Modrinth {
                        project_id,
                        version_id,
                    },
                    Some(e),
                ) => {
                    match self
                        .modrinth_target(&new_profile, c.kind, project_id, e, &found)
                        .await?
                    {
                        Some(v) if v.id == *version_id => (Action::Nothing, true),
                        Some(v) => {
                            item.to_version = Some(v.version_number.clone());
                            let action = Action::Modrinth {
                                project: project_id.clone(),
                                version: v.id,
                            };
                            (action, true)
                        }
                        None => match substitute(project_id, target.loader) {
                            Some((project, name)) => {
                                item.substitute = Some(name.to_string());
                                let action = Action::Substitute {
                                    project: project.to_string(),
                                };
                                (action, true)
                            }
                            None if c.kind == ContentKind::Mod => (Action::Missing, false),
                            None => (Action::Nothing, false),
                        },
                    }
                }
                (
                    Source::Curseforge {
                        project_id,
                        file_id,
                    },
                    Some(_),
                ) => {
                    match self
                        .curseforge_target(&new_profile, c.kind, *project_id)
                        .await
                    {
                        Some(f) if f.id == *file_id => (Action::Nothing, true),
                        Some(f) => {
                            item.to_version = Some(f.display_name.clone());
                            let action = Action::Curseforge {
                                project: *project_id,
                                file: f.id,
                            };
                            (action, true)
                        }
                        None if c.kind == ContentKind::Mod => (Action::Missing, false),
                        None => (Action::Nothing, false),
                    }
                }
                // Eigene Mods ohne Quelle: Kompatibilität unbekannt.
                _ if c.kind == ContentKind::Mod => (Action::Missing, false),
                _ => (Action::Nothing, false),
            };
            item.status = match &action {
                Action::Nothing if confirmed => ItemStatus::Compatible,
                Action::Nothing => ItemStatus::Kept,
                Action::Modrinth { .. } | Action::Curseforge { .. } => ItemStatus::Update,
                Action::Substitute { .. } => ItemStatus::Substitute,
                Action::Missing => ItemStatus::Missing,
            };
            items.push(item);
            actions.push(action);
        }

        let saves = game_dir.join("saves");
        let worlds = std::fs::read_dir(&saves)
            .map(|d| d.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        let plan = MigrationPlan {
            from_minecraft: profile.minecraft.clone(),
            from_loader: profile.loader.clone(),
            downgrade: compare_versions(&minecraft, &profile.minecraft).is_lt(),
            minecraft,
            loader,
            items,
            pack_offer: self.pack_offer(profile, &new_profile).await,
            worlds,
        };
        Ok((plan, actions))
    }

    /// Passende Modrinth-Version: aus dem Sammelabgleich, außer sie ist instabiler als die
    /// installierte (dann gezielt nach einer stabileren suchen).
    async fn modrinth_target(
        &self,
        target: &Profile,
        kind: ContentKind,
        project: &str,
        entry: &LockEntry,
        found: &HashMap<String, modrinth::Version>,
    ) -> Result<Option<modrinth::Version>> {
        let channel = entry.channel.unwrap_or(VersionType::Release);
        match found.get(&entry.sha1) {
            Some(v) if v.version_type >= channel => Ok(Some(v.clone())),
            Some(v) => {
                let loaders = loaders_for(target, kind).unwrap_or_default();
                let versions = self
                    .content()
                    .modrinth()
                    .versions(project, &loaders, std::slice::from_ref(&target.minecraft))
                    .await?;
                Ok(Some(
                    versions
                        .iter()
                        .find(|x| x.version_type >= channel)
                        .unwrap_or(v)
                        .clone(),
                ))
            }
            None => Ok(None),
        }
    }

    async fn curseforge_target(
        &self,
        target: &Profile,
        kind: ContentKind,
        project: u64,
    ) -> Option<curseforge::File> {
        let cf = self.content().curseforge()?;
        let loader = (kind == ContentKind::Mod)
            .then(|| target.loader.as_ref().map(|l| l.kind))
            .flatten();
        match cf.files(project, Some(&target.minecraft), loader).await {
            Ok(files) => curseforge::best_file(&files)
                .filter(|f| f.download_url.is_some())
                .cloned(),
            Err(e) => {
                tracing::warn!("CurseForge-Projekt {project} nicht prüfbar: {e}");
                None
            }
        }
    }

    /// Das Pack des Profils in einer Version für das Ziel?
    async fn pack_offer(&self, profile: &Profile, target: &Profile) -> Option<PackOffer> {
        let info = self.packs().pack_info(&profile.id).ok()??;
        let loader = target.loader.as_ref().map(|l| l.kind);
        match &info.source {
            PackSource::Modrinth { project_id, .. } => {
                let loaders: Vec<String> = loader
                    .map(|k| vec![k.name().to_ascii_lowercase()])
                    .unwrap_or_default();
                let versions = self
                    .content()
                    .modrinth()
                    .versions(
                        project_id,
                        &loaders,
                        std::slice::from_ref(&target.minecraft),
                    )
                    .await
                    .ok()?;
                let best = modrinth::best_version(&versions)?;
                Some(PackOffer {
                    name: info.name.clone(),
                    platform: "modrinth".into(),
                    project_id: project_id.clone(),
                    version_id: best.id.clone(),
                    version: best.version_number.clone(),
                })
            }
            PackSource::Curseforge { project_id, .. } => {
                let cf = self.content().curseforge()?;
                let files = cf
                    .files(*project_id, Some(&target.minecraft), loader)
                    .await
                    .ok()?;
                let best = curseforge::best_file(&files)?;
                Some(PackOffer {
                    name: info.name.clone(),
                    platform: "curseforge".into(),
                    project_id: project_id.to_string(),
                    version_id: best.id.to_string(),
                    version: best.display_name.clone(),
                })
            }
            PackSource::Twogg { pack_id, .. } => {
                let pack = self.packs().twogg()?.pack(pack_id).await.ok()?;
                let versions = pack.versions.unwrap_or_default();
                let best = crate::twogg::best_version(&versions, Some(&target.minecraft))?;
                // Loader muss passen (Pack-Versionen haben einen festen Loader).
                if best.loader.as_deref()
                    != loader.map(|k| k.name().to_ascii_lowercase()).as_deref()
                {
                    return None;
                }
                Some(PackOffer {
                    name: info.name.clone(),
                    platform: "twogg".into(),
                    project_id: pack_id.clone(),
                    version_id: best.id.clone(),
                    version: best.version.clone(),
                })
            }
            PackSource::File => None,
        }
    }

    /// Zieht das Profil in ein neues Profil um. Einzelne fehlgeschlagene Inhalte stehen in den
    /// Warnungen; das neue Profil bleibt trotzdem bestehen.
    pub async fn migrate(
        &self,
        profile: &Profile,
        opts: &MigrateOptions,
    ) -> Result<MigrationResult> {
        let (plan, actions) = self.compute_migration(profile, &opts.target).await?;
        let base = opts
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{} ({})", profile.name, plan.minecraft));
        let mut name = base.clone();
        let mut n = 2;
        while self.profiles().find(&name)?.is_some() {
            name = format!("{base} ({n})");
            n += 1;
        }
        let mut new_profile = profile.clone();
        new_profile.id = String::new();
        new_profile.name = name;
        new_profile.minecraft = plan.minecraft.clone();
        new_profile.loader = plan.loader.clone();
        let new_profile = self.profiles().create(new_profile)?;

        let result = self
            .fill_migrated(profile, &new_profile, &plan, actions, opts)
            .await;
        match result {
            Ok(report) => Ok(MigrationResult {
                profile: new_profile,
                report,
            }),
            Err(e) => {
                let _ = self.profiles().delete(&new_profile.id);
                Err(e)
            }
        }
    }

    async fn fill_migrated(
        &self,
        old: &Profile,
        new: &Profile,
        plan: &MigrationPlan,
        actions: Vec<Action>,
        opts: &MigrateOptions,
    ) -> Result<InstallReport> {
        let from = self.game_dir(old);
        let to = self.game_dir(new);
        let skip: HashSet<&str> = SKIP_DIRS
            .iter()
            .copied()
            .chain((!opts.include_worlds).then_some("saves"))
            .collect();
        let (from_c, to_c) = (from.clone(), to.clone());
        tokio::task::spawn_blocking(move || copy_dir(&from_c, &to_c, &skip, true))
            .await
            .map_err(|e| Error::Invalid(format!("Kopieren abgebrochen: {e}")))??;

        // Eingeschaltete Sync-Packs gelten auch im neuen Profil.
        let state = self.paths().instance_dir(&old.id).join("syncpacks.json");
        if state.is_file() {
            let dest = self.paths().instance_dir(&new.id).join("syncpacks.json");
            std::fs::copy(&state, &dest).map_err(Error::io(&dest))?;
        }

        // Herkunft der Inhalte übernehmen; das neue Profil ist kein Pack-Profil mehr.
        let content = self.content();
        let mut lock = content.load_lock(&old.id)?;
        lock.pack = None;
        for e in lock.files.values_mut() {
            e.from_pack = false;
        }
        content.save_lock(&new.id, &lock)?;

        let mut report = InstallReport::default();
        for (item, action) in plan.items.iter().zip(actions) {
            let step = match action {
                Action::Nothing => Ok(None),
                Action::Modrinth { project, version } => content
                    .install(new, &to, item.kind, &project, Some(&version))
                    .await
                    .map(Some),
                Action::Curseforge { project, file } => content
                    .install_curseforge(new, &to, item.kind, project, Some(file))
                    .await
                    .map(Some),
                Action::Substitute { project } => {
                    let was_enabled = to.join(&item.key).is_file();
                    match content.install(new, &to, item.kind, &project, None).await {
                        Ok(r) => {
                            content.remove(&new.id, &to, &item.key)?;
                            if !was_enabled
                                && let Some(key) = installed_key(content, &new.id, &project)?
                            {
                                content.set_enabled(&to, &key, false)?;
                            }
                            Ok(Some(r))
                        }
                        Err(e) => Err(e),
                    }
                }
                Action::Missing => {
                    match opts.missing.get(&item.key).copied().unwrap_or_default() {
                        MissingChoice::Disable => content.set_enabled(&to, &item.key, false)?,
                        MissingChoice::Drop => content.remove(&new.id, &to, &item.key)?,
                    }
                    Ok(None)
                }
            };
            match step {
                Ok(Some(r)) => {
                    report.installed.extend(r.installed);
                    report.already_present.extend(r.already_present);
                    report.warnings.extend(r.warnings);
                }
                Ok(None) => {}
                Err(e) => report.warnings.push(format!("{}: {e}", item.name)),
            }
        }
        content.identify(&new.id, &to).await?;
        Ok(report)
    }
}

fn substitute(project: &str, loader: Option<LoaderKind>) -> Option<(&'static str, &'static str)> {
    let loader = loader?;
    SUBSTITUTES
        .iter()
        .find(|(from, to_loader, _, _)| *from == project && *to_loader == loader)
        .map(|(_, _, to, name)| (*to, *name))
}

/// Schlüssel der Datei eines Modrinth-Projekts im Profil.
fn installed_key(
    content: &crate::content::ContentManager,
    profile_id: &str,
    project: &str,
) -> Result<Option<String>> {
    Ok(content
        .load_lock(profile_id)?
        .files
        .into_iter()
        .find(|(_, e)| matches!(&e.source, Source::Modrinth { project_id, .. } if project_id == project))
        .map(|(k, _)| k))
}

/// Kopiert einen Ordner rekursiv (ohne Symlinks); `skip` gilt nur auf oberster Ebene.
pub(crate) fn copy_dir(from: &Path, to: &Path, skip: &HashSet<&str>, top: bool) -> Result<()> {
    std::fs::create_dir_all(to).map_err(Error::io(to))?;
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for e in entries.flatten() {
        let name = e.file_name();
        if top && name.to_str().is_some_and(|n| skip.contains(n)) {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        let target = to.join(&name);
        if ft.is_dir() {
            copy_dir(&e.path(), &target, skip, false)?;
        } else if ft.is_file() {
            std::fs::copy(e.path(), &target).map_err(Error::io(&target))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_only_for_matching_loader() {
        assert_eq!(
            substitute("qvIfYCYJ", Some(LoaderKind::Fabric)),
            Some(("P7dR8mSH", "Fabric API"))
        );
        assert_eq!(substitute("qvIfYCYJ", Some(LoaderKind::Quilt)), None);
        assert_eq!(substitute("qvIfYCYJ", None), None);
        assert_eq!(substitute("AANobbMI", Some(LoaderKind::Fabric)), None);
    }

    #[test]
    fn copy_skips_top_level_dirs_only() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a");
        for rel in [
            "logs/latest.log",
            "saves/w/level.dat",
            "config/logs/x.json",
            "options.txt",
        ] {
            let p = src.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        }
        let dst = dir.path().join("b");
        copy_dir(&src, &dst, &HashSet::from(["logs", "saves"]), true).unwrap();
        assert!(!dst.join("logs").exists());
        assert!(!dst.join("saves").exists());
        assert!(dst.join("config/logs/x.json").is_file());
        assert!(dst.join("options.txt").is_file());
    }
}
