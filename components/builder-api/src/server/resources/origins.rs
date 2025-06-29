// TODO: Origins is still huge ... should it break down further into
// sub-resources?

use crate::{bldr_core::crypto,
            db::models::{account::*,
                         channel::Channel,
                         integration::*,
                         invitations::*,
                         keys as db_keys,
                         origin::*,
                         package::{BuilderPackageIdent,
                                   ListPackages,
                                   Package,
                                   PackageVisibility},
                         projects::Project,
                         secrets::*,
                         settings::OriginPackageSettings},
            protocol::originsrv::OriginKeyIdent,
            server::{authorize::{authorize_session,
                                 check_origin_member,
                                 check_origin_owner},
                     error::{Error,
                             Result},
                     framework::headers,
                     helpers::{self,
                               role_results_json,
                               Pagination,
                               Role},
                     resources::pkgs::postprocess_package_list,
                     AppState}};
use actix_web::{body::BoxBody,
                http::{self,
                       header::{Charset,
                                ContentDisposition,
                                DispositionParam,
                                DispositionType,
                                ExtendedValue},
                       StatusCode},
                web::{self,
                      Bytes as ActixBytes,
                      Data,
                      Json,
                      Path,
                      Query,
                      ServiceConfig},
                HttpRequest,
                HttpResponse};
use builder_core::Error::OriginDeleteError;
use bytes::Bytes;
use diesel::{pg::PgConnection,
             result::Error::NotFound};
use biome_core::{crypto::keys::{self as core_keys,
                                  generate_origin_encryption_key_pair,
                                  generate_signing_key_pair,
                                  AnonymousBox,
                                  Key,
                                  KeyCache,
                                  KeyFile},
                   package::{ident,
                             PackageIdent}};
