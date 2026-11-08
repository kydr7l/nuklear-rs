use nuklear_sys::*;
use std::os::raw::{c_char, c_int};

unsafe extern "C" fn width(_: nk_handle, _: f32, _: *const c_char, len: c_int) -> f32 {
    len as f32 * 6.0
}

#[test]
fn init_begin_end_free() {
    let font = nk_user_font {
        height: 12.0,
        width: Some(width),
        ..Default::default()
    };
    let mut ctx: Box<nk_context> = Box::default();
    unsafe {
        assert!(nk_init_default(&mut *ctx, &font));
        let shown = nk_begin(
            &mut *ctx,
            c"win".as_ptr(),
            nk_rect(0.0, 0.0, 100.0, 100.0),
            NK_WINDOW_BORDER as u32,
        );
        assert!(shown);
        nk_end(&mut *ctx);
        nk_clear(&mut *ctx);
        nk_free(&mut *ctx);
    }
}
