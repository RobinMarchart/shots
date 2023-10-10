#![allow(clippy::too_many_arguments)]

use std::{
    cell::{Cell, RefCell},
    os::unix::net::UnixListener,
    process::exit,
    rc::Rc,
};

use gtk4::{
    gdk::{prelude::DisplayExt, Display, Key, ModifierType},
    gio::Subprocess,
    glib::{self, clone, Bytes, MainContext, Propagation},
    prelude::{ApplicationExt, ApplicationExtManual},
    style_context_add_provider_for_display,
    traits::{BoxExt, ButtonExt, GtkWindowExt, WidgetExt},
    Adjustment, AlternativeTrigger, Application, ApplicationWindow, Box, Button, CallbackAction,
    CheckButton, CssProvider, KeyvalTrigger, Label, Picture, Revealer, ScrolledWindow, Shortcut,
    ShortcutController, SpinButton, STYLE_PROVIDER_PRIORITY_APPLICATION,
};

struct KillSubprocessGuard {
    process: Subprocess,
}

impl Drop for KillSubprocessGuard {
    fn drop(&mut self) {
        self.process.force_exit();
    }
}

impl KillSubprocessGuard {
    pub fn new(process: Subprocess) -> Self {
        KillSubprocessGuard { process }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShotType {
    Fullscreen,
    Selection,
}

mod activate;
mod save_to_file;
mod set_clipboard;
mod snap_full;
mod snap_selection;

fn main() -> anyhow::Result<()> {
    let listener = Cell::new(Some(activate::activate_or_open()?));

    let app = Application::builder().application_id("com.shots").build();

    app.connect_activate(move |app| build_ui(app, &listener));
    app.connect_startup(|_| {
        let css_provider = CssProvider::new();
        css_provider.load_from_string(include_str!("style.css"));
        style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to display"),
            &css_provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    exit(app.run().value())
}

fn build_ui(app: &Application, listener: &Cell<Option<UnixListener>>) {
    let listener = listener.take().unwrap();
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Shots")
        .build();
    let main_container = Box::builder()
        .css_classes(["main"])
        .orientation(gtk4::Orientation::Vertical)
        .build();
    let error_revealer = Revealer::new();
    let error_box = Box::builder()
        .css_classes(["error"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let error_label = Label::new(None);
    let error_close = Button::builder()
        .icon_name("window-close")
        .css_classes(["error"])
        .build();

    let horizontal = Box::builder()
        .css_classes(["horizontal"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();

    let settings = Box::builder()
        .css_classes(["settings"])
        .orientation(gtk4::Orientation::Vertical)
        .build();

    let delay_box = Box::builder()
        .css_classes(["setting"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let delay_label1 = Label::new(Some("Delay for"));
    let delay_label2 = Label::new(Some("seconds"));
    let delay_button = SpinButton::builder()
        .numeric(true)
        .snap_to_ticks(true)
        .update_policy(gtk4::SpinButtonUpdatePolicy::IfValid)
        .wrap(false)
        .adjustment(&Adjustment::new(0.0, 0.0, 255.0, 1.0, 10.0, 10.0))
        .build();

    let cursor_box = Box::builder()
        .css_classes(["setting"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let cursor_label = Label::new(Some("Include Cursor"));
    let cursor_check = CheckButton::new();

    let capture_box = Box::builder()
        .css_classes(["setting", "buttons"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let capture_full = Button::with_label("Full Screen");
    let capture_selection = Button::with_label("Selection");

    let save_box = Box::builder()
        .css_classes(["setting", "buttons"])
        .orientation(gtk4::Orientation::Horizontal)
        .build();
    let save_file = Button::with_label("Save to File");
    let save_clip = Button::with_label("Copy to Clipboard");

    let image_revealer = Revealer::builder()
        .css_classes(["image_revealer"])
        .hexpand(true)
        .vexpand(true)
        .build();
    let image_scroll = ScrolledWindow::builder()
        .css_classes(["image_scroll"])
        .build();
    let image_view = Picture::builder()
        .can_shrink(false)
        .css_classes(["image"])
        .build();

    let image: Rc<RefCell<Option<Bytes>>> = Rc::new(RefCell::new(None));
    let last_shot: Rc<Cell<ShotType>> = Rc::new(Cell::new(ShotType::Selection));

    let main_context = MainContext::default();

    let display = Display::default().expect("could not connect to display");
    let clipboard = display.clipboard();

    let shortcuts = ShortcutController::new();

    main_context.spawn_local(activate::wait_for_activation(
        last_shot.clone(),
        window.clone(),
        listener,
        image.clone(),
        image_view.clone(),
        image_revealer.clone(),
        delay_button.clone(),
        cursor_check.clone(),
        error_revealer.clone(),
        error_label.clone(),
    ));

    error_close.connect_clicked(clone!(@weak error_revealer => move|_|{
        error_revealer.set_reveal_child(false);
    }));
    capture_full.connect_clicked(snap_full::get_handler(
        &last_shot,
        &main_context,
        &image,
        &image_view,
        &image_revealer,
        &delay_button,
        &cursor_check,
        &error_revealer,
        &error_label,
        &window,
    ));
    capture_selection.connect_clicked(snap_selection::get_handler(
        &last_shot,
        &main_context,
        &image,
        &image_view,
        &image_revealer,
        &delay_button,
        &cursor_check,
        &error_revealer,
        &error_label,
        &window,
    ));
    save_file.connect_clicked(save_to_file::get_handler(
        &main_context,
        &window,
        &image,
        &error_revealer,
        &error_label,
    ));
    save_clip.connect_clicked(set_clipboard::get_handler(
        &image,
        &clipboard,
        &error_revealer,
        &error_label,
    ));

    shortcuts.add_shortcut(
        Shortcut::builder()
            .trigger(&AlternativeTrigger::new(
                KeyvalTrigger::new(Key::Q, ModifierType::empty()),
                KeyvalTrigger::new(Key::Escape, ModifierType::empty()),
            ))
            .action(&CallbackAction::new(
                clone!(@weak window => @default-return false, move |_,_|{
                    window.set_visible(false);
                    true
                }),
            ))
            .build(),
    );

    window.connect_close_request(
        clone!(@weak window => @default-return Propagation::Proceed, move |_| {
            window.set_visible(false);
            Propagation::Stop
        }),
    );

    window.add_controller(shortcuts);

    error_box.append(&error_label);
    error_box.append(&error_close);
    error_revealer.set_child(Some(&error_box));
    main_container.append(&error_revealer);

    delay_box.append(&delay_label1);
    delay_box.append(&delay_button);
    delay_box.append(&delay_label2);
    settings.append(&delay_box);

    cursor_box.append(&cursor_label);
    cursor_box.append(&cursor_check);
    settings.append(&cursor_box);

    capture_box.append(&capture_full);
    capture_box.append(&capture_selection);
    settings.append(&capture_box);

    save_box.append(&save_file);
    save_box.append(&save_clip);
    settings.append(&save_box);

    horizontal.append(&settings);

    image_scroll.set_child(Some(&image_view));
    image_revealer.set_child(Some(&image_scroll));
    horizontal.append(&image_revealer);

    main_container.append(&horizontal);

    window.set_child(Some(&main_container));

    window.present();
}
