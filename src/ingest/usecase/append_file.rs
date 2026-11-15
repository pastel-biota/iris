use std::{borrow::Cow, collections::HashSet, path::Path, sync::Arc};

use crate::{
    Context,
    api::error::ApiError,
    auth::{extractor::ValidUserSession, session::ValidSession, whitelist},
    ingest::technicals::stream::StreamPayloadHandle,
    model::{PhotoMeta, PhotoRelatedFile, PhotoRelatedFileRole, PhotoVisibility},
    repository::io::LengthedStream,
    services::hash::retrieve_file_hash,
};

pub struct AppendFileUsecaseParam<'s> {
    pub file: StreamPayloadHandle<'s>,
    pub file_name: Option<String>,
}

struct FileName<'s>(&'s str, &'s str, &'s str);

impl FileName<'_> {
    pub fn new(file_name: &str) -> Option<FileName> {
        let (stem, ext) = file_name.rsplit_once(".")?;
        Some(FileName(file_name, stem, ext))
    }

    pub fn full_name(&self) -> &str {
        self.0
    }

    pub fn stem(&self) -> &str {
        self.1
    }

    pub fn ext(&self) -> &str {
        self.2
    }
}

pub type AppendFileUsecaseResult = Result<AppendFileUsecaseSuccessful, AppendFileUsecaseError>;

#[derive(Debug)]
pub struct AppendFileUsecaseSuccessful {
    pub changed: bool,
    pub meta: PhotoMeta,
}

#[derive(Debug)]
pub enum AppendFileUsecaseError {
    NoFilename,
    UnknownExtension,
    NoAdequatePhoto,
    Conflict(Cow<'static, str>),
    InternalError(Cow<'static, str>),
}

pub async fn append_file<'s>(
    session: &ValidSession,
    ctx: Arc<Context>,
    param: AppendFileUsecaseParam<'s>,
) -> AppendFileUsecaseResult {
    let Some(file_name) = &param.file_name.clone() else {
        return Err(AppendFileUsecaseError::NoFilename);
    };

    let Some(file) = FileName::new(file_name.as_str()) else {
        return Err(AppendFileUsecaseError::UnknownExtension);
    };

    let Some(role) = PhotoRelatedFileRole::from_ext(file.ext()) else {
        return Err(AppendFileUsecaseError::UnknownExtension);
    };

    match role {
        PhotoRelatedFileRole::Original => {
            map_filename_with_original(session, ctx, param, file).await
        }
        PhotoRelatedFileRole::RAW => update_related_raw_file(session, ctx, param, file).await,
    }
}

async fn map_filename_with_original<'s>(
    session: &ValidSession,
    ctx: Arc<Context>,
    param: AppendFileUsecaseParam<'s>,
    file_name: FileName<'s>,
) -> AppendFileUsecaseResult {
    let sha256 = retrieve_file_hash(
        param
            .file
            .read()
            .await
            .map_err(|e| AppendFileUsecaseError::InternalError(e.to_string().into()))?
            .bytes
            .iter()
            .as_slice(),
    );

    let meta = {
        let registry = ctx.registry.read().await;
        registry
            .get_image_with_hash(session.photo_visibility(), &sha256)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to retrieve photo by hash");
                AppendFileUsecaseError::InternalError(
                    "The matching photo could not be retrieved with internal error".into(),
                )
            })?
            .ok_or(AppendFileUsecaseError::NoAdequatePhoto)?
    };

    if let Some(name) = &meta.file_name_hint {
        return if name == file_name.full_name() {
            Ok(AppendFileUsecaseSuccessful {
                changed: false,
                meta,
            })
        } else {
            Err(AppendFileUsecaseError::Conflict(
                format!(
                    "{}\n{}\n  Existing name: {}\n  New name: {}",
                    "The file name was already registered for this image file, but it's different...?",
                    "Did you rename the one?",
                    name,
                    file_name.full_name(),
                ).into()
            ))
        };
    }

    let meta = {
        let mut registry = ctx.registry.write().await;
        registry
            .rename_legacy_file(meta.id(), file_name.full_name())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update the image for renaming");
                AppendFileUsecaseError::InternalError("The photo could not be updated".into())
            })?
    };

    Ok(AppendFileUsecaseSuccessful {
        changed: true,
        meta,
    })
}

async fn update_related_raw_file<'s>(
    session: &ValidSession,
    ctx: Arc<Context>,
    param: AppendFileUsecaseParam<'s>,
    file_name: FileName<'s>,
) -> AppendFileUsecaseResult {
    let photo = {
        let registry = ctx.registry.read().await;
        registry
            .get_image_from_stem(session.photo_visibility(), file_name.stem())
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to retrieve photo by stem");
                AppendFileUsecaseError::InternalError(
                    "The matching photo could not be retrieved with internal error".into(),
                )
            })?
            .ok_or(AppendFileUsecaseError::NoAdequatePhoto)?
    };

    let file = PhotoRelatedFile {
        file_name: file_name.full_name().to_string(),
        role: HashSet::from_iter([PhotoRelatedFileRole::RAW]),
    };

    if photo.files.contains_key(file_name.full_name()) {
        return Ok(AppendFileUsecaseSuccessful {
            changed: false,
            meta: photo,
        });
    }

    let meta = {
        let mut registry = ctx.registry.write().await;
        let stream = param.file.stream_read().await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to open the stream to copy the data");
            AppendFileUsecaseError::InternalError("The received content could not be opened".into())
        })?;

        registry
            .upload_related_file(photo.id(), file, stream.stream)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to upload and update the related file");
                AppendFileUsecaseError::InternalError("The photo could not be updated".into())
            })?
    };

    Ok(AppendFileUsecaseSuccessful {
        changed: true,
        meta,
    })
}
