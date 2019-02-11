// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// TODO: Origins is still huge ... should it break down further into
// sub-resources?

use std::collections::HashMap;
use std::str::from_utf8;

use actix_web::http::header::{
    Charset, ContentDisposition, DispositionParam, DispositionType, ExtendedValue,
};
use actix_web::http::{self, Method, StatusCode};
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, Json, Path, Query};
use actix_web::{AsyncResponder, FutureResponse, HttpMessage};
use bytes::Bytes;
use diesel::pg::PgConnection;
use diesel::result::Error::NotFound;
use futures::future::Future;
use serde_json;

use crate::bldr_core;
use crate::hab_core::crypto::keys::{parse_key_str, parse_name_with_rev, PairType};
use crate::hab_core::crypto::{BoxKeyPair, SigKeyPair};
use crate::hab_core::package::{ident, PackageIdent};

use crate::protocol::originsrv::OriginKeyIdent;

use crate::db::models::account::*;
use crate::db::models::integration::*;
use crate::db::models::invitations::*;
use crate::db::models::keys::*;
use crate::db::models::origin::*;
use crate::db::models::package::{BuilderPackageIdent, ListPackages, Package, PackageVisibility};
use crate::db::models::secrets::*;

use crate::server::authorize::{authorize_session, check_origin_empty, check_origin_owner};
use crate::server::error::{Error, Result};
use crate::server::framework::headers;
use crate::server::helpers::{self, Pagination};
use crate::server::resources::pkgs::postprocess_package_list;
use crate::server::AppState;

#[derive(Clone, Serialize, Deserialize)]
struct OriginSecretPayload {
    #[serde(default)]
    name: String,
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
    //
    // Route registration
    //
    pub fn register(app: App<AppState>) -> App<AppState> {
        app.route("/depot/{origin}/pkgs", Method::GET, list_unique_packages)
            .route("/depot/origins/{origin}", Method::GET, get_origin)
            .route("/depot/origins/{origin}", Method::PUT, update_origin)
            .route("/depot/origins/{origin}", Method::DELETE, delete_origin)
            .route("/depot/origins", Method::POST, create_origin)
            .route(
                "/depot/origins/{origin}/users",
                Method::GET,
                list_origin_members,
            )
            .route(
                "/depot/origins/{origin}/users/{user}",
                http::Method::DELETE,
                origin_member_delete,
            )
            .route(
                "/depot/origins/{origin}/invitations",
                Method::GET,
                list_origin_invitations,
            )
            .route(
                "/depot/origins/{origin}/users/{username}/invitations",
                Method::POST,
                invite_to_origin,
            )
            .route(
                "/depot/origins/{origin}/invitations/{invitation_id}",
                Method::PUT,
                accept_invitation,
            )
            .route(
                "/depot/origins/{origin}/invitations/{invitation_id}",
                Method::DELETE,
                rescind_invitation,
            )
            .route(
                "/depot/origins/{origin}/invitations/{invitation_id}/ignore",
                Method::PUT,
                ignore_invitation,
            )
            .route(
                "/depot/origins/{origin}/keys/latest",
                Method::GET,
                download_latest_origin_key,
            )
            .route("/depot/origins/{origin}/keys", Method::POST, create_keys)
            .route(
                "/depot/origins/{origin}/keys",
                Method::GET,
                list_origin_keys,
            )
            .route(
                "/depot/origins/{origin}/keys/{revision}",
                http::Method::POST,
                upload_origin_key,
            )
            .route(
                "/depot/origins/{origin}/keys/{revision}",
                http::Method::GET,
                download_origin_key,
            )
            .route(
                "/depot/origins/{origin}/secret",
                Method::GET,
                list_origin_secrets,
            )
            .route(
                "/depot/origins/{origin}/secret",
                Method::POST,
                create_origin_secret,
            )
            .route(
                "/depot/origins/{origin}/encryption_key",
                Method::GET,
                download_latest_origin_encryption_key,
            )
            .route(
                "/depot/origins/{origin}/integrations",
                Method::GET,
                fetch_origin_integrations,
            )
            .route(
                "/depot/origins/{origin}/secret/{secret}",
                Method::DELETE,
                delete_origin_secret,
            )
            .route(
                "/depot/origins/{origin}/secret_keys/latest",
                Method::GET,
                download_latest_origin_secret_key,
            )
            .route(
                "/depot/origins/{origin}/secret_keys/{revision}",
                Method::POST,
                upload_origin_secret_key,
            )
            .route(
                "/depot/origins/{origin}/integrations/{integration}/names",
                Method::GET,
                fetch_origin_integration_names,
            )
            .route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::GET,
                get_origin_integration,
            )
            .route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::DELETE,
                delete_origin_integration,
            )
            .route(
                "/depot/origins/{origin}/integrations/{integration}/{name}",
                Method::PUT,
                create_origin_integration_async,
            )
    }
}

