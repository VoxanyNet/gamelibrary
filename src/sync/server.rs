use std::net::{SocketAddr, TcpListener, TcpStream};

use diff::Diff;
use lz4_flex::{compress_prepend_size, decompress_size_prepended};
use serde::{de::DeserializeOwned, Serialize};
use tungstenite::{Message, WebSocket};

pub struct SyncServer<T: Serialize + DeserializeOwned + Diff + Clone + PartialEq> {
    state: T,
    clients: Vec<WebSocket<TcpStream>>,
    listener: TcpListener

}

impl<T> SyncServer<T>
where 
    T: Serialize + DeserializeOwned + Diff + Clone + PartialEq,
    <T as Diff>::Repr: DeserializeOwned + Serialize {
    
    pub fn new(address: SocketAddr, initial_state: T) -> Self {

        let listener = match TcpListener::bind(address) {
            Ok(listener) => listener,
            Err(error) => panic!("failed to bind listener: {}", error),
        };

        match listener.set_nonblocking(true) {
            Ok(_) => {},
            Err(error) => panic!("failed to set server as non blocking: {}", error),
        };

        Self {
            state: initial_state, 
            clients: vec![], 
            listener
        }
    }

    pub fn receive_updates(&mut self) {

        let mut client_index = 0;

        'client_loop: while client_index < self.clients.len() {

            // take the client out, receive all updates, then put it back in
            let mut client = self.clients.remove(client_index);
            
            // keep trying to receive updates until there are none
            loop {

                let compressed_state_diff_bytes = match client.read() {
                    Ok(message) => {
                        match message {
                            Message::Binary(compressed_state_diff_bytes) => {
                                compressed_state_diff_bytes
                            },
                            _ => todo!("client tried to send non binary message")
                        }
                    },
                    Err(error) => {
                        match error {

                            tungstenite::Error::Io(io_error) => {
                                match io_error.kind() {
                                    std::io::ErrorKind::WouldBlock => {
                                        // this means that there was no update to read
                                        self.clients.insert(client_index, client);
                                        
                                        client_index += 1;
                                        
                                        continue 'client_loop // move to the next client
                                    },
                                    std::io::ErrorKind::ConnectionReset => {
                                        println!("client {} disconnected", client_index);

                                        // do not increment client index because we arent putting this one back

                                        continue 'client_loop;
                                    }
                                    _ => todo!("unhandled io error: {}", io_error),
                                }
                            },
                            _ => todo!("unhandled websocket message read error: {}", error)
                        }
                    },
                };
                let state_diff_bytes = decompress_size_prepended(&compressed_state_diff_bytes).expect("Failed to decompress game state diff string bytes");
    
                let state_diff: <T as Diff>::Repr = match bitcode::deserialize(&state_diff_bytes) {
                    Ok(state_diff) => state_diff,
                    Err(error) => {
                        todo!("unhandled game state diff deserialization error: {}", error);
                    },
                };
    
                // relay this update to other clients
                'relay: for other_client_index in 0..self.clients.len() {
    
                    let mut other_client = self.clients.remove(other_client_index);
    
                    match other_client.send(Message::Binary(compressed_state_diff_bytes.clone())) {
                        Ok(_) => {
                            self.clients.insert(other_client_index, other_client);

                            continue 'relay;

                        },
                        Err(error) => {
                            todo!("unhandled error when relaying update data to client: {}", error);
    
                        },
                    }
    
                }

                // apply it to our own game state
                self.state.apply(&state_diff);
            }
        }
    }

    pub fn accept_new_client(&mut self) -> Option<()> {
        match self.listener.accept() {
            Ok((stream, address)) => {
                println!("received new connection from address: {}", address);

                stream.set_nonblocking(true).expect("Failed to set new client as non blocking");

                let mut websocket_stream = loop {
                    match tungstenite::accept(stream.try_clone().expect("failed to clone stream")) {
                        Ok(websocket_stream) => break websocket_stream,
                        Err(error) => {
                            match error {
                                tungstenite::HandshakeError::Interrupted(_) => continue, // try again if the handshake isnt done yet
                                tungstenite::HandshakeError::Failure(error) => panic!("handshake failed with new client: {}", error),
                            }
                        },
                    };
                };
                

                // send client current state
                let state_bytes = bitcode::serialize(&self.state).expect("Failed to serialize current game state");

                let compressed_state_bytes = compress_prepend_size(&state_bytes);

                // keep attempting to send initial state to client
                loop {
                    match websocket_stream.send(
                        Message::Binary(compressed_state_bytes.clone())
                    ) {
                        Ok(_) => break,
                        Err(error) => {
                            match error {
                                tungstenite::Error::Io(io_error) => {
                                    match io_error.kind() {
                                        std::io::ErrorKind::WouldBlock => {
                                            continue; // try again if the socket blocked
                                        },
                                        _ => panic!("Something went wrong trying to send initial state: {}", io_error)
                                    }
                                },
                                _ => panic!("Something went wrong trying to send initial state: {}", error)
                            }
                        },
                    }
                }

                println!("pushing new client");

                self.clients.push(websocket_stream);

                return Some(())

            },
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::WouldBlock => return None, // no new clients

                    _ => {
                        println!("Something went wrong trying to accept a new client");
                        return None
                    }
                }
            },
        }
    }
}