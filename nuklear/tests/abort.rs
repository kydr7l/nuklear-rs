//! Tests that expect the process to abort. Each test runs itself again in a child process.
mod common;
use nuklear::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::Command;

/// The child process gets the name of its test in this variable.
const CHILD: &str = "NUKLEAR_RS_ABORT_CHILD";
const WIN: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 300.0,
    h: 300.0,
};

/// Run the test `name` of this binary in a child process. If this process is that child, run `child` and return `None`.
/// Otherwise return the exit status and the stderr text of the child.
fn in_child(name: &str, child: impl FnOnce()) -> Option<(std::process::ExitStatus, String)> {
    if std::env::var(CHILD).as_deref() == Ok(name) {
        child();
        return None;
    }
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args([name, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .output()
        .unwrap();
    Some((
        out.status,
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[test]
fn caught_panic_in_group_scrolled_aborts() {
    let Some((status, err)) = in_child("caught_panic_in_group_scrolled_aborts", || {
        let font = UserFont::new(common::FixedFont);
        let mut ctx = Context::new(&font).unwrap();
        ctx.window("w", WIN, PanelFlags::BORDER, |w| {
            w.layout_row_dynamic(100.0, 1);
            let _ = catch_unwind(AssertUnwindSafe(|| {
                let mut scroll = Box::new((0u32, 0u32));
                w.group_scrolled(&mut scroll, "g", PanelFlags::empty(), |_| panic!("boom"));
            }));
            eprintln!("the window body continued");
        });
        eprintln!("the frame ended");
    }) else {
        return;
    };
    assert!(!status.success(), "child status {status}, stderr: {err}");
    assert!(err.contains("boom"), "stderr: {err}");
    assert!(!err.contains("the window body continued"), "stderr: {err}");
}

#[test]
fn same_window_name_twice_fails_the_nuklear_assertion() {
    let Some((status, err)) =
        in_child("same_window_name_twice_fails_the_nuklear_assertion", || {
            let font = UserFont::new(common::FixedFont);
            let mut ctx = Context::new(&font).unwrap();
            ctx.window("w", WIN, PanelFlags::BORDER, |_| ());
            ctx.window("w", WIN, PanelFlags::BORDER, |_| ());
            eprintln!("the second window returned");
        })
    else {
        return;
    };
    assert!(!status.success(), "child status {status}, stderr: {err}");
    assert!(err.contains("nuklear assertion failed"), "stderr: {err}");
    assert!(!err.contains("the second window returned"), "stderr: {err}");
}

// A guard made during an unwind must not abort when its body returns normally.
#[test]
fn scope_inside_a_drop_during_unwind_does_not_abort() {
    struct Late<'a, 'f>(&'a mut Context<'f>);
    impl Drop for Late<'_, '_> {
        fn drop(&mut self) {
            assert_eq!(
                self.0.window("late", WIN, PanelFlags::BORDER, |_| 7),
                Some(7)
            );
        }
    }
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let r = catch_unwind(AssertUnwindSafe(|| {
        let _late = Late(&mut ctx);
        panic!("outside of every scope");
    }));
    assert!(r.is_err());
    ctx.clear();
}
