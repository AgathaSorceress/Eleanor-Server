use std::process;

use clap::{arg, command, Command};
use config::Config;
use miette::{ensure, miette, IntoDiagnostic, Result};
use paris::info;
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::SchemaManager;
use server::{add_user, remove_user, routes};
use {
    fetching::{index_initial, index_new},
    utils::{create_app_data, is_first_run, prepare_db},
};

mod config;
mod fetching;
mod migrator;
mod model;
mod server;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = command!()
        .version("0.1")
        .author("Agatha V. Lovelace")
        .subcommand(
            Command::new("user")
                .about("Manage users")
                .subcommand_required(true)
                .subcommand(
                    Command::new("add")
                        .about("Add a user")
                        .arg(arg!(<USERNAME>))
                        .arg(arg!(<PASSWORD>)),
                )
                .subcommand(
                    Command::new("remove")
                        .about("Remove a user")
                        .arg(arg!(<USERNAME>)),
                ),
        )
        .get_matches();

    // First, make sure that the app's files exist
    let first_run = is_first_run()?;
    if first_run {
        info!("No previous configuration found; Starting first run process");
        create_app_data()?;
    }

    // Create a database connection
    let db: DatabaseConnection = Database::connect(&format!(
        "sqlite://{}/eleanor-server.db?mode=rwc",
        std::env::current_dir().into_diagnostic()?.display()
    ))
    .await
    .into_diagnostic()?;

    // Run migrations
    prepare_db(&db).await?;

    let schema_manager = SchemaManager::new(&db);

    ensure!(
        schema_manager
            .has_table("library")
            .await
            .into_diagnostic()?,
        miette!("Running migrations failed")
    );

    // Handle user management
    match matches.subcommand() {
        Some(("user", args)) => {
            match args.subcommand() {
                Some(("add", args)) => {
                    add_user(
                        &db,
                        args.get_one::<String>("USERNAME")
                            .ok_or(miette!("No username provided"))?
                            .to_string(),
                        args.get_one::<String>("PASSWORD")
                            .ok_or(miette!("No password provided"))?
                            .to_string(),
                    )
                    .await?;
                }
                Some(("remove", args)) => {
                    remove_user(
                        &db,
                        args.get_one::<String>("USERNAME")
                            .ok_or(miette!("No username provided"))?
                            .to_string(),
                    )
                    .await?;
                }
                _ => (),
            }

            process::exit(0);
        }
        _ => (),
    }

    if first_run {
        index_initial(&db).await?;
    } else {
        // Index only new songs
        index_new(&db).await?;
    }

    let config = Config::read_config()?;

    // Start the server
    info!("Serving on port {}", config.port);

    axum::Server::bind(
        &format!("0.0.0.0:{}", config.port)
            .parse()
            .into_diagnostic()?,
    )
    .serve(routes(db).into_make_service())
    .await
    .into_diagnostic()?;

    Ok(())
}
