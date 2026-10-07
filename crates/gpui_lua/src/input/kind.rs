/// Marker trait for input kinds (single-line or multi-line).
pub trait InputModeKind: 'static + Sized {
    const MULTI_LINE: bool;
}

/// A single-line text field mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputMode;

impl InputModeKind for InputMode {
    const MULTI_LINE: bool = false;
}

/// A multi-line textarea mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextareaMode;

impl InputModeKind for TextareaMode {
    const MULTI_LINE: bool = true;
}
