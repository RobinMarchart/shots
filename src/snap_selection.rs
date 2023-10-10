use std::{
    cell::{Cell, RefCell},
    ffi::OsStr,
    rc::Rc,
    str::from_utf8,
};

use gtk4::{
    gdk::Texture,
    gio::{Subprocess, SubprocessFlags},
    glib::{self, clone, timeout_future_seconds, Bytes, MainContext},
    prelude::TextureExt,
    traits::{CheckButtonExt, WidgetExt},
    ApplicationWindow, Button, CheckButton, Label, Picture, Revealer, SpinButton,
};

use anyhow::{anyhow, Context, Result};

use crate::{KillSubprocessGuard, ShotType};

async fn snap_selection(cursor: bool, wait_seconds: u32) -> Result<Bytes> {
    timeout_future_seconds(wait_seconds).await;

    let _hyprpicker = KillSubprocessGuard::new(
        Subprocess::newv(
            &[OsStr::new("hyprpicker"), OsStr::new("-r"), OsStr::new("-z")],
            SubprocessFlags::NONE,
        )
        .context("spawning hyprpicker")?,
    );

    let slurp = Subprocess::newv(
        &[OsStr::new("slurp")],
        SubprocessFlags::STDOUT_PIPE | SubprocessFlags::STDERR_PIPE,
    )
    .context("spawning slurp")?;
    let (out, err) = slurp
        .communicate_utf8_future(None)
        .await
        .context("receiving output from grim")?;
    let selection = if slurp.is_successful() {
        Ok(out.expect("stdout output"))
    } else {
        let err = err.expect("stderr output");
        if err.is_empty() {
            Err(anyhow!(
                "slurp failed with exit status {} but no error output was provided",
                slurp.exit_status()
            ))
        } else {
            Err(anyhow!(
                "grim failed with exit status {}:\n{}",
                slurp.exit_status(),
                err
            ))
        }
    }?;
    let selection = selection.trim_end();
    let grim_with_c = [
        OsStr::new("grim"),
        OsStr::new("-g"),
        OsStr::new(&selection),
        OsStr::new("-c"),
        OsStr::new("-"),
    ];
    let grim_without_c = [
        OsStr::new("grim"),
        OsStr::new("-g"),
        OsStr::new(&selection),
        OsStr::new("-"),
    ];
    let grim = Subprocess::newv(
        if cursor {
            &grim_with_c
        } else {
            &grim_without_c
        },
        SubprocessFlags::STDOUT_PIPE | SubprocessFlags::STDERR_PIPE,
    )
    .context("spawning grim")?;
    let (out, err) = grim
        .communicate_future(None)
        .await
        .context("receiving output from grim")?;

    if grim.is_successful() {
        Ok(out.expect("stdout output"))
    } else {
        let err = err.expect("stderr output");
        if err.is_empty() {
            Err(anyhow!(
                "grim failed with exit status {} but no error output was provided",
                grim.exit_status()
            ))
        } else {
            Err(anyhow!(
                "grim failed with exit status {}:\n{}",
                grim.exit_status(),
                from_utf8(&err).context("decoding grom stderr output")?
            ))
        }
    }
}

pub(crate) async fn handler_inner(
    image: &Rc<RefCell<Option<Bytes>>>,
    image_view: &Picture,
    image_revealer: &Revealer,
    delay_button: &SpinButton,
    cursor_check: &CheckButton,
    error_revealer: &Revealer,
    error_label: &Label,
    window: &ApplicationWindow,
) {
    match snap_selection(cursor_check.is_active(), delay_button.value() as u32)
        .await
        .and_then(move |bytes| {
            let texture = Texture::from_bytes(&bytes).context("loading screenshot image")?;
            image.replace(Some(bytes));
            Ok(texture)
        }) {
        Ok(texture) => {
            image_view.set_paintable(Some(&texture));
            image_view.set_width_request(texture.width());
            image_view.set_height_request(texture.height());
            image_revealer.set_reveal_child(true);
            error_revealer.set_reveal_child(false);
            window.set_visible(true);
        }
        Err(e) => {
            error_label.set_text(&format!("{:?}", e));
            error_revealer.set_reveal_child(true);
            window.set_visible(true);
        }
    }
}

fn handler(
    last_shot: &Rc<Cell<ShotType>>,
    main_context: &MainContext,
    image: &Rc<RefCell<Option<Bytes>>>,
    image_view: &Picture,
    image_revealer: &Revealer,
    delay_button: &SpinButton,
    cursor_check: &CheckButton,
    error_revealer: &Revealer,
    error_label: &Label,
    window: &ApplicationWindow,
) {
    window.set_visible(false);
    last_shot.set(ShotType::Selection);
    main_context.spawn_local(clone!(
        @strong main_context,
        @strong image,
        @strong image_view,
        @strong image_revealer,
        @strong delay_button,
        @strong cursor_check,
        @strong error_revealer,
        @strong error_label,
        @weak window
            => async move{
                handler_inner(&image, &image_view, &image_revealer, &delay_button, &cursor_check, &error_revealer, &error_label, &window).await

    }));
}
pub(crate) fn get_handler(
    last_shot: &Rc<Cell<ShotType>>,
    main_context: &MainContext,
    image: &Rc<RefCell<Option<Bytes>>>,
    image_view: &Picture,
    image_revealer: &Revealer,
    delay_button: &SpinButton,
    cursor_check: &CheckButton,
    error_revealer: &Revealer,
    error_label: &Label,
    window: &ApplicationWindow,
) -> impl Fn(&Button) {
    clone!(
            @strong last_shot,
            @strong main_context,
            @strong image,
            @strong image_view,
            @strong image_revealer,
            @strong delay_button,
            @strong cursor_check,
            @strong error_revealer,
            @strong error_label,
            @weak window
                => move |_|{
    handler(&last_shot,&main_context, &image, &image_view, &image_revealer,& delay_button, &cursor_check,& error_revealer,& error_label,& window)
                        })
}
