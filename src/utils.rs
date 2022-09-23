use miette::{miette, IntoDiagnostic, Result};
use std::fs::File;

use crate::{config::Config, migrator::Migrator, model::library};
use paris::success;
use sea_orm_migration::prelude::*;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

/// If no files have been created in the current directory, the app is running for the first time
pub fn is_first_run() -> Result<bool> {
    let path = std::env::current_dir()
        .map(|v| v.join("settings.toml"))
        .into_diagnostic()?;

    Ok(!path.exists())
}

/// Create the necessary files on first run
pub fn create_app_data() -> Result<()> {
    let path = std::env::current_dir().into_diagnostic()?;

    File::create(&path.join("eleanor-server.db")).into_diagnostic()?;
    Config::write_config(&Default::default())?;
    success!("Created configuration file");

    Ok(())
}

/// Run unapplied migrations
pub async fn prepare_db(db: &sea_orm::DatabaseConnection) -> Result<()> {
    Migrator::up(db, None).await.into_diagnostic()?;

    success!("Applied migrations");

    Ok(())
}

/// Returns a file path from the audio hash
pub async fn path_from_hash(db: &sea_orm::DatabaseConnection, hash: u32) -> Result<String> {
    let track = library::Entity::find()
        .filter(library::Column::Hash.eq(hash))
        .one(db)
        .await
        .ok()
        .flatten()
        .ok_or(miette!("Track not found"))?;

    let path = format!("{}/{}", track.path, track.filename);

    Ok(path)
}
