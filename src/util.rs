pub fn collect_n<const N: usize, I>(mut iter: impl Iterator<Item = I>) -> Option<[I; N]> {
    let mut array = [const { Option::<I>::None }; N];

    for elem in array.iter_mut() {
        *elem = Some(iter.next()?);
    }

    if iter.next().is_some() {
        return None;
    }

    Some(array.map(|value| value.unwrap()))
}

