use metal::MetalLayer;
use objc2_quartz_core::{CAAutoresizingMask, CAMetalLayer as Objc2CAMetalLayer};

/// Creates the CAMetalLayer that backs a window's view. The renderer drawing
/// into it sets its device and pixel format.
pub fn new_window_layer(transparent: bool) -> MetalLayer {
    let layer = MetalLayer::new();
    // Support direct-to-display rendering if the window is not transparent
    // https://developer.apple.com/documentation/metal/managing-your-game-window-for-metal-in-macos
    layer.set_opaque(!transparent);
    // `metal::MetalLayer` is a CAMetalLayer retained by the Metal crate.
    // Reborrow its Objective-C object as the generated objc2 class to keep
    // selector encodings and the autoresizing mask type checked here.
    let layer_object = unsafe { &*(layer.as_ptr() as *const Objc2CAMetalLayer) };
    layer_object.setAllowsNextDrawableTimeout(false);
    layer_object.setNeedsDisplayOnBoundsChange(true);
    layer_object.setAutoresizingMask(
        CAAutoresizingMask::LayerWidthSizable | CAAutoresizingMask::LayerHeightSizable,
    );
    layer
}
