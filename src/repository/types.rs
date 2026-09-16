use crate::model::{Identifier, TagIdentifier};

pub struct PhotoFilter<'a> {
    pub beginning: Option<(i64, &'a Identifier)>,
    pub size: usize,
    pub tag: Option<&'a [TagIdentifier]>,
    pub order: SortOrder,
}

impl Default for PhotoFilter<'_> {
    fn default() -> Self {
        Self {
            beginning: Default::default(),
            tag: Default::default(),
            order: Default::default(),
            size: 50,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum SortOrder {
    #[default]
    ShotTime,
    Random,
}
