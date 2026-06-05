//! oe1022d-chart-core — Tauri command bindings and downsampling service
//! for the live chart (Plotly frontend).
//!
//! C1 placeholder. Real implementation in C9.

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
