use std::net::{Shutdown, SocketAddr};

use tokio_core::reactor::Core;
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::AsyncRead;
use tokio_io::io::{copy, shutdown};
use futures::{Stream, Future};

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

    let done = socket.incoming().for_each(move |(client, client_addr)| {
        trace!("Connection accepted from {}", client_addr);
        let server = TcpStream::connect(&server_addr, &handle);

        let res = server.and_then(move |server| {
            trace!("Server connection established!");
            let (client_reader, client_writer) = client.split();
            let (server_reader, server_writer) = server.split();

            let client_to_server = copy(client_reader, server_writer).map(|(n, _, _)| n);
            let server_to_client = copy(server_reader, client_writer).map(|(n, _, _)| n);;

            client_to_server.join(server_to_client)
        });

        let out = res.map(move |(from_client, from_server)| {
            debug!("Connection closes got {} bytes from client and {} bytes from server", from_client, from_server);
        }).map_err(|e| {
            error!("Got error handling connection: {}", e);
        });

        handle.spawn(out);

        Ok(())
    });

    event_loop.run(done).unwrap();
}