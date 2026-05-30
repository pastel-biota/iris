use std::collections::HashMap;

use futures_util::{Stream, TryStreamExt};
use reqwest_middleware::ClientWithMiddleware;

use crate::{auth::endpoint::{LoginBody, LoginEndpoint}, federation, ingest::api::{get_image::{GetImageEndpoint, GetImageRequest}, get_photo_meta::{GetPhotoMetaEndpoint, GetPhotoMetaRequest}, get_photos_list::{GetPhotosListEndpoint, GetPhotosListQuery}}, model::{EntityName, Identifier, PhotoMeta, PhotoOrigin, PhotoReference, RemoteOrigin}, repository::{config::FederationConfig, io::{LengthedStream, ScopedPath}}};

pub struct FederatedPhotoIndex {
    client: ClientWithMiddleware,
    pub config: FederationConfig,
    session: HashMap<EntityName, String>,
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
            client: crate::federation::request::create_client(),
            config,
            session: Default::default(),
        }
    }

    pub async fn list_photos(&mut self, name: &EntityName, size: Option<u32>, cursor: Option<Identifier>) -> anyhow::Result<PagedPhotoRefList> {
        let host = self.config.hosts.get(name).unwrap().clone();
        let session = self.ensure_logged_in(name).await.unwrap();

        let cred = crate::federation::protocol::request::<GetPhotosListEndpoint>(
            Some(&session),
            &host.origin,
            GetPhotosListQuery { cursor, size }
        ).await.unwrap();

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

    pub async fn get_photos_meta(&mut self, origin: &RemoteOrigin) -> anyhow::Result<PhotoMeta> {
        let host = self.config.hosts.get(&origin.federator).unwrap().clone();
        let session = self.ensure_logged_in(&origin.federator).await.unwrap();

        let photo = crate::federation::protocol::request::<GetPhotoMetaEndpoint>(
            Some(&session),
            &host.origin,
            GetPhotoMetaRequest { photo_id: origin.identifier.clone() }
        ).await.unwrap();

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
        &mut self,
        origin: &RemoteOrigin,
        image_id: &str
    ) -> anyhow::Result<LengthedStream> {
        let host = self.config.hosts.get(&origin.federator).unwrap().clone();
        let session = self.ensure_logged_in(&origin.federator).await.unwrap();

        let photo = crate::federation::protocol::request_stream::<GetImageEndpoint>(
            Some(session),
            host.origin.clone(),
            GetImageRequest { photo_id: origin.identifier.clone(), image_id: image_id.to_string() }
        ).await.unwrap();

        Ok(photo)
    }

    async fn login(&mut self, name: &EntityName) -> anyhow::Result<String> {
        let host = self.config.hosts.get(name).unwrap();

        let cred = crate::federation::protocol::request::<LoginEndpoint>(
            None,
            &host.origin,
            LoginBody {
                username: host.username.clone(),
                password: host.password.clone(),
            }
        ).await.unwrap();

        self.session.insert(name.clone(), cred.session_key.clone());

        Ok(cred.session_key)
    }

    async fn ensure_logged_in(&mut self, name: &EntityName) -> anyhow::Result<String> {
        // TODO: Handle session expire

        if let Some(session) = self.session.get(name) {
            Ok(session.clone())
        } else {
            self.login(name).await
        }
    }
}

