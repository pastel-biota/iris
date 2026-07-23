use std::collections::HashMap;

use anyhow::Context as _;
use tokio::sync::RwLock;

use crate::{auth::endpoint::{LoginBody, LoginEndpoint, MeEndpoint}, federation::protocol::RequestError, ingest::api::{get_image::{GetImageEndpoint, GetImageRequest}, get_photo_meta::{GetPhotoMetaEndpoint, GetPhotoMetaRequest}, get_photos_list::{GetPhotosListEndpoint, GetPhotosListQuery}}, model::{EntityName, Identifier, PhotoMeta, PhotoOrigin, PhotoReference, RemoteOrigin}, repository::{config::{FederationConfig, FederationHost}, io::LengthedStream}};

pub struct FederatedPhotoIndex {
    pub config: FederationConfig,
    session: RwLock<HashMap<String, String>>,
}

#[derive(Clone)]
pub struct PagedPhotoRefList {
    pub photos: Vec<PhotoReference>,
    pub next_cursor: Option<Identifier>,
    pub total_count: u32,
}

impl FederatedPhotoIndex {
    pub fn new(config: FederationConfig) -> Self {
        Self {
            config,
            session: Default::default(),
        }
    }

    pub async fn list_photos(&self, name: &EntityName, size: Option<u32>, cursor: Option<Identifier>) -> anyhow::Result<PagedPhotoRefList> {
        let host = self.config.hosts.get(name)
            .with_context(|| format!("No such host configured: {name}"))?
            .clone();
        let session = self.ensure_logged_in(&host)
            .await
            .with_context(|| format!("Failed to log in to the host: {name}"))?;

        let cred = crate::federation::protocol::request::<GetPhotosListEndpoint>(
            Some(&session),
            &host.origin,
            GetPhotosListQuery { cursor, size }
        ).await.context("Failed to retrieve the photo list from the federated host")?;

        Ok(PagedPhotoRefList {
            photos: cred.photos.into_iter().map(|photo| {
                let mut photo_ref = PhotoReference::from(photo);
                photo_ref.origin = PhotoOrigin::Federated(
                    RemoteOrigin {
                        federator: name.clone(),
                        identifier: photo_ref.origin.id().clone()
                    }
                );

                photo_ref
            }).collect(),
            next_cursor: cred.next_cursor,
            total_count: cred.total_count,
        })
    }

    pub async fn get_photos_meta(&self, origin: &RemoteOrigin) -> anyhow::Result<PhotoMeta> {
        let host = self.config.hosts.get(&origin.federator)
            .with_context(|| format!("No such host configured: {}", &origin.federator))?
            .clone();
        let session = self.ensure_logged_in(&host)
            .await
            .with_context(|| format!("Failed to log in to the host: {}", &origin.federator))?;

        let photo = crate::federation::protocol::request::<GetPhotoMetaEndpoint>(
            Some(&session),
            &host.origin,
            GetPhotoMetaRequest { photo_id: origin.identifier.clone() }
        ).await.context("Failed to retrieve the photo meta from the federated host")?;

        let mut photo = PhotoMeta::from(photo.photo);
        photo.origin = PhotoOrigin::Federated(
            RemoteOrigin {
                federator: origin.federator.clone(),
                identifier: photo.origin.id().clone()
            }
        );

        Ok(photo)
    }

    pub async fn get_photo_image(
        &self,
        origin: &RemoteOrigin,
        image_id: &str
    ) -> anyhow::Result<LengthedStream> {
        let host = self.config.hosts.get(&origin.federator)
            .with_context(|| format!("No such host configured: {}", &origin.federator))?
            .clone();
        let session = self.ensure_logged_in(&host)
            .await
            .with_context(|| format!("Failed to log in to the host: {}", &origin.federator))?;

        let photo = crate::federation::protocol::request_stream::<GetImageEndpoint>(
            Some(session),
            host.origin.clone(),
            GetImageRequest { photo_id: origin.identifier.clone(), image_id: image_id.to_string() }
        ).await.context("Failed to retrieve the image from the federated host")?;

        Ok(photo)
    }

    async fn login(&self, host: &FederationHost) -> anyhow::Result<String> {
        let cred = crate::federation::protocol::request::<LoginEndpoint>(
            None,
            &host.origin,
            LoginBody {
                username: host.username.clone(),
                password: host.password.clone(),
            }
        ).await.context("Failed to log in to the federated host")?;

        self.session.write().await.insert(host.origin.clone(), cred.session_key.clone());

        Ok(cred.session_key)
    }

    async fn ensure_logged_in(&self, host: &FederationHost) -> anyhow::Result<String> {
        // TODO: Handle session expire

        if let Some(session) = self.verify_token(host).await? {
            return Ok(session);
        }

        self.login(host).await?;

        let token = self.verify_token(host)
            .await?
            .context("Could not refresh the token")?;

        Ok(token)
    }

    async fn verify_token(&self, host: &FederationHost) -> anyhow::Result<Option<String>> {
        let Some(session) = self.session.read().await.get(&host.origin).cloned() else {
            return Ok(None);
        };

        let me = crate::federation::protocol::request::<MeEndpoint>(
            Some(&session),
            &host.origin,
            ()
        ).await;

        match me {
            Ok(_) => Ok(Some(session)),
            Err(RequestError::Applictaion(http::StatusCode::UNAUTHORIZED, _)) => Ok(None),
            Err(RequestError::Applictaion(code, reason)) => Err(anyhow::anyhow!("The request to /auth/me failed with {code}: {reason}")),
            Err(RequestError::Anyhow(anyhow)) => Err(anyhow)
        }
    }
}

