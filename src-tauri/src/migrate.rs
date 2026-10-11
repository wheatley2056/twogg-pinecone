//! Befehle für den Versionswechsel: planen und als neues Profil umziehen.

use pinecone_core::migrate::{MigrateOptions, MigrationPlan, MigrationTarget};
use serde::Serialize;
use tauri::State;

use crate::AppState;
use crate::commands::{CmdResult, err};
use crate::dto::ProfileDto;

#[tauri::command]
pub async fn migrate_plan(
    state: State<'_, AppState>,
    id: String,
    target: MigrationTarget,
) -> CmdResult<MigrationPlan> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    state
        .launcher
        .plan_migration(&profile, &target)
        .await
        .map_err(err)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateDto {
    profile: ProfileDto,
    warnings: Vec<String>,
}

#[tauri::command]
pub async fn migrate_apply(
    state: State<'_, AppState>,
    id: String,
    options: MigrateOptions,
) -> CmdResult<MigrateDto> {
    let profile = state.launcher.profiles().get(&id).map_err(err)?;
    let result = state
        .launcher
        .migrate(&profile, &options)
        .await
        .map_err(err)?;
    Ok(MigrateDto {
        profile: ProfileDto::new(result.profile, false),
        warnings: result.report.warnings,
    })
}
