#![cfg(target_os = "macos")]
//! Shared Apple platform support for GPUI.
//!
//! This crate contains the `new_window_layer` helper shared
//! by GPUI's Apple platform backends.

pub mod metal_renderer;
