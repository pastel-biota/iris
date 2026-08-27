use crate::model::{Identifier, TagIdentifier};

pub struct PhotoFilter<'a> {
    pub beginning: Option<(i64, &'a Identifier)>,
    pub size: usize,
    pub tag: Option<&'a [TagIdentifier]>,
}

impl Default for PhotoFilter<'_> {
    fn default() -> Self {
        Self {
            beginning: Default::default(),
            tag: Default::default(),
            size: 50,
        }
    }
}