//
// Route handlers - these functions can return any Responder trait
//
#[allow(clippy::needless_pass_by_value)]
fn get_origin(req: HttpRequest<AppState>) -> HttpResponse {
    let origin_name = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Origin::get(&origin_name, &*conn) {
        Ok(origin) => HttpResponse::Ok()
            .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
            .json(origin),
        Err(NotFound) => HttpResponse::NotFound().into(),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin(
    (req, body): (HttpRequest<AppState>, Json<CreateOriginHandlerReq>),
) -> HttpResponse {
    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    let dpv = match body.clone().default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    if !ident::is_valid_origin_name(&body.name) {
        return HttpResponse::ExpectationFailed().into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let new_origin = NewOrigin {
        name: &body.0.name,
        owner_id: session.get_id() as i64,
        default_package_visibility: &dpv,
    };

    match Origin::create(&new_origin, &*conn).map_err(Error::DieselError) {
        Ok(origin) => HttpResponse::Created().json(origin),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn update_origin(
    (req, body): (HttpRequest<AppState>, Json<UpdateOriginHandlerReq>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let dpv = match body.0.default_package_visibility {
        Some(viz) => viz,
        None => PackageVisibility::Public,
    };

    match Origin::update(&origin, dpv, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let session = match authorize_session(&req, None) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    debug!("Request to delete origin {}", &origin);

    if !check_origin_empty(&req, &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match Origin::delete(&origin, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().into(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_keys(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let pair =
        SigKeyPair::generate_pair_for_origin(&origin).expect("failed to generate origin key pair");

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let pk_body = match pair.to_public_string().map_err(Error::HabitatCore) {
        Ok(pk) => pk.into_bytes(),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let new_pk = NewOriginPublicSigningKey {
        owner_id: account_id as i64,
        origin: &origin,
        full_name: &format!("{}-{}", &origin, &pair.rev),
        name: &origin,
        revision: &pair.rev,
        body: &pk_body,
    };

    match OriginPublicSigningKey::create(&new_pk, &*conn).map_err(Error::DieselError) {
        Ok(_) => (),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    }

    let sk_body = match pair.to_secret_string().map_err(Error::HabitatCore) {
        Ok(sk) => sk.into_bytes(),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let new_sk = NewOriginPrivateSigningKey {
        owner_id: account_id as i64,
        origin: &origin,
        full_name: &format!("{}-{}", &origin, &pair.rev),
        name: &origin,
        revision: &pair.rev,
        body: &sk_body,
    };

    match OriginPrivateSigningKey::create(&new_sk, &*conn).map_err(Error::DieselError) {
        Ok(_) => (),
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    }

    HttpResponse::Created().finish()
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_keys(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginPublicSigningKey::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            let list: Vec<OriginKeyIdent> = list
                .iter()
                .map(|key| {
                    let mut ident = OriginKeyIdent::new();
                    ident.set_location(format!("/origins/{}/keys/{}", &key.name, &key.revision));
                    ident.set_origin(key.name.to_string());
                    ident.set_revision(key.revision.to_string());
                    ident
                })
                .collect();

            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(&list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn upload_origin_key((req, body): (HttpRequest<AppState>, String)) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match parse_key_str(&body) {
        Ok((PairType::Public, _, _)) => {
            debug!("Received a valid public key");
        }
        Ok(_) => {
            debug!("Received a secret key instead of a public key");
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
        Err(e) => {
            debug!("Invalid public key content: {}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    let new_pk = NewOriginPublicSigningKey {
        owner_id: account_id as i64,
        origin: &origin,
        full_name: &format!("{}-{}", &origin, &revision),
        name: &origin,
        revision: &revision,
        body: &body.into_bytes(),
    };

    match OriginPublicSigningKey::create(&new_pk, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created()
            .header(http::header::LOCATION, format!("{}", req.uri()))
            .body(format!("/origins/{}/keys/{}", &origin, &revision)),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_origin_key(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key =
        match OriginPublicSigningKey::get(&origin, &revision, &*conn).map_err(Error::DieselError) {
            Ok(key) => key,
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPublicSigningKey::latest(&origin, &*conn).map_err(Error::DieselError) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_secrets(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            // Need to map to different struct for hab cli backward compat
            let new_list: Vec<OriginSecretWithOriginId> =
                list.into_iter().map(|s| s.into()).collect();
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(&new_list)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin_secret(
    (req, body): (HttpRequest<AppState>, Json<OriginSecretPayload>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id() as i64,
        Err(err) => return err.into(),
    };

    if body.name.is_empty() {
        return HttpResponse::with_body(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Missing value for field `name`",
        );
    }

    if body.value.is_empty() {
        return HttpResponse::with_body(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Missing value for field `value`",
        );
    }

    // get metadata from secret payload
    let secret_metadata = match BoxKeyPair::secret_metadata(&body.value) {
        Ok(res) => res,
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::with_body(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Failed to get metadata from payload: {}", err),
            );
        }
    };

    debug!("Secret Metadata: {:?}", secret_metadata);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    // fetch the private origin encryption key from the database
    let priv_key =
        match OriginPrivateEncryptionKey::get(&origin, &*conn).map_err(Error::DieselError) {
            Ok(key) => {
                let key_str = from_utf8(&key.body).unwrap();
                match BoxKeyPair::secret_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(err) => {
                        debug!("{}", err);
                        return HttpResponse::with_body(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            format!("Failed to get secret from payload: {}", err),
                        );
                    }
                }
            }
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let (name, rev) = match parse_name_with_rev(secret_metadata.sender) {
        Ok(val) => val,
        Err(e) => {
            return HttpResponse::with_body(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("Failed to parse name and revision: {}", e),
            );
        }
    };

    debug!("Using key {:?}-{:?}", name, &rev);

    // fetch the public origin encryption key from the database
    let pub_key =
        match OriginPublicEncryptionKey::get(&origin, &rev, &*conn).map_err(Error::DieselError) {
            Ok(key) => {
                let key_str = from_utf8(&key.body).unwrap();
                match BoxKeyPair::public_key_from_str(key_str) {
                    Ok(key) => key,
                    Err(err) => {
                        debug!("{}", err);
                        return HttpResponse::with_body(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            format!("{}", err),
                        );
                    }
                }
            }
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let box_key_pair = BoxKeyPair::new(name, rev.clone(), Some(pub_key), Some(priv_key));

    debug!("Decrypting string: {:?}", &secret_metadata.ciphertext);

    // verify we can decrypt the message
    match box_key_pair.decrypt(&secret_metadata.ciphertext, None, None) {
        Ok(_) => (),
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::with_body(StatusCode::UNPROCESSABLE_ENTITY, format!("{}", err));
        }
    };

    match OriginSecret::create(
        &NewOriginSecret {
            origin: &origin,
            name: &body.name,
            value: &body.value,
            owner_id: account_id,
        },
        &*conn,
    )
    .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin_secret(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, secret) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginSecret::delete(&origin, &secret, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn upload_origin_secret_key(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.body()
        .from_err()
        .and_then(move |bytes: Bytes| Ok(do_upload_origin_secret_key(&req, &bytes)))
        .responder()
}

#[allow(clippy::needless_pass_by_value)]
fn do_upload_origin_secret_key(req: &HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
    let (origin, revision) = Path::<(String, String)>::extract(req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match String::from_utf8(body.to_vec()) {
        Ok(content) => match parse_key_str(&content) {
            Ok((PairType::Secret, _, _)) => {
                debug!("Received a valid secret key");
            }
            Ok(_) => {
                debug!("Received a public key instead of a secret key");
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
            Err(e) => {
                debug!("Invalid secret key content: {}", e);
                return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
            }
        },
        Err(e) => {
            debug!("Can't parse secret key upload content: {}", e);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    }

    let new_sk = NewOriginPrivateSigningKey {
        owner_id: account_id as i64,
        origin: &origin,
        name: &origin,
        full_name: &format!("{}-{}", &origin, &revision),
        revision: &revision,
        body,
    };

    match OriginPrivateSigningKey::create(&new_sk, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_secret_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPrivateSigningKey::get(&origin, &*conn).map_err(Error::DieselError) {
        Ok(key) => key,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let xfilename = format!("{}-{}.sig.key", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn list_unique_packages(
    (req, pagination): (HttpRequest<AppState>, Query<Pagination>),
) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let opt_session_id = match authorize_session(&req, None) {
        Ok(session) => Some(session.get_id()),
        Err(_) => None,
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
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

    let lpr = ListPackages {
        ident: BuilderPackageIdent(ident.clone()),
        visibility: helpers::visibility_for_optional_session(&req, opt_session_id, &origin),
        page: page as i64,
        limit: per_page as i64,
    };

    match Package::distinct_for_origin(lpr, &*conn) {
        Ok((packages, count)) => postprocess_package_list(&req, &packages, count, &pagination),
        Err(err) => {
            debug!("{}", err);
            Error::DieselError(err).into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn download_latest_origin_encryption_key(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let key = match OriginPublicEncryptionKey::latest(&origin, &*conn) {
        Ok(key) => key,
        Err(NotFound) => {
            // TODO: redesign to not be generating keys during d/l
            match generate_origin_encryption_keys(&origin, account_id, &conn) {
                Ok(key) => key,
                Err(err) => {
                    debug!("{}", err);
                    return err.into();
                }
            }
        }
        Err(err) => {
            debug!("{}", err);
            return Error::DieselError(err).into();
        }
    };

    let xfilename = format!("{}-{}.pub", key.name, key.revision);
    download_content_as_file(&key.body, xfilename)
}

#[allow(clippy::needless_pass_by_value)]
fn invite_to_origin(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, user) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    debug!("Creating invitation for user {} origin {}", &user, &origin);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let (recipient_id, recipient_name) =
        match Account::get(&user, &*conn).map_err(Error::DieselError) {
            Ok(account) => (account.id, account.name),
            Err(err) => {
                debug!("{}", err);
                return err.into();
            }
        };

    let new_invitation = NewOriginInvitation {
        origin: &origin,
        account_id: recipient_id,
        account_name: &recipient_name,
        owner_id: account_id as i64,
    };

    // store invitations in the originsrv
    match OriginInvitation::create(&new_invitation, &*conn).map_err(Error::DieselError) {
        Ok(invitation) => HttpResponse::Created().json(&invitation),
        // TODO (SA): Check for error case where invitation already exists
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn accept_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let account_id = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(_) => return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY),
    };

    debug!(
        "Accepting invitation for user {} origin {}",
        account_id, origin
    );

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::accept(invitation_id, false, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn ignore_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let _ = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    debug!(
        "Ignoring invitation id {} for origin {}",
        invitation_id, &origin
    );

    match OriginInvitation::ignore(invitation_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn rescind_invitation(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, invitation) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let _ = match authorize_session(&req, None) {
        Ok(session) => session.get_id(),
        Err(err) => return err.into(),
    };

    let invitation_id = match invitation.parse::<u64>() {
        Ok(invitation_id) => invitation_id,
        Err(err) => {
            debug!("{}", err);
            return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
        }
    };

    debug!(
        "Rescinding invitation id {} for user from origin {}",
        invitation_id, &origin
    );

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::rescind(invitation_id, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_invitations(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginInvitation::list_by_origin(&origin, &*conn).map_err(Error::DieselError) {
        Ok(list) => {
            let json = json!({
                "origin": &origin,
                "invitations": serde_json::to_value(list).unwrap()
            });

            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn list_origin_members(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginMember::list(&origin, &*conn).map_err(Error::DieselError) {
        Ok(users) => {
            let json = json!({
                "origin": &origin,
                "members": serde_json::to_value(users).unwrap()
            });

            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(json)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn origin_member_delete(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, user) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    let session = match authorize_session(&req, Some(&origin)) {
        Ok(session) => session,
        Err(err) => return err.into(),
    };

    if !check_origin_owner(&req, session.get_id(), &origin).unwrap_or(false) {
        return HttpResponse::new(StatusCode::FORBIDDEN);
    }

    // Do not allow the owner to be removed which would orphan the origin
    if user == session.get_name() {
        return HttpResponse::new(StatusCode::UNPROCESSABLE_ENTITY);
    }

    debug!("Deleting origin member {} from origin {}", &user, &origin);

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginMember::delete(&origin, &user, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fetch_origin_integrations(req: HttpRequest<AppState>) -> HttpResponse {
    let origin = Path::<String>::extract(&req).unwrap().into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin(&origin, &*conn).map_err(Error::DieselError) {
        Ok(oir) => {
            let integrations_response: HashMap<String, Vec<String>> =
                oir.iter().fold(HashMap::new(), |mut acc, ref i| {
                    acc.entry(i.integration.to_owned())
                        .or_insert_with(Vec::new)
                        .push(i.name.to_owned());
                    acc
                });
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(integrations_response)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn fetch_origin_integration_names(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration) = Path::<(String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::list_for_origin_integration(&origin, &integration, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(integrations) => {
            let names: Vec<String> = integrations.iter().map(|i| i.name.to_string()).collect();
            let mut hm: HashMap<String, Vec<String>> = HashMap::new();
            hm.insert("names".to_string(), names);
            HttpResponse::Ok()
                .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                .json(hm)
        }
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin_integration_async(req: HttpRequest<AppState>) -> FutureResponse<HttpResponse> {
    req.body()
        .from_err()
        .and_then(move |bytes: Bytes| Ok(create_origin_integration(req, &bytes)))
        .responder()
}

#[allow(clippy::needless_pass_by_value)]
fn create_origin_integration(req: HttpRequest<AppState>, body: &Bytes) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    let encrypted = match encrypt(&req, &body) {
        Ok(encrypted) => encrypted,
        Err(err) => {
            debug!("{}", err);
            return err.into();
        }
    };

    let noi = NewOriginIntegration {
        origin: &origin,
        integration: &integration,
        name: &name,
        body: &encrypted,
    };

    match OriginIntegration::create(&noi, &*conn).map_err(Error::DieselError) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn delete_origin_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::delete(&origin, &integration, &name, &*conn)
        .map_err(Error::DieselError)
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
fn get_origin_integration(req: HttpRequest<AppState>) -> HttpResponse {
    let (origin, integration, name) = Path::<(String, String, String)>::extract(&req)
        .unwrap()
        .into_inner(); // Unwrap Ok

    if let Err(err) = authorize_session(&req, Some(&origin)) {
        return err.into();
    }

    let conn = match req.state().db.get_conn().map_err(Error::DbError) {
        Ok(conn_ref) => conn_ref,
        Err(err) => return err.into(),
    };

    match OriginIntegration::get(&origin, &integration, &name, &*conn).map_err(Error::DieselError) {
        Ok(integration) => match decrypt(&req, &integration.body) {
            Ok(decrypted) => {
                let val = serde_json::from_str(&decrypted).unwrap();
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
                    .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
                    .json(sanitized)
            }
            Err(err) => {
                debug!("{}", err);
                err.into()
            }
        },
        Err(err) => {
            debug!("{}", err);
            err.into()
        }
    }
}

//
// Internal helpers
//

fn download_content_as_file(content: &[u8], filename: String) -> HttpResponse {
    HttpResponse::Ok()
        .header(
            http::header::CONTENT_DISPOSITION,
            ContentDisposition {
                disposition: DispositionType::Attachment,
                parameters: vec![DispositionParam::FilenameExt(ExtendedValue {
                    charset: Charset::Iso_8859_1, // The character set for the bytes of the filename
                    language_tag: None, // The optional language tag (see `language-tag` crate)
                    value: filename.as_bytes().to_vec(), // the actual bytes of the filename
                })],
            },
        )
        .header(
            http::header::HeaderName::from_static(headers::XFILENAME),
            filename,
        )
        .header(http::header::CACHE_CONTROL, headers::NO_CACHE)
        .body(Bytes::from(content))
}

fn generate_origin_encryption_keys(
    origin: &str,
    session_id: u64,
    conn: &PgConnection,
) -> Result<OriginPublicEncryptionKey> {
    debug!("Generating encryption keys for {}", origin);

    let pair = BoxKeyPair::generate_pair_for_origin(origin).map_err(Error::HabitatCore)?;

    let pk_body = pair
        .to_public_string()
        .map_err(Error::HabitatCore)?
        .into_bytes();

    let new_pk = NewOriginPublicEncryptionKey {
        owner_id: session_id as i64,
        name: &origin,
        origin: &origin,
        full_name: &format!("{}-{}", &origin, &pair.rev),
        revision: &pair.rev,
        body: &pk_body,
    };

    let sk_body = pair
        .to_secret_string()
        .map_err(Error::HabitatCore)?
        .into_bytes();

    let new_sk = NewOriginPrivateEncryptionKey {
        owner_id: session_id as i64,
        name: origin,
        origin: &origin,
        full_name: &format!("{}-{}", &origin, &pair.rev),
        revision: &pair.rev,
        body: &sk_body,
    };

    OriginPrivateEncryptionKey::create(&new_sk, &*conn)?;
    OriginPublicEncryptionKey::create(&new_pk, &*conn).map_err(Error::DieselError)
}

fn encrypt(req: &HttpRequest<AppState>, content: &Bytes) -> Result<String> {
    bldr_core::integrations::encrypt(&req.state().config.api.key_path, content)
        .map_err(Error::BuilderCore)
}

fn decrypt(req: &HttpRequest<AppState>, content: &str) -> Result<String> {
    let bytes = bldr_core::integrations::decrypt(&req.state().config.api.key_path, content)?;
    Ok(String::from_utf8(bytes)?)
}