use std::{collections::HashMap,
          convert::TryInto,
          str::FromStr};

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    #[serde(default)]
    name:  String,
    #[serde(default)]
    value: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateOriginHandlerReq {
    pub name: String,
    pub default_package_visibility: Option<PackageVisibility>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOriginHandlerReq {
    pub default_package_visibility: Option<PackageVisibility>,
}

pub struct Origins {}

impl Origins {
    // Route registration
    //
    pub fn register(cfg: &mut ServiceConfig) {
        cfg.route("/depot/{origin}/pkgs", web::get().to(list_unique_packages))
           .route("/depot/origins/{origin}", web::get().to(get_origin))
           .route("/depot/origins/{origin}", web::put().to(update_origin))
           .route("/depot/origins/{origin}", web::delete().to(delete_origin))
           .route("/depot/origins", web::post().to(create_origin))
           .route("/depot/origins/{origin}/users",
                  web::get().to(list_origin_members))
           .route("/depot/origins/{origin}/users/{user}",
                  web::delete().to(origin_member_delete))
           .route("/depot/origins/{origin}/transfer/{user}",
                  web::post().to(transfer_origin_ownership))
           .route("/depot/origins/{origin}/depart",
                  web::post().to(depart_from_origin))
           .route("/depot/origins/{origin}/invitations",
                  web::get().to(list_origin_invitations))
           .route("/depot/origins/{origin}/users/{username}/invitations",
                  web::post().to(invite_to_origin))
           .route("/depot/origins/{origin}/users/{username}/role",
                  web::get().to(get_origin_member_role))
           .route("/depot/origins/{origin}/users/{username}/role",
                  web::put().to(update_origin_member_role))
           .route("/depot/origins/{origin}/invitations/{invitation_id}",
                  web::put().to(accept_invitation))
           .route("/depot/origins/{origin}/invitations/{invitation_id}",
                  web::delete().to(rescind_invitation))
           .route("/depot/origins/{origin}/invitations/{invitation_id}/ignore",
                  web::put().to(ignore_invitation))
           .route("/depot/origins/{origin}/keys/latest",
                  web::get().to(download_latest_origin_key))
           .route("/depot/origins/{origin}/keys", web::post().to(create_keys))
           .route("/depot/origins/{origin}/keys",
                  web::get().to(list_origin_keys))
           .route("/depot/origins/{origin}/keys/{revision}",
                  web::post().to(upload_origin_key))
           .route("/depot/origins/{origin}/keys/{revision}",
                  web::get().to(download_origin_key))
           .route("/depot/origins/{origin}/secret",
                  web::get().to(list_origin_secrets))
           .route("/depot/origins/{origin}/secret",
                  web::post().to(create_origin_secret))
           .route("/depot/origins/{origin}/encryption_key",
                  web::get().to(download_latest_origin_encryption_key))
           .route("/depot/origins/{origin}/integrations",
                  web::get().to(fetch_origin_integrations))
           .route("/depot/origins/{origin}/secret/{secret}",
                  web::delete().to(delete_origin_secret))
           .route("/depot/origins/{origin}/secret_keys/latest",
                  web::get().to(download_latest_origin_secret_key))
           .route("/depot/origins/{origin}/secret_keys/{revision}",
                  web::post().to(upload_origin_secret_key))
           .route("/depot/origins/{origin}/integrations/{integration}/names",
                  web::get().to(fetch_origin_integration_names))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::get().to(get_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::delete().to(delete_origin_integration))
           .route("/depot/origins/{origin}/integrations/{integration}/{name}",
                  web::put().to(create_origin_integration));
    }
}

// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
async fn get_origin(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin_name = path.into_inner();

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Origin::get(&origin_name, &mut conn) {
        Ok(origin) => {
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(origin)
        }
        Err(NotFound) => HttpResponse::NotFound().into(),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn create_origin(req: HttpRequest,
                       body: Json<CreateOriginHandlerReq>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let session = match authorize_session(&req, None, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };
    if !state.config.api.allowed_users_for_origin_create.is_empty()
       && !state.config
                .api
                .allowed_users_for_origin_create
                .contains(&session.get_name().to_string())
    {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    let dpv = match body.clone().default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    if !ident::is_valid_origin_name(&body.name) {
        return HttpResponse::ExpectationFailed().into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let new_origin = NewOrigin { name: &body.0.name,
                                 owner_id: session.get_id() as i64,
                                 default_package_visibility: &dpv, };

    match Origin::create(&new_origin, &mut conn).map_err(Error::DieselError) {
        Ok(origin) => {
            origin_audit(&body.0.name,
                         OriginOperation::OriginCreate,
                         &body.0.name,
                         session.get_id() as i64,
                         session.get_name(),
                         &mut conn);
            HttpResponse::Created().json(origin)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn update_origin(req: HttpRequest,
                       path: Path<String>,
                       body: Json<UpdateOriginHandlerReq>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let dpv = match body.0.default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    match Origin::update(&origin, dpv, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_origin(req: HttpRequest,
                       path: Path<String>,
                       state: Data<AppState>)
                       -> HttpResponse {
    let origin = path.into_inner();

    let session = match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Owner)) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    debug!("Request to delete origin {}", &origin);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // Prior to passing the deletion request to the backend, we validate
    // that the user has already cleaned up the most critical origin data.
    match origin_delete_preflight(&origin, &mut conn) {
        Ok(_) => {
            match Origin::delete(&origin, &mut conn).map_err(Error::DieselError) {
                Ok(_) => {
                    origin_audit(&origin,
                                 OriginOperation::OriginDelete,
                                 &origin,
                                 session.get_id() as i64,
                                 session.get_name(),
                                 &mut conn);
                    HttpResponse::NoContent().into()
                }
                Err(err) => {
                    debug!("Origin {} deletion failed! err = {}", origin, err);
                    // We do not want to expose any database details from diesel
                    // thus we simply return a 409 with an empty body.
                    HttpResponse::new(StatusCode::CONFLICT)
                }
            }
        }
        Err(err) => {
            debug!("Origin preflight determined that {} is not deletable, err = {}!",
                   origin, err);
            // Here we want to enrich the http response with a sanitized error
            // by returning a 409 with a helpful message in the body.
            let body = Bytes::from(format!("{}", err).into_bytes());
            let body = BoxBody::new(body);
            HttpResponse::with_body(StatusCode::CONFLICT, body)
        }
    }
}

fn origin_delete_preflight(origin: &str, conn: &mut PgConnection) -> Result<()> {
    match Project::count_origin_projects(origin, conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} projects remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => return Err(Error::DieselError(e)),
    };

    match OriginMember::count_origin_members(origin, conn) {
        // allow 1 - the origin owner
        Ok(1) => {}
        Ok(count) => {
            let err = format!("There are {} members remaining in origin {}. Only one is allowed.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match OriginSecret::count_origin_secrets(origin, conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} secrets remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match OriginIntegration::count_origin_integrations(origin, conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} integrations remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match Channel::count_origin_channels(origin, conn) {
        // allow 2 - [unstable, stable] channels cannot be deleted
        Ok(2) => {}
        Ok(count) => {
            let err = format!("There are {} channels remaining in origin {}. Only two are \
                               allowed [unstable, stable].",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match Package::count_origin_packages(origin, conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} packages remaining in origin {}. Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    match OriginPackageSettings::count_origin_package_settings(origin, conn) {
        Ok(0) => {}
        Ok(count) => {
            let err = format!("There are {} package settings entries remaining in origin {}. \
                               Must be zero.",
                              count, origin);
            return Err(Error::BuilderCore(OriginDeleteError(err)));
        }
        Err(e) => {
            return Err(Error::DieselError(e));
        }
    };

    Ok(())
}

#[allow(clippy::needless_pass_by_value)]
async fn create_keys(req: HttpRequest, path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => {
            error!("create_keys: Failed to get DB connection, err={}", err);
            return err.into();
        }
    };

    // For Builder, we actually don't want to go through the KeyCache
    // to create a pair, because we don't want to store anything to
    // disk. That's why we have a database.
    let (public, secret) = match origin.parse().map_err(Error::BiomeCore) {
        Ok(o) => generate_signing_key_pair(&o),
        Err(err) => return err.into(),
    };

    if let Err(e) = save_public_origin_signing_key(account_id, &origin, &public, &mut conn) {
        error!("Failed to save public signing key for origin '{}', err={}",
               origin, e);
        return e.into();
    }

    if let Err(e) = save_secret_origin_signing_key(account_id,
                                                   &origin,
                                                   &state.config.api.key_path,
                                                   &secret,
                                                   &mut conn)
    {
        error!("Failed to save secret signing key for origin '{}', err={}",
               origin, e);
        return e.into();
    }

    HttpResponse::Created().finish()
}

#[allow(clippy::needless_pass_by_value)]
async fn list_origin_keys(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match db_keys::OriginPublicSigningKey::list(&origin, &mut conn).map_err(Error::DieselError) {
        Ok(list) => {
            let list: Vec<OriginKeyIdent> =
                list.iter()
                    .map(|key| {
                        let mut ident = OriginKeyIdent::new();
                        ident.set_location(format!("/origins/{}/keys/{}",
                                                   &key.name, &key.revision));
                        ident.set_origin(key.name.to_string());
                        ident.set_revision(key.revision.to_string());
                        ident
                    })
                    .collect();

            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(&list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn upload_origin_key(req: HttpRequest,
                           body: String,
                           path: Path<(String, String)>,
                           state: Data<AppState>)
                           -> HttpResponse {
    let (origin, revision) = path.into_inner();

    // Since this route allows users to upload keys, we verify their membership
    // before we determine if the key they're using actually exists. This is a
    // backward compatibility workaround needed when RBAC was introduced. bio
    // pkg upload optimistically uploads keys as well as packages, but the RBAC
    // changes only give 'members' and 'maintainers' upload permissions for
    // packages, not keys.
    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    if db_keys::OriginPublicSigningKey::get(&origin, &revision, &mut conn).is_ok() {
        HttpResponse::new(StatusCode::CONFLICT)
    } else {
        // In this case we are checking if the user actually has permissions to write a
        // NEW key into the origin_public_keys data table
        let account_id =
            match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
                Ok(session) => session.get_id(),
                Err(_) => {
                    debug!("Unable to upload origin public signing key due to lack of permissions");
                    let body = Bytes::from(format!("You do not have permissions to upload a new \
                                                    origin signing public key: {}-{}",
                                                   origin, revision).into_bytes());
                    let body = BoxBody::new(body);
                    return HttpResponse::with_body(StatusCode::FORBIDDEN, body);
                }
            };
        let key = match body.parse::<core_keys::PublicOriginSigningKey>() {
            Ok(key) => key,
            Err(e) => {
                debug!("Invalid public key content: {}", e);
                let body = Bytes::from_static(b"Invalid origin public key");
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                               BoxBody::new(body));
            }
        };

        match save_public_origin_signing_key(account_id, &origin, &key, &mut conn) {
            Ok(_) => {
                HttpResponse::Created().append_header((http::header::LOCATION,
                                                       format!("{}", req.uri())))
                                       .body(format!("/origins/{}/keys/{}",
                                                     origin,
                                                     key.named_revision().revision()))
            }
            Err(err) => {
                debug!("{}", err);
                err.into()
            }
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn download_origin_key(path: Path<(String, String)>, state: Data<AppState>) -> HttpResponse {
    let (origin, revision) = path.into_inner();

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match get_specific_public_origin_signing_key(&origin, &revision, &mut conn) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    key_as_http_response(&key)
}

#[allow(clippy::needless_pass_by_value)]
async fn download_latest_origin_key(path: Path<String>, state: Data<AppState>) -> HttpResponse {
    let origin = path.into_inner();

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match get_latest_public_origin_signing_key(&origin, &mut conn) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    key_as_http_response(&key)
}

#[allow(clippy::needless_pass_by_value)]
async fn list_origin_secrets(req: HttpRequest,
                             path: Path<String>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::list(&origin, &mut conn).map_err(Error::DieselError) {
        Ok(list) => {
            // Need to map to different struct for bio cli backward compat
            let new_list: Vec<OriginSecretWithOriginId> =
                list.into_iter().map(|s| s.into()).collect();
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(&new_list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn create_origin_secret(req: HttpRequest,
                              body: Json<OriginSecretPayload>,
                              path: Path<String>,
                              state: Data<AppState>)
                              -> HttpResponse {
    let origin = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id() as i64,
            Err(err) => return err.into(),
        };

    if body.name.is_empty() {
        let body = Bytes::from_static(b"Missing value for field `name`");
        let body = BoxBody::new(body);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
    }

    if body.value.is_empty() {
        let body = Bytes::from_static(b"Missing value for field `value`");
        let body = BoxBody::new(body);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
    }

    let anonymous_box = match body.value.parse::<AnonymousBox>() {
        Ok(res) => {
            debug!("Secret Metadata: {:?}", res);
            res
        }
        Err(err) => {
            debug!("{}", err);
            let body =
                Bytes::from(format!("Failed to parse encrypted message from payload: {}", err));
            let body = BoxBody::new(body);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
        }
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key_cache = &state.config.api.key_path;

    // Fetch the origin's secret encryption key from the database
    let secret_encryption_key =
        match get_secret_origin_encryption_key(&origin, key_cache, &mut conn) {
            Ok(key) => key,
            Err(err) => {
                debug!("{}", err);
                let body = Bytes::from(format!("Failed to get secret from payload: {}", err));
                let body = BoxBody::new(body);
                return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
            }
        };

    // Though we're storing the data in its encrypted form, we still
    // need to ensure that we have the ability to decrypt it.
    if let Err(err) = secret_encryption_key.decrypt(&anonymous_box) {
        debug!("{}", err);
        let body = Bytes::from(format!("{}", err).into_bytes());
        let body = BoxBody::new(body);
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, body);
    };

    match OriginSecret::create(&NewOriginSecret { origin:   &origin,
                                                  name:     &body.name,
                                                  value:    &body.value,
                                                  owner_id: account_id, },
                               &mut conn).map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_origin_secret(req: HttpRequest,
                              path: Path<(String, String)>,
                              state: Data<AppState>)
                              -> HttpResponse {
    let (origin, secret) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator))
    {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::delete(&origin, &secret, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn upload_origin_secret_key(req: HttpRequest,
                                  path: Path<(String, String)>,
                                  body: ActixBytes,
                                  state: Data<AppState>)
                                  -> HttpResponse {
    let (origin, _revision) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match String::from_utf8(body.to_vec()) {
        Ok(content) => {
            match content.parse::<core_keys::SecretOriginSigningKey>() {
                Ok(key) => key,
                Err(e) => {
                    debug!("Invalid secret key content: {}", e);
                    let body = Bytes::from_static(b"Invalid origin secret key");
                    return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY,
                                                   BoxBody::new(body));
                }
            }
        }
        Err(e) => {
            debug!("Can't parse secret key upload content: {}", e);
            let body = Bytes::from_static(b"Cannot parse origin secret key");
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    if let Err(e) = save_secret_origin_signing_key(account_id,
                                                   &origin,
                                                   &state.config.api.key_path,
                                                   &key,
                                                   &mut conn)
    {
        error!("Failed to save uploaded secret signing key for origin '{}', err={}",
               origin, e);
        return e.into();
    }

    HttpResponse::Created().finish()
}

#[allow(clippy::needless_pass_by_value)]
async fn download_latest_origin_secret_key(req: HttpRequest,
                                           path: Path<String>,
                                           state: Data<AppState>)
                                           -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match get_latest_secret_origin_signing_key(&origin,
                                                         &state.config.api.key_path,
                                                         &mut conn)
    {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    key_as_http_response(&key)
}

#[allow(clippy::needless_pass_by_value)]
async fn list_unique_packages(req: HttpRequest,
                              pagination: Query<Pagination>,
                              path: Path<String>,
                              state: Data<AppState>)
                              -> HttpResponse {
    let origin = path.into_inner();

    let opt_session_id = match authorize_session(&req, None, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => {
            return {
                debug!("{}", err);
                err.into()
            };
        }
    };

    let ident = PackageIdent::new(origin.clone(), String::from(""), None, None);

    let (page, per_page) = helpers::extract_pagination_in_pages(&pagination);

    let lpr = ListPackages { ident:      BuilderPackageIdent(ident),
                             visibility: helpers::visibility_for_optional_session(&req,
                                                                                  opt_session_id,
                                                                                  &origin),
                             page:       page as i64,
                             limit:      per_page as i64, };

    match Package::distinct_for_origin(&lpr, &mut conn) {
        Ok((packages, count)) => {
            postprocess_package_list(&req, packages.as_slice(), count, &pagination)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn download_latest_origin_encryption_key(req: HttpRequest,
                                               path: Path<String>,
                                               state: Data<AppState>)
                                               -> HttpResponse {
    let origin = path.into_inner();

    let account_id = match authorize_session(&req, Some(&origin), None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key_cache = &state.config.api.key_path;
    let key = match get_latest_public_origin_encryption_key(&origin, &mut conn) {
        Ok(key) => key,
        Err(Error::DieselError(NotFound)) => {
            // TODO: redesign to not be generating keys during d/l
            match generate_origin_encryption_keys(&origin, account_id, key_cache, &mut conn) {
                Ok(key) => key,
                Err(err) => {
                    debug!("{}", err);
                    return err.into();
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    key_as_http_response(&key)
}

#[allow(clippy::needless_pass_by_value)]
async fn invite_to_origin(req: HttpRequest,
                          path: Path<(String, String)>,
                          state: Data<AppState>)
                          -> HttpResponse {
    let (origin, user) = path.into_inner();

    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    debug!("Creating invitation for user {} origin {}", &user, &origin);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (recipient_id, recipient_name) =
        match Account::get(&user, &mut conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let new_invitation = NewOriginInvitation { origin:       &origin,
                                               account_id:   recipient_id,
                                               account_name: &recipient_name,
                                               owner_id:     account_id as i64, };

    // store invitations in the originsrv
    match OriginInvitation::create(&new_invitation, &mut conn).map_err(Error::DieselError) {
        Ok(invitation) => HttpResponse::Created().json(&invitation),
        // TODO (SA): Check for error case where invitation already exists
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn accept_invitation(req: HttpRequest,
                           path: Path<(String, String)>,
                           state: Data<AppState>)
                           -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let account_id = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(_) => {
            let body = Bytes::from(format!("Invalid invitation id '{}'", invitation).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    debug!("Accepting invitation for user {} origin {}",
           account_id, origin);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::accept(invitation_id, false, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn ignore_invitation(req: HttpRequest,
                           path: Path<(String, String)>,
                           state: Data<AppState>)
                           -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let _ = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            let body = Bytes::from(format!("Invalid invitation id '{}'", invitation).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    debug!("Ignoring invitation id {} for origin {}",
           invitation_id, &origin);

    match OriginInvitation::ignore(invitation_id, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn rescind_invitation(req: HttpRequest,
                            path: Path<(String, String)>,
                            state: Data<AppState>)
                            -> HttpResponse {
    let (origin, invitation) = path.into_inner();

    let _ = match authorize_session(&req, None, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            let body = Bytes::from(format!("Invalid invitation id '{}'", invitation).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    debug!("Rescinding invitation id {} for user from origin {}",
           invitation_id, &origin);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::rescind(invitation_id, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn list_origin_invitations(req: HttpRequest,
                                 path: Path<String>,
                                 state: Data<AppState>)
                                 -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::list_by_origin(&origin, &mut conn).map_err(Error::DieselError) {
        Ok(list) => {
            let json = json!({
                "origin": &origin,
                "invitations": serde_json::to_value(list).unwrap()
            });

            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_origin_member_role(req: HttpRequest,
                                path: Path<(String, String)>,
                                state: Data<AppState>)
                                -> HttpResponse {
    let (origin, username) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Member)) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // The account id of the user being requested
    let (target_user_id, _) = match Account::get(&username, &mut conn).map_err(Error::DieselError) {
        Ok(account) => (account.id, account.name),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    match OriginMember::member_role(&origin, target_user_id, &mut conn) {
        Ok(role) => {
            let body = role_results_json(role);

            HttpResponse::Ok().append_header((http::header::CONTENT_TYPE,
                                              headers::APPLICATION_JSON))
                              .append_header((http::header::CACHE_CONTROL,
                                              headers::Cache::NoCache.to_string()))
                              .body(body)
        }
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn update_origin_member_role(req: HttpRequest,
                                   path: Path<(String, String)>,
                                   req_role: Query<Role>,
                                   state: Data<AppState>)
                                   -> HttpResponse {
    let (origin, username) = path.into_inner();
    let target_role = match OriginMemberRole::from_str(&req_role.role) {
        Ok(r) => {
            debug!("role {}", r);
            r
        }
        Err(err) => {
            debug!("{}", err);
            let body =
                Bytes::from(format!("Invalid member role '{}'", &req_role.role).into_bytes());
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
        }
    };

    // Account id of the user making the request
    let account_id =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session.get_id(),
            Err(err) => return err.into(),
        };

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // We cannot allow a user to escalate to Owner. That must be done via Origin owner transfer.
    if target_role == OriginMemberRole::Owner {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // The account id of the user being requested
    let (target_user_id, _) = match Account::get(&username, &mut conn) {
        Ok(account) => (account.id, account.name),
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    };

    // We cannot allow a user to change their own role
    if account_id as i64 == target_user_id {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // We cannot allow a user to change the role of the origin owner
    if check_origin_owner(&req, target_user_id as u64, &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    state.memcache
         .borrow_mut()
         .clear_cache_for_member_role(&origin, target_user_id as u64);

    match OriginMember::update_member_role(&origin, target_user_id, &mut conn, target_role) {
        Ok(0) => HttpResponse::NotFound().into(),
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn transfer_origin_ownership(req: HttpRequest,
                                   path: Path<(String, String)>,
                                   state: Data<AppState>)
                                   -> HttpResponse {
    let (origin, user) = path.into_inner();

    let session = match authorize_session(&req, None, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to transfer ownership to themselves
    if user == session.get_name() {
        let body = Bytes::from_static(b"Cannot transfer origin ownership to self");
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    debug!(" Transferring origin {} to new owner {}", &origin, &user);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (recipient_id, _recipient_name) =
        match Account::get(&user, &mut conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    // Do not allow transfer to recipent that is not already an origin member
    if !check_origin_member(&req, &origin, recipient_id as u64).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    match Origin::transfer(&origin, recipient_id, &mut conn).map_err(Error::DieselError) {
        Ok(_) => {
            origin_audit(&origin,
                         OriginOperation::OwnerTransfer,
                         &recipient_id.to_string(),
                         session.get_id() as i64,
                         session.get_name(),
                         &mut conn);
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn depart_from_origin(req: HttpRequest,
                            path: Path<String>,
                            state: Data<AppState>)
                            -> HttpResponse {
    let origin = path.into_inner();

    let session = match authorize_session(&req, None, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    // Do not allow an origin owner to depart which would orphan the origin
    if check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        let body = Bytes::from_static(b"Departing the owner from the origin is not allowed");
        return HttpResponse::with_body(StatusCode::FORBIDDEN, BoxBody::new(body));
    }

    // Pass a meaningful error in the case that the user isn't a member of origin
    if !check_origin_member(&req, &origin, session.get_id()).unwrap_or(false) {
        let body =
            Bytes::from(format!("Do not have access to the origin '{}'", origin).into_bytes());
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    debug!("Departing user {} from origin {}",
           session.get_name(),
           &origin);

    match Origin::depart(&origin, session.get_id() as i64, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn list_origin_members(req: HttpRequest,
                             path: Path<String>,
                             state: Data<AppState>)
                             -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginMember::list(&origin, &mut conn).map_err(Error::DieselError) {
        Ok(users) => {
            let json = json!({
                "origin": &origin,
                "members": serde_json::to_value(users).unwrap()
            });

            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn origin_member_delete(req: HttpRequest,
                              path: Path<(String, String)>,
                              state: Data<AppState>)
                              -> HttpResponse {
    let (origin, user) = path.into_inner();

    let session =
        match authorize_session(&req, Some(&origin), Some(OriginMemberRole::Administrator)) {
            Ok(session) => session,
            Err(err) => return err.into(),
        };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to be removed which would orphan the origin
    if user == session.get_name() {
        let body = Bytes::from_static(b"Removing the owner is not allowd");
        return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, BoxBody::new(body));
    }

    debug!("Deleting origin member {} from origin {}", &user, &origin);

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (target_account_id, _target_account_name) =
        match Account::get(&user, &mut conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    match OriginMember::delete(&origin, &user, &mut conn).map_err(Error::DieselError) {
        Ok(_) => {
            state.memcache
                 .borrow_mut()
                 .clear_cache_for_member_role(&origin, target_account_id as u64);
            HttpResponse::NoContent().finish()
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn fetch_origin_integrations(req: HttpRequest,
                                   path: Path<String>,
                                   state: Data<AppState>)
                                   -> HttpResponse {
    let origin = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin(&origin, &mut conn).map_err(Error::DieselError) {
        Ok(oir) => {
            let integrations_response: HashMap<String, Vec<String>> =
                oir.iter().fold(HashMap::new(), |mut acc, i| {
                              acc.entry(i.integration.to_owned())
                                 .or_default()
                                 .push(i.name.to_owned());
                              acc
                          });
            HttpResponse::Ok().append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                              .json(integrations_response)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn fetch_origin_integration_names(req: HttpRequest,
                                        path: Path<(String, String)>,
                                        state: Data<AppState>)
                                        -> HttpResponse {
    let (origin, integration) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin_integration(&origin, &integration, &mut conn)
        .map_err(Error::DieselError)
    {
        Ok(integrations) => {
            let names: Vec<String> = integrations.iter().map(|i| i.name.to_string()).collect();
            let mut hm: HashMap<String, Vec<String>> = HashMap::new();
            hm.insert("names".to_string(), names);
            HttpResponse::Ok()
                .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                .json(hm)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn create_origin_integration(req: HttpRequest,
                                   path: Path<(String, String, String)>,
                                   body: ActixBytes,
                                   state: Data<AppState>)
                                   -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (encrypted, _) =
        match crypto::encrypt(&state.config.api.key_path, &body).map_err(Error::BuilderCore) {
            Ok(encrypted) => encrypted,
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let noi = NewOriginIntegration { origin:      &origin,
                                     integration: &integration,
                                     name:        &name,
                                     body:        &encrypted, };

    match OriginIntegration::create(&noi, &mut conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn delete_origin_integration(req: HttpRequest,
                                   path: Path<(String, String, String)>,
                                   state: Data<AppState>)
                                   -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), Some(OriginMemberRole::Maintainer)) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::delete(&origin, &integration, &name, &mut conn).map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
async fn get_origin_integration(req: HttpRequest,
                                path: Path<(String, String, String)>,
                                state: Data<AppState>)
                                -> HttpResponse {
    let (origin, integration, name) = path.into_inner();

    if let Err(err) = authorize_session(&req, Some(&origin), None) {
        return err.into();
    }

    let mut conn = match state.db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::get(&origin, &integration, &name, &mut conn).map_err(Error::DieselError) {
        Ok(integration) => {
            match crypto::decrypt(&state.config.api.key_path, &integration.body)
                .map_err(Error::BuilderCore)
            {
                Ok(decrypted) => {
                    let val = serde_json::from_slice(&decrypted).unwrap();
                    let mut map: serde_json::Map<String, serde_json::Value> =
                        serde_json::from_value(val).unwrap();

                    map.remove("password");

                    let sanitized = json!({
                        "origin": integration.origin.to_string(),
                        "integration": integration.integration.to_string(),
                        "name": integration.name.to_string(),
                        "body": serde_json::to_value(map).unwrap()
                    });

                    HttpResponse::Ok()
                        .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
                        .json(sanitized)
                }
                Err(err) => {
                    debug!("{}", err);
                    err.into()
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

// Internal helpers
//

/// Return a Biome key as a file.
///
/// This is essentially doing what an `actix_web::Responder`
/// implementation for each key type would do, if we could directly
/// implement it on those types (we can't, though, because they're not
/// from this crate!).
fn key_as_http_response<K>(key: &K) -> HttpResponse
    where K: KeyFile
{
    let filename = key.own_filename().display().to_string();
    let contents = key.to_key_string();

    HttpResponse::Ok()
        .append_header((
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::FilenameExt(ExtendedValue {
                    charset: Charset::Iso_8859_1, // The character set for the bytes of the filename
                    language_tag: None, // The optional language tag (see `language-tag` crate)
                    value: filename.clone().into_bytes(), // the actual bytes of the filename
                })],
            },
        ))
        .append_header((
            http::header::HeaderName::from_static(headers::XFILENAME),
            filename,
        ))
        .append_header((http::header::CACHE_CONTROL, headers::NO_CACHE))
        .body(contents)
}

fn generate_origin_encryption_keys(origin: &str,
                                   session_id: u64,
                                   key_cache: &KeyCache,
                                   conn: &mut PgConnection)
                                   -> Result<core_keys::OriginPublicEncryptionKey> {
    debug!("Generating encryption keys for {}", origin);
    let (public, secret) = generate_origin_encryption_key_pair(&origin.parse()?);

    let pk_body = public.to_key_string();
    let new_pk =
        db_keys::NewOriginPublicEncryptionKey { owner_id: session_id as i64,
                                                origin,
                                                name: public.named_revision().name(),
                                                full_name: &public.named_revision().to_string(),
                                                revision: public.named_revision().revision(),
                                                body: &pk_body };

    save_secret_origin_encryption_key(&secret, session_id, key_cache, conn)?;
    db_keys::OriginPublicEncryptionKey::create(&new_pk, &mut *conn)?;

    Ok(public)
}

pub fn save_secret_origin_encryption_key(key: &core_keys::OriginSecretEncryptionKey,
                                         owner_id: u64,
                                         key_cache: &KeyCache,
                                         conn: &mut PgConnection)
                                         -> Result<()> {
    let (encrypted, _bldr_key_rev) = crypto::encrypt(key_cache, key.to_key_string())?;

    let new_sk =
        db_keys::NewOriginPrivateEncryptionKey { owner_id:  owner_id as i64,
                                                 // The name of the key is the origin!
                                                 origin:    key.named_revision().name(),
                                                 name:      key.named_revision().name(),
                                                 full_name: &key.named_revision().to_string(),
                                                 revision:  key.named_revision().revision(),
                                                 body:      &encrypted, };

    db_keys::OriginPrivateEncryptionKey::create(&new_sk, &mut *conn)?;

    Ok(())
}

/// Retrieve an encryption key from the origin
fn get_secret_origin_encryption_key(origin: &str,
                                    key_cache: &KeyCache,
                                    conn: &mut PgConnection)
                                    -> Result<core_keys::OriginSecretEncryptionKey> {
    let db_record = db_keys::OriginPrivateEncryptionKey::latest(origin, &mut *conn)?;
    let decrypted = crypto::decrypt(key_cache, &db_record.body)?;
    Ok(AsRef::<[u8]>::as_ref(&decrypted).try_into()?)
}

/// Retrieve the latest available revision of the origin's public
/// encryption key from the database.
// TODO (CM): NOTE - There doesn't appear to be a way to update origin
// encryption keys, so "latest" is a distinction without a difference.
fn get_latest_public_origin_encryption_key(origin: &str,
                                           conn: &mut PgConnection)
                                           -> Result<core_keys::OriginPublicEncryptionKey> {
    let db_record = db_keys::OriginPublicEncryptionKey::latest(origin, &mut *conn)?;
    Ok(db_record.body.parse()?)
}

fn save_public_origin_signing_key(account_id: u64,
                                  origin: &str,
                                  key: &core_keys::PublicOriginSigningKey,
                                  conn: &mut PgConnection)
                                  -> Result<()> {
    let key_body = key.to_key_string();

    let new_pk = db_keys::NewOriginPublicSigningKey { owner_id: account_id as i64,
                                                      origin,
                                                      full_name: &key.named_revision()
                                                                     .to_string(),
                                                      name: key.named_revision().name(),
                                                      revision: key.named_revision().revision(),
                                                      body: &key_body };

    db_keys::OriginPublicSigningKey::create(&new_pk, &mut *conn)?;
    Ok(())
}

/// Retrieve the latest available revision of the origin's public
/// signing key from the database.
fn get_latest_public_origin_signing_key(origin: &str,
                                        conn: &mut PgConnection)
                                        -> Result<core_keys::PublicOriginSigningKey> {
    let db_record = db_keys::OriginPublicSigningKey::latest(origin, &mut *conn)?;
    Ok(db_record.body.parse()?)
}

/// Retrieve a specific revision of the origin's public
/// signing key from the database.
fn get_specific_public_origin_signing_key(origin: &str,
                                          revision: &str, // TODO (CM): KeyRevision
                                          conn: &mut PgConnection)
                                          -> Result<core_keys::PublicOriginSigningKey> {
    let db_record = db_keys::OriginPublicSigningKey::get(origin, revision, &mut *conn)?;
    Ok(db_record.body.parse()?)
}

fn save_secret_origin_signing_key(account_id: u64,
                                  origin: &str,
                                  key_cache: &KeyCache,
                                  key: &core_keys::SecretOriginSigningKey,
                                  conn: &mut PgConnection)
                                  -> Result<()> {
    // Here we want to encrypt the full contents of the secret signing
    // key (i.e., encrypt the full "file", not merely the
    // cryptographic material) using our Builder encryption key. The
    // resulting bytes are what need to be saved in the database.
    let (sk_encrypted, bldr_key_rev) = crypto::encrypt(key_cache, key.to_key_string())?;

    let new_sk = db_keys::NewOriginPrivateSigningKey { owner_id: account_id as i64,
                                                       origin,
                                                       full_name: &key.named_revision()
                                                                      .to_string(),
                                                       name: key.named_revision().name(),
                                                       revision: key.named_revision().revision(),
                                                       body: &sk_encrypted,
                                                       encryption_key_rev: &bldr_key_rev };

    db_keys::OriginPrivateSigningKey::create(&new_sk, &mut *conn)?;
    Ok(())
}

/// Retrieve the latest secret origin signing key for an origin,
/// decrypting it in the process.
fn get_latest_secret_origin_signing_key(origin: &str,
                                        key_cache: &KeyCache,
                                        conn: &mut PgConnection)
                                        -> Result<core_keys::SecretOriginSigningKey> {
    let db_record = db_keys::OriginPrivateSigningKey::get(origin, &mut *conn)?;

    let key = if db_record.encryption_key_rev.is_some() {
        let bytes = crypto::decrypt(key_cache, &db_record.body)?;
        AsRef::<[u8]>::as_ref(&bytes).try_into()?
    } else {
        db_record.body.parse()?
    };

    Ok(key)
}
