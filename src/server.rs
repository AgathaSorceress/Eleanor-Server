use crate::{
    model::{library, users},
    utils::path_from_hash,
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{Path, RequestParts},
    headers::{authorization::Basic, Authorization, HeaderMapExt},
    http::{
        header::{self, HeaderName},
        Request, StatusCode,
    },
    middleware::{self, Next},
    response::Response,
    routing::get,
    Extension, Router,
};
use miette::{miette, IntoDiagnostic};
use paris::success;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter, Set};
use tower::ServiceBuilder;

pub fn routes(db: DatabaseConnection) -> Router {
    Router::new()
        .route("/", get(list_library).post(reindex))
        .route("/:hash", get(get_stream))
        .route("/:hash/cover", get(get_album_art))
        .layer(
            ServiceBuilder::new()
                .layer(Extension(db))
                .layer(middleware::from_fn(auth)),
        )
}

async fn list_library(
    Extension(ref db): Extension<DatabaseConnection>,
) -> Result<Vec<u8>, StatusCode> {
    let library = library::Entity::find()
        .all(db)
        .await
        .ok()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = rmp_serde::to_vec(&library).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(result)
}

async fn reindex(Extension(ref db): Extension<DatabaseConnection>) -> Result<(), StatusCode> {
    super::fetching::reindex(&db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_stream(
    Path(hash): Path<u32>,
    Extension(ref db): Extension<DatabaseConnection>,
) -> Result<([(HeaderName, String); 1], Vec<u8>), StatusCode> {
    let path = path_from_hash(&db, hash)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let data = std::fs::read(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mime = mime_guess::from_path(&path)
        .first()
        .map(|v| v.to_string())
        .unwrap_or("application/octet-stream".into());

    Ok(([(header::CONTENT_TYPE, mime)], data))
}

async fn get_album_art(
    Path(hash): Path<u32>,
    Extension(ref db): Extension<DatabaseConnection>,
) -> Result<([(HeaderName, String); 1], Vec<u8>), StatusCode> {
    let path = path_from_hash(&db, hash)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let tagged_file = lofty::read_from_path(path, false).map_err(|_| StatusCode::NOT_FOUND)?;

    let tags = tagged_file
        .primary_tag()
        .unwrap_or(tagged_file.first_tag().ok_or(StatusCode::NOT_FOUND)?);

    let picture = tags.pictures().get(0).ok_or(StatusCode::NOT_FOUND)?;

    Ok((
        [(header::CONTENT_TYPE, picture.mime_type().to_string())],
        picture.data().to_vec(),
    ))
}

pub async fn add_user(
    db: &DatabaseConnection,
    username: String,
    password: String,
) -> miette::Result<()> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(16384, 3, 1, None).map_err(|err| {
            return miette!("Couldn't initialize argon2 parameters: {}", err.to_string());
        })?,
    );

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| return miette!("Couldn't hash password: {}", err.to_string()))?
        .to_string();

    let user = users::ActiveModel {
        name: Set(username.clone()),
        password: Set(hash),
        ..Default::default()
    };

    users::Entity::insert(user)
        .on_conflict(
            sea_query::OnConflict::column(users::Column::Name)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await
        .into_diagnostic()?;

    success!("User {username} added");

    Ok(())
}

pub async fn remove_user(db: &DatabaseConnection, username: String) -> miette::Result<()> {
    let user = users::Entity::find()
        .filter(users::Column::Name.eq(username.clone()))
        .one(db)
        .await
        .into_diagnostic()?;

    user.ok_or(miette!("User {} not found", username))?
        .delete(db)
        .await
        .into_diagnostic()?;

    success!("User {username} removed");

    Ok(())
}

fn verify_password(password: &str, hash: &str) -> miette::Result<bool> {
    let hash = PasswordHash::new(hash)
        .map_err(|err| return miette!("Couldn't parse password hash: {}", err.to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .is_ok())
}

async fn authenticate(
    db: &DatabaseConnection,
    auth: Authorization<Basic>,
) -> Result<(), StatusCode> {
    let user = users::Entity::find()
        .filter(users::Column::Name.eq(auth.username()))
        .one(db)
        .await
        .ok()
        .flatten()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Compare the provided password with the password hash stored in the database
    let authorized = verify_password(auth.password(), &user.password)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if authorized {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn auth<B: std::marker::Send>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let req = RequestParts::new(req);

    let auth = req
        .headers()
        .typed_get::<Authorization<Basic>>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let db: &DatabaseConnection = req
        .extensions()
        .get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(error) = authenticate(db, auth).await {
        Err(error)
    } else {
        let req = req
            .try_into_request()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let res = next.run(req).await;
        Ok(res)
    }
}
