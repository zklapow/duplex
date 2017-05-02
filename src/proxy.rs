use std::io;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::net::{Shutdown, SocketAddr};

use tokio_core::reactor::Core;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::io::{copy, shutdown};
use tokio_io::io::{ReadHalf, WriteHalf};
use futures::{Stream, Future};

struct Proxy {
    inner: Arc<Mutex<(Option<ReadHalf<TcpStream>>, Option<WriteHalf<TcpStream>>)>>
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
        let proxy = Proxy{ inner: Arc::new(Mutex::new((Option::Some(pread), Option::Some(pwrite)))) };
        //let (server_reader, server_writer) = server.split();

        socket.incoming().for_each(move |(client, client_addr)| {
            trace!("Connection accepted from {}", client_addr);

            let (client_reader, client_writer) = client.split();
            let lock = proxy.inner.clone();

            let res = lock.try_lock().map(|mut gaurd| {
                let (ref mut boxed_reader, ref mut boxed_writer) = *gaurd;
                let client_to_server = copy(client_reader, boxed_writer.take().unwrap()).map(|(n, _, _)| n);
                let server_to_client = copy(boxed_reader.take().unwrap(), client_writer).map(|(n, _, _)| n);

                let res = client_to_server.join(server_to_client);

                res.map(move |(from_client, from_server)| {
                    debug!("Connection closes got {} bytes from client and {} bytes from server", from_client, from_server);
                }).map_err(|e| {
                    error!("Got error handling connection: {}", e);
                })
            });

            match res {
                Ok(f) => {
                    handle.spawn(f);
                    Ok(())
                },
                Err(e) => Err(io::Error::new(io::ErrorKind::WouldBlock, e.description()))
            }
        })
    });

    event_loop.run(done).unwrap();
}
