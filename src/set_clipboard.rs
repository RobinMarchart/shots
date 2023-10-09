use std::{
    cell::{Ref, RefCell},
    ops::Deref,
    rc::Rc,
};

use anyhow::{anyhow, Context, Result};
use gtk4::{
    gdk::{Clipboard, ContentProvider},
    glib::{Bytes, clone},
    Label, Revealer, Button,
};

fn set_cliboard(image: &Rc<RefCell<Option<Bytes>>>, clipboard: &Clipboard) -> Result<()> {
    clipboard
        .set_content(Some(&ContentProvider::for_bytes(
            "image/png",
            Ref::filter_map(image.borrow(), Option::as_ref)
                .map_err(|_| anyhow!("No screenshot available to save"))?
                .deref(),
        )))
        .context("Saving Image to Clipboard")
}

fn handler(
    image: &Rc<RefCell<Option<Bytes>>>,
    clipboard: &Clipboard,
    error_revealer: &Revealer,
    error_label: &Label,
) {
    if let Err(e) = set_cliboard(image, clipboard) {
        error_label.set_text(&format!("{e:?}"));
        error_revealer.set_reveal_child(true);
    }
}

pub fn get_handler(
    image: &Rc<RefCell<Option<Bytes>>>,
    clipboard: &Clipboard,
    error_revealer: &Revealer,
    error_label: &Label,
) -> impl Fn(&Button){
    clone!(
        @strong image,
        @strong clipboard,
        @strong error_revealer,
        @strong error_label
            => move |_| handler(&image,&clipboard,&error_revealer, &error_label)
    )
}
