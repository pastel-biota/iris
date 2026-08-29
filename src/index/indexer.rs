use crate::{
    index::{FileIndexer, IndexInterest, photo::PhotoIndexer},
    repository::io::ScopedPath,
    tag::index::TagIndexer,
};

pub struct Indexers {
    pub photo_indexer: PhotoIndexer,
    pub tag_indexer: TagIndexer,
}

impl Indexers {
    pub fn find_interested_indexer<'indexer>(
        &'indexer self,
        path: &'_ ScopedPath,
    ) -> anyhow::Result<Option<(Indexer<'indexer>, IndexInterest)>> {
        if let Some(interest) = self.photo_indexer.get_index_name(path)? {
            return Ok(Some((Indexer::PhotoIndexer(&self.photo_indexer), interest)));
        };

        if let Some(interest) = self.tag_indexer.get_index_name(path)? {
            return Ok(Some((Indexer::TagIndexer(&self.tag_indexer), interest)));
        };

        Ok(None)
    }
}

pub enum Indexer<'s> {
    PhotoIndexer(&'s PhotoIndexer),
    TagIndexer(&'s TagIndexer),
}

impl Indexer<'_> {
    pub async fn index(&self, scope: &ScopedPath) -> anyhow::Result<()> {
        match self {
            Indexer::PhotoIndexer(indexer) => indexer.index(scope).await,
            Indexer::TagIndexer(indexer) => indexer.index(scope).await,
        }
    }

    pub fn indexer_version(&self) -> &'static str {
        match self {
            Indexer::PhotoIndexer(indexer) => indexer.indexer_version(),
            Indexer::TagIndexer(indexer) => indexer.indexer_version(),
        }
    }
}
