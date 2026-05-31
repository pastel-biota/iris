use crate::{auth::{context::AuthContext, session::Session}, model::Identifier, repository::whitelist::WhitelistRepositoryError};

#[derive(thiserror::Error, Debug)]
pub enum WhitelistError {
    #[error("The photo is not in the whitelist for the entity")]
    NotAllowed,

    #[error(transparent)]
    FileError(#[from] WhitelistRepositoryError),
}

pub fn ensure_photo_allowed(ctx: &AuthContext, session: &Session, photo: &Identifier) -> Result<(), WhitelistError> {
    let Some(session) = session.not_bypassed() else {
        return Ok(());
    };

    let state = ctx.state.lock().unwrap();
    let whitelist = state.whitelist.get_whitelist(session.entity.name())?;

    if whitelist.is_allowed(photo) {
        Ok(())
    } else {
        Err(WhitelistError::NotAllowed)
    }
}

pub struct PagedIdentifiers {
    pub ids: Vec<Identifier>,
    pub next_cursor: Option<Identifier>,
    pub total_count: u32,
}

pub fn get_allowed_photos(ctx: &AuthContext, session: &Session, size: usize, cursor: Option<Identifier>) -> Result<Option<PagedIdentifiers>, WhitelistError> {
    let Some(session) = session.not_bypassed() else {
        return Ok(None);
    };

    let state = ctx.state.lock().unwrap();
    let whitelist = state.whitelist.get_whitelist(session.entity.name())?;

    let Some(pics) = whitelist.seleted_pics() else {
        return Ok(None);
    };

    let paged_pics = if let Some(cursor) = &cursor {
        pics.iter()
            .skip_while(|id| &cursor != id)
            .skip(1)
            .take(size)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        pics.iter()
            .take(size)
            .cloned()
            .collect::<Vec<_>>()
    };

    let next_cursor = if paged_pics.len() == size {
        paged_pics.last().cloned()
    } else {
        None
    };

    Ok(Some(PagedIdentifiers { ids: paged_pics, next_cursor, total_count: pics.len() as u32 }))
}

