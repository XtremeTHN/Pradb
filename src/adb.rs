use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Instant;
use thiserror::Error;

use log::{info, debug, error as err};

/// Adb errors
#[derive(Debug, Error)]
pub enum AdbSocketError {
    #[error("Server not running")]
    ServerError(String),
    #[error("Cannot retrieve the server response")]
    RecvError(String),
    #[error("Socket writing error")]
    WriteError(#[from] io::Error)
}

/// Adb client for sending commands to the adb server
pub struct Adb {
    s: TcpStream
}

// Enum representing the possible results returned by the server
#[derive(Debug)]
pub enum Response {
    Ok(String),
    Fail(String),
    Unknown(String)
}

// General errors
#[derive(Debug, Error)]
pub enum Errors {
    #[error("Response of OKAY expected")]
    ResponseRelated(Response),
    #[error("Adb socket related")]
    AdbRelated(#[from] AdbSocketError),
    #[error("IO Error")]
    IOError(#[from] io::Error),
    #[error("Unkown error")]
    Unknown(Option<String>)
}

// Future device class
struct Device {
    adb: Adb,
}

impl Device {
}

impl Adb {
    // Constructs a new Adb instance and runs the server if it isn't running
    pub fn new() -> Result<Self, AdbSocketError> {
        env_logger::init();
        info!("Adb::new(): Creating connection to the adb server");
        let socket = TcpStream::connect("127.0.0.1:5037");
        match socket {
            Ok(sock) => {
                info!("Adb::new(): Sucessfully connected to the adb server");
                return Ok(Adb { s: sock });
            }
            Err(err) => {
                err!("Adb::new(): Couldn't connect to the server. Error: {}", err);
                err!("Adb::new(): Trying to run server...");
                return Err(AdbSocketError::ServerError(err.to_string()));
            }
        }
    }

    // Private method; Sends the gived command to the server
    fn _exec_cmd(&mut self, cmd: &str) -> Result<Response, AdbSocketError> {
        debug!("[Adb::_exec_cmd()]: _exec_cmd called with command '{}'...", cmd);
        debug!("[Adb::_exec_cmd()]: Preparing header...");
        let hex_length = format!("{:04X}{}", cmd.len(), cmd);
        debug!("[Adb::_exec_cmd()]: Sending request...");

        let send_time: Instant = Instant::now();
        self.s.write_all(hex_length.as_bytes())?;
        debug!("[Adb::_exec_cmd()]: Took {}s", send_time.elapsed().as_secs_f32());

        debug!("[Adb::_exec_cmd()]: Recieving response...");
        let recv_time: Instant = Instant::now();
        let mut buffer = [0; 4];
        self.s.read_exact(&mut buffer)?;
        debug!("[Adb::_exec_cmd()]: Took {}s", recv_time.elapsed().as_secs_f32());
        
        match String::from_utf8(buffer.to_vec()) {
            Ok(rp) => {
                    let mut string_buff = String::new();
                    
                    if let Err(err) = self.s.read_to_string(&mut string_buff) {
                        err!("[Adb::_exec_cmd()]: Error while trying to read the server response. Error: {err}");
                    }
                    
                    debug!("{string_buff}");
                    debug!("[Adb::_exec_cmd()] Returning the string recievied...");
                    if &rp == "OKAY" {
                        debug!("Returning ok");
                        return Ok(Response::Ok(string_buff));
                    } else if &rp == "FAIL" {
                        return Ok(Response::Fail(string_buff.drain(4..).collect()));
                    } else {
                        return Ok(Response::Unknown(string_buff));
                    }
            }
            Err(err) => {
                return Err(AdbSocketError::RecvError(err.to_string()));
            }
        }

        // debug!("{:?}", buffer);

        // if let Err(err) = response {
        //     err!("[Adb::_exec_cmd()]: Cannot recieve response of the adb server. Error: {}", err.to_string());
        //     return Err(AdbSocketError::RecvError(err.to_string()));
        // } else {
        //     debug!("[Adb::_exec_cmd()]: Returning response...");
        //     let rp = {
        //         let rp1 = buffer.clone().drain(0..4).collect::<String>();
        //         if rp1 != "OK" {
        //             Response::Ok(buffer.clone().drain(8..).collect::<String>())
        //         } else {
        //             Response::Fail(buffer)
        //         }
        //     };
        //     return Ok(rp);
        // };

    }

    // Return a list of devices
    pub fn devices(&mut self) -> Result<Option<Vec<String>>, Errors> {
        let devices = self._exec_cmd("host:devices");
        match devices {
            Ok(Response::Ok(rp)) => {
                if rp == "0000" {
                    return Ok(None);
                } else {
                    let mut splited = String::from(rp.trim());
                    let yes: String = splited.drain(4..).collect();
                    return Ok(Some(vec![String::from(yes)]));
                }
            }
            Ok(Response::Fail(rp)) => {
                
                return Err(Errors::ResponseRelated(Response::Fail(rp)));
            }
            Ok(Response::Unknown(ro)) => {
                return Err(Errors::Unknown(Some(ro)));
            }
            Err(AdbSocketError::RecvError(rp)) => {
                return Err(Errors::AdbRelated(AdbSocketError::RecvError(rp)));
            }
            Err(_) => {
                return Err(Errors::Unknown(None));
            }
        }
    }

    // Connects to a usb device
    pub fn connect_local(&mut self, serial_no: String) -> Result<Response, Errors> {
        Ok(self._exec_cmd(&format!("host:transport:{}", serial_no))?)
    }

    // Shutdown both channels of the socket
    pub fn close(&mut self) -> Result<(), Errors> {
        Ok(self.s.shutdown(std::net::Shutdown::Both)?)
    }

    pub fn get_serial_no(&mut self) -> Result<Response, Errors> {
        return Ok(self._exec_cmd("host:get-serialn")?);
    }

    // Returns version
    pub fn version(&mut self) -> Result<Response, Errors> {
        Ok(self._exec_cmd("host:version")?)
    }
}