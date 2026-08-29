pub trait TryCollectN<I> {
    fn try_collect_n<const N: usize>(self) -> Option<[I; N]>;
}

impl<T: Iterator<Item = I>, I> TryCollectN<I> for T {
    fn try_collect_n<const N: usize>(mut self) -> Option<[I; N]> {
        let mut array = [const { Option::<I>::None }; N];

        for elem in array.iter_mut() {
            *elem = Some(self.next()?);
        }

        if self.next().is_some() {
            return None;
        }

        Some(array.map(|value| value.unwrap()))
    }
}
