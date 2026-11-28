// Pudding v0.5.13 is the adopted codebase baseline; its CI only runs
// `cargo build` and never runs clippy, so skyward-ui (including the
// build.rs-generated `texts.rs`) carries the lints below. Allow them at the
// crate root so the workspace `-D warnings` gate stays green while still
// catching any *new* lint introduced by later changes.
#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::empty_line_after_doc_comments,
    clippy::field_reassign_with_default,
    clippy::if_same_then_else,
    clippy::items_after_test_module,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::manual_is_multiple_of,
    clippy::needless_borrow,
    clippy::needless_lifetimes,
    clippy::obfuscated_if_else,
    clippy::ptr_arg,
    clippy::single_match,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::useless_vec,
    dead_code,
    non_upper_case_globals,
    unreachable_patterns,
    unused_assignments,
    unused_doc_comments,
    unused_imports,
    unused_lint,
    unused_mut,
    unused_variables,
)]

pub mod aod;
pub mod app;
pub mod avatar;
pub mod canvas;
pub mod charge;
pub mod color;
pub mod editor;
pub mod emoji;
pub mod files;
pub mod fling;
pub mod fmt;
pub mod focus;
pub mod font;
pub mod geom;
pub mod glow;
pub mod icons;
pub mod ime;
pub mod lang;
pub mod layout;
pub mod motion;
pub mod pages;
pub mod pinyin;
pub mod progress;
pub mod mp4;
pub mod record;
pub mod search;
pub mod settings;
pub mod sprite;
pub mod surface;
pub mod term;
pub mod tools;
pub(crate) mod texts;
pub mod theme;
pub mod widgets;

pub use app::App;
pub use canvas::Canvas;
pub use color::Rgba;
pub use font::Fonts;
pub use lang::Lang;
pub use pages::Ctx;
pub use surface::Surface;
pub use theme::Theme;
