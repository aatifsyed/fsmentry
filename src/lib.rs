#[doc(inline)]
pub use fsmentry_core::FSMGenerator;

#[cfg(feature = "macros")]
#[doc(inline)]
pub use fsmentry_macros::{dot, dsl};

#[cfg(test)]
mod tests {
    #[test]
    fn trybuild() {
        let t = trybuild::TestCases::new();
        t.pass("trybuild/pass/**/*.rs");
        t.compile_fail("trybuild/fail/**/*.rs")
    }
}
