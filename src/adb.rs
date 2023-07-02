use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Instant;

use log::{info, debug, error};

/// Adb errors
#[derive(Debug)]
pub enum AdbSocketError {
    ServerError(String),
    RecvError(String)
}

/// Adb client for sending commands to the adb server
pub struct Adb {
    s: TcpStream
}

#[derive(Debug)]
pub enum Response {
    Ok(String),
    Fail(String)
}

#[derive(Debug)]
pub enum DevicesErrors {
    ResponseRelated(Response),
    AdbRelated(AdbSocketError),
    Unknown(Option<String>)
}

impl Adb {
    // Constructs a new Adb instance and runs the server if it isn't running
    pub fn new() -> Result<Self, AdbSocketError> {
        info!("Adb::new(): Creating connection to the adb server");
        let socket = TcpStream::connect("127.0.0.1:5037");
        match socket {
            Ok(sock) => {
                info!("Adb::new(): Sucessfully connected to the adb server");
                return Ok(Adb { s: sock });
            }
            Err(err) => {
                error!("Adb::new(): Couldn't connect to the server. Error: {}", err);
                return Err(AdbSocketError::ServerError(err.to_string()));
            }
        }
    }
    fn _exec_cmd(&mut self, cmd: String) -> Result<Response, AdbSocketError> {
        debug!("Adb::_exec_cmd(): _exec_cmd called with command '{}'...", cmd);
        debug!("Adb::_exec_cmd(): Preparing header...");
        let hex_length = format!("{:04X}{}", cmd.len(), cmd);
        debug!("Adb::_exec_cmd(): Sending request...");

        let send_time: Instant = Instant::now();
        self.s.write_all(hex_length.as_bytes());
        debug!("Adb::_exec_cmd(): Took {}s", send_time.elapsed().as_secs());

        debug!("Adb::_exec_cmd(): Recieving response...");
        let recv_time: Instant = Instant::now();
        let mut buffer = String::new();
        let response = self.s.read_to_string(&mut buffer);
        debug!("Adb::_exec_cmd(): Took {}s", recv_time.elapsed().as_secs());
        if let Err(err) = response {
            error!("Adb::_exec_cmd(): Cannot recieve response of the adb server. Error: {}", err.to_string());
            return Err(AdbSocketError::RecvError(err.to_string()));
        } else {
            debug!("Adb::_exec_cmd(): Returning response...");
            let rp = {
                let rp1 = buffer.clone().drain(0..4).collect::<String>();
                if rp1 != "OK" {
                    Response::Ok(buffer.clone().drain(8..).collect::<String>())
                } else {
                    Response::Fail(buffer)
                }
            };
            return Ok(rp);
        };

    }

    pub fn devices(&mut self) -> Result<Option<Vec<String>>, DevicesErrors> {
        let devices = self._exec_cmd(String::from("host:devices"));
        match devices {
            Ok(Response::Ok(rp)) => {
                if rp == "0000" {
                    return Ok(None);
                } else {
                    return Ok(Some(vec![rp]));
                }
            }
            Ok(Response::Fail(rp)) => {
                
                return Err(DevicesErrors::ResponseRelated(Response::Fail(rp)));
            }
            Err(AdbSocketError::RecvError(rp)) => {
                return Err(DevicesErrors::AdbRelated(AdbSocketError::RecvError(rp)));
            }
            Err(_) => {
                return Err(DevicesErrors::Unknown(None));
            }
        }
    }
}