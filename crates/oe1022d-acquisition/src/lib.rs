//! oe1022d-acquisition — 4-thread acquisition core (acquisition / parser / writer / downsample).
//!
//! C1 placeholder. Real implementation in C5-C7.

#![deny(unsafe_code)]
#![warn(missing_debug_implementations)]

pub const SCAFFOLD_VERSION: &str = "0.1.0-c1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_compiles() {
        assert_eq!(SCAFFOLD_VERSION, "0.1.0-c1");
    }
}
