macro_rules! tests {
    ($(
        $ident:ident {
            $($tt:tt)*
        }
    )*) => {
        $(
            #[test]
            fn $ident() {
                let entry: fsmentry_core::FsmEntry = syn::parse_quote! {
                    $($tt)*
                };
                let mut pretty = prettyplease::unparse(&syn::parse_quote! {
                    #entry
                });
                pretty.insert_str(0, "#![cfg_attr(rustfmt, rustfmt_skip)]\n");
                expect_test::expect_file![
                    concat!("check_compile/", stringify!($ident), ".rs")
                ].assert_eq(&pretty);
            }
        )*

        #[allow(unused)]
        mod check_compile {
            $(
                mod $ident;
            )*
        }
    };
}

tests! {
    full {
        /// This is a state machine that explores all vertex types
        #[derive(Debug)]
        #[fsmentry(
            entry = pub(crate) MyEntry,
            unsafe(true),
        )]
        pub enum State<'a, T>
        where
            T: Ord
        {
            /// An isolated vertex with data.
            PopulatedIsland(String),
            /// An isolated vertex without data.
            DesertIsland,

            /// A source vertex with data.
            Fountain(&'a mut T)
                /// I've overridden transition method name
                -fountain2bridge->
                /// A non-terminal vertex with data
                BeautifulBridge(Vec<u8>)
                -bridge2tombstone->
                /// A sink vertex with data
                Tombstone(char),

            Fountain -> Plank -> UnmarkedGrave,

            Stream -> BeautifulBridge,
            Stream -> Plank,
        }
    }
    simple {
        enum Road {
            Start -> Fork -> End,
            Fork -> Start,
        }
    }
}
