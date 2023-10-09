use std::{
    cell::RefCell,
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
    traits::WidgetExt,
    ApplicationWindow, CheckButton, Label, Picture, Revealer, SpinButton,
};

const SOCKET_NAME: &str = "shots";

pub fn activate_or_open() -> Result<UnixListener> {
    let addr = SocketAddr::from_abstract_name(SOCKET_NAME)?;
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

pub async fn wait_for_activation(
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
        .await;
        window.set_visible(true);
    }
}
