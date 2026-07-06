#[macro_export]
macro_rules! symmetrical_from_into {
    (
        #[$($attr:meta)+]
        $pub:vis struct $ident:ident (= $equiv:path) {
            $($(#[$fattr:meta])* $fpub:vis $fident:ident : $ty:ty $(|$fn_ident:ident| -> $rty:ty, $rty2:ty => $expr:expr)* ,)+
        }
    ) => {
        #[$($attr)+]
        $pub struct $ident {
            $($(#[$fattr])* $fpub $fident : $ty ,)+
        }

        impl From<$equiv> for $ident {
            fn from(equiv: $equiv) -> $ident {
                $ident {
                    $($fident : {
                        let ret = equiv . $fident;
                        $(
                            let func = |$fn_ident: $rty2| -> $rty { type Rty = $rty; $expr };
                            let ret : $rty = func(ret);
                        )*
                        ret.into()
                    },)+
                }
            }
        }

        impl Into<$equiv> for $ident {
            fn into(self) -> $equiv {
                $equiv {
                    $($fident : {
                        let ret = self . $fident;
                        $(
                            let func = |$fn_ident: $rty| -> $rty2 { type Rty = $rty2; $expr };
                            let ret : $rty2 = func(ret);
                        )*
                        ret.into()
                    },)+
                }
            }
        }
    };
}

#[macro_export]
macro_rules! map_hash_map {
    ($from:ty => $to:ty) => {
        |x| ->
                $from,
                $to
            => x.into_iter().map(|(k, v)| (k, v.into())).collect::<Rty>()
    }
}
