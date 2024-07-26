use std::{
    cell::{Cell, RefCell},
    io::ErrorKind,
    os::{
        linux::net::SocketAddrExt,
        unix::net::{SocketAddr, UnixListener, UnixStream},
    },
    process::exit,
    rc::Rc,
};

use anyhow::{Context, Result};
use gtk4::{
    gio::{Socket, SocketListener},
    glib::{Bytes, Object, Priority},
    prelude::{IOStreamExt, SocketListenerExt},
    ApplicationWindow, CheckButton, Label, Picture, Revealer, SpinButton,
};

use crate::ShotType;

pub fn activate_or_open() -> Result<UnixListener> {
    let addr =
        SocketAddr::from_abstract_name(format!("shots-{}", std::env::var("XDG_SESSION_ID")?))?;
    match UnixListener::bind_addr(&addr) {
        Ok(listener) => Ok(listener),
        Err(e) => {
            if let ErrorKind::AddrInUse = e.kind() {
                UnixStream::connect_addr(&addr).context("opening activation socket")?;
                exit(0)
            } else {
                Err(e).context("creating activation socket")
            }
        }
    }
}

pub(crate) async fn wait_for_activation(
    last_shot: Rc<Cell<ShotType>>,
    window: ApplicationWindow,
    listener: UnixListener,
    image: Rc<RefCell<Option<Bytes>>>,
    image_view: Picture,
    image_revealer: Revealer,
    delay_button: SpinButton,
    cursor_check: CheckButton,
    error_revealer: Revealer,
    error_label: Label,
) {
    let sockets = SocketListener::new();
    let socket = unsafe { Socket::from_fd(listener) }.unwrap();
    sockets
        .add_socket(&socket, Option::<&Object>::None)
        .unwrap();
    loop {
        sockets
            .accept_future()
            .await
            .unwrap()
            .0
            .close_future(Priority::DEFAULT)
            .await
            .unwrap();
        match last_shot.get() {
            ShotType::Fullscreen => {
                crate::snap_full::handler_inner(
                    &image,
                    &image_view,
                    &image_revealer,
                    &delay_button,
                    &cursor_check,
                    &error_revealer,
                    &error_label,
                    &window,
                )
                .await
            }
            ShotType::Selection => {
                crate::snap_selection::handler_inner(
                    &image,
                    &image_view,
                    &image_revealer,
                    &delay_button,
                    &cursor_check,
                    &error_revealer,
                    &error_label,
                    &window,
                )
                .await
            }
        };
    }
}
