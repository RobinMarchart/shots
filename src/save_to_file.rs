use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use gtk4::{
    gio::{FileCreateFlags, ListStore},
    glib::{self, clone, Bytes, MainContext, Priority},
    prelude::{FileExt, OutputStreamExtManual},
    ApplicationWindow, Button, FileDialog, FileFilter, Label, Revealer,
};

use anyhow::{anyhow, Context, Result};

async fn save_to_file(window: ApplicationWindow, file: Rc<RefCell<Option<Bytes>>>) -> Result<()> {
    let file = Ref::filter_map(file.borrow(), Option::as_ref)
        .map_err(|_| anyhow!("No screenshot available to save"))?
        .clone();
    let filter = FileFilter::new();
    filter.add_suffix("png");
    let filters = ListStore::new::<FileFilter>();
    filters.append(&filter);
    FileDialog::builder()
        .default_filter(&filter)
        .filters(&filters)
        .build()
        .save_future(Some(&window))
        .await
        .context("choosing output file")?
        .create_future(FileCreateFlags::REPLACE_DESTINATION, Priority::DEFAULT)
        .await
        .context("creating output file")?
        .write_all_future(file, Priority::DEFAULT)
        .await
        .map_err(|(_, e)| e)
        .context("writing image to file")?;
    Ok(())
}
fn handler(
    main_context: &MainContext,
    window: &ApplicationWindow,
    file: &Rc<RefCell<Option<Bytes>>>,
    error_revealer: &Revealer,
    error_label: &Label,
) {
    main_context.spawn_local(clone!(@strong file, @weak window, @strong error_revealer, @strong error_label => async move{
                if let Err(e) = save_to_file(window, file).await{
                    error_label.set_text(&format!("{e:?}"));
                    error_revealer.set_reveal_child(true);
                }
            }));
}

pub fn get_handler(
    main_context: &MainContext,
    window: &ApplicationWindow,
    file: &Rc<RefCell<Option<Bytes>>>,
    error_revealer: &Revealer,
    error_label: &Label,
) -> impl Fn(&Button) {
    clone!(@strong file, @weak window, @strong error_revealer, @strong error_label, @strong main_context => move |_| handler(&main_context, &window, &file, &error_revealer, &error_label))
}
