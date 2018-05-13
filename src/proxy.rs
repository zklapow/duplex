use futures::{Future, Stream};
use std::error::Error;
use std::io;
use std::mem;
use std::net::{Shutdown, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor::Core;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{copy, shutdown};
use tokio_io::io::{ReadHalf, WriteHalf};

enum SocketStatus {
    Ready(ReadHalf<TcpStream>, WriteHalf<TcpStream>),
    InUse,
}

struct Proxy {
    inner: Arc<Mutex<SocketStatus>>
}

pub fn proxy(listen_addr: &str, server_addr: &str) {
    trace!("Parsing listener address {:?}", listen_addr);
    let listen_addr = listen_addr.parse::<SocketAddr>().unwrap();

    trace!("Parsing server address {:?}", server_addr);
    let server_addr = server_addr.parse::<SocketAddr>().unwrap();

    trace!("Starting tokio event loop");
    let mut event_loop = Core::new().unwrap();
    let handle = event_loop.handle();

    let socket = TcpListener::bind(&listen_addr, &event_loop.handle()).unwrap();

    info!("Listening on: {}", listen_addr);
    info!("Proxying to: {}", server_addr);

    let server = TcpStream::connect(&server_addr, &handle);
    let done = server.and_then(move |server| {
        trace!("Server connection established!");
        let (pread, pwrite) = server.split();
        let proxy = Proxy { inner: Arc::new(Mutex::new(SocketStatus::Ready(pread, pwrite))) };

        socket.incoming().for_each(move |(client, client_addr)| {
            info!("Connection accepted from {}", client_addr);

            let lock = proxy.inner.clone();

            let mut state = lock.lock().expect("Could not lock socket state");
            let owned_socket_state = mem::replace(&mut *state, SocketStatus::InUse);

            match owned_socket_state {
                SocketStatus::Ready(read, write) => {
                    let (client_reader, client_writer) = client.split();
                    let client_to_server = copy(client_reader, write)
                        .map(|(n, _, _)| n);
                    let server_to_client = copy(read, client_writer)
                        .map(|(n, _, _)| n);

                    let f = client_to_server.select(server_to_client)
                        .map(|_| {
                            info!("Connection finished");
                        })
                        .map_err(|(err, _)| {
                            error!("Connection error {:?}", err);
                        });

                    handle.spawn(f)
                }
                SocketStatus::InUse => {
                    handle.spawn_fn(move || {
                        client.shutdown(Shutdown::Both)
                            .map(|_|())
                            .map_err(|_| ())
                        //Ok(())
                    })
                }
            }

            Ok(())
        })
    });

    event_loop.run(done).unwrap();
}
