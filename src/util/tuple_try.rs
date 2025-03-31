pub trait TupleTry<T, E> {
    fn tuple_try(self) -> Result<T, E>;
}

macro_rules! impl_tuple_try {
    (
        $(
            ( $($ok:ident),+ ), ( $($err:ident),+ );
        )*
    ) => {
        $(
            impl<Error, $($ok),+ , $($err),+> TupleTry<($($ok,)+), Error> for ( $( Result<$ok, $err> ,)+ )
            where
                $( Error: From<$err> ),+
            {
                #[allow(non_snake_case)]
                fn tuple_try(self) -> Result<($($ok,)+), Error> {
                    let ( $( $ok ,)+ ) = self;
                    Ok(( $( $ok ? ,)+ ))
                }
            }
        )*
    };
}

impl_tuple_try![
    (A), (AE);
    (A, B), (AE, BE);
    (A, B, C), (AE, BE, CE);
    // (A, B, C, D), (AE, BE, CE, DE);
    // (A, B, C, D, E), (AE, BE, CE, DE, EE);
    // (A, B, C, D, E, F), (AE, BE, CE, DE, EE, FE);
    // (A, B, C, D, E, F, G), (AE, BE, CE, DE, EE, FE, GE);
    // (A, B, C, D, E, F, G, H), (AE, BE, CE, DE, EE, FE, GE, HE);
    // (A, B, C, D, E, F, G, H, I), (AE, BE, CE, DE, EE, FE, GE, HE, IE);
    // (A, B, C, D, E, F, G, H, I, J), (AE, BE, CE, DE, EE, FE, GE, HE, IE, JE);
    // (A, B, C, D, E, F, G, H, I, J, K), (AE, BE, CE, DE, EE, FE, GE, HE, IE, JE, KE);
    // (A, B, C, D, E, F, G, H, I, J, K, L), (AE, BE, CE, DE, EE, FE, GE, HE, IE, JE, KE, LE);
];
