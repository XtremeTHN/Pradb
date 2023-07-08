use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::net::TcpStream;
use std::time::Instant;
use thiserror::Error;
use regex::Regex;

use log::{debug, error as err, info};

/// Adb errors
#[derive(Debug, Error)]
pub enum AdbSocketError {
    #[error("Server not running")]
    ServerError(String),
    #[error("Invalid string recieved from server")]
    InvalidString(#[from] std::string::FromUtf8Error),
    #[error("Invalid hexadecimal value recieved from server")]
    InvalidHex(#[from] std::num::ParseIntError),
    #[error("Socket writing error")]
    IOError(#[from] io::Error),
}

#[derive(Debug, Error)]
pub enum AdbDeviceError {
    #[error("The specified serial number cannot be found in the connected devices")]
    DeviceNotFound(String),
    #[error("Placeholder")]
    GeneralErrors(#[from] PradbErrors),
}

// Enum representing the possible results returned by the server
#[derive(Debug)]
pub enum Response {
    Ok(String),
    Fail(String),
    Unknown(String),
}

/// General errors
#[derive(Debug, Error)]
pub enum PradbErrors {
    #[error("Response of OKAY expected")]
    ResponseRelated(Response),
    #[error("Adb socket related")]
    AdbRelated(#[from] AdbSocketError),
    #[error("IO Error")]
    IOError(#[from] io::Error),
    #[error("Unkown error")]
    Unknown(Option<String>),
}

/// Adb client for sending commands to the adb server
#[derive(Debug)]
pub struct Adb {
    s: TcpStream,
}

#[derive(Debug, Error)]
pub enum PropertiesErrors {
    #[error("the adb socket caused an error")]
    ServerError(#[from] AdbSocketError),
    #[error("the server returned FAIL or unkown")]
    ResponseError(String),
}

/// Future device class
#[derive(Debug)]
pub struct Device {
    adb: Adb,
    serial_no: String,
    model: String,
}

impl Device {
    pub fn new(sn: String, model: String) -> Result<Self, PradbErrors> {
        Ok(Device {
            adb: Adb::new()?,
            serial_no: sn,
            model: model,
        })
    }

    pub fn use_device(&mut self) -> Result<Response, AdbSocketError> {
        self.adb._exec_cmd(&format!("host:transport:{}", self.serial_no), false, false)
    }

    pub fn get_properties(&mut self) -> Result<HashMap<String, String>, PropertiesErrors> {
        let unformatted_props = self.adb._exec_cmd(&format!("shell:getprop"), true, false)?;
        match unformatted_props {
            Response::Ok(props) => {
                // let properties: HashMap<String, String> = props
                // .lines()
                // .filter_map(|line| {
                //     let mut parts = line.splitn(2, '=');
                //     Some((parts.next()?.to_owned(), parts.next()?.to_owned()))
                // })
                // .collect();
                // Ok(properties)
                let result_pattern = Regex::new(r"^\[([\s\S]*?)\]: \[([\s\S]*?)\]\r?$").unwrap();

                let mut properties = HashMap::new();
                for line in props.split('\n') {
                    if let Some(captures) = result_pattern.captures(line) {
                        let key = captures.get(1).unwrap().as_str().to_string();
                        let value = captures.get(2).unwrap().as_str().to_string();
                        properties.insert(key, value);
                    }
                }
            
                Ok(properties)
            },
            Response::Fail(err) | Response::Unknown(err) => Err(PropertiesErrors::ResponseError(err)),
        }
    }

    pub fn getserial_no(&self) -> String {
        self.serial_no.clone()
    }
}

impl Adb {
    /// Constructs a new Adb instance and runs the server if it isn't running
    pub fn new() -> Result<Self, AdbSocketError> {
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

    /// Private method; Sends the gived command to the server
    fn _exec_cmd(&mut self, cmd: &str, read_response: bool, has_length: bool) -> Result<Response, AdbSocketError> {
        debug!(
            "[Adb::_exec_cmd()]: _exec_cmd called with command '{}'...",
            cmd
        );
        debug!("[Adb::_exec_cmd()]: Preparing header...");
        let hex_length = format!("{:04X}{}", cmd.len(), cmd);

        debug!("[Adb::_exec_cmd()]: Sending request...");
        self.s.write_all(hex_length.as_bytes())?;

        debug!("[Adb::_exec_cmd()]: Recieving response...");
        let mut buffer = [0; 4];
        self.s.read_exact(&mut buffer)?;
        let rp = String::from_utf8(buffer.to_vec())?;

        let mut string_buff = String::new();
        if read_response {
            if has_length {
                // Recieving the response length
                let mut response_length = [0; 4];
                self.s.read_exact(&mut response_length)?;
                let hex_length = i32::from_str_radix(&String::from_utf8(response_length.to_vec())?, 16)?;

                // Recieving the response
                let mut response = vec![0; hex_length as usize];
                self.s.read_exact(&mut response)?;

                string_buff = String::from_utf8(response)?;
            } else {
                self.s.read_to_string(&mut string_buff)?;
            }
            debug!("{:?}", string_buff);
            debug!("[Adb::_exec_cmd()] Returning the string recievied...");
        }
        if &rp == "OKAY" {
            debug!("Returning ok");
            return Ok(Response::Ok(string_buff));
        } else if &rp == "FAIL" {
            return Ok(Response::Fail(string_buff));
        } else {
            return Ok(Response::Unknown(string_buff));
        }
    }

    /// Return a list of devices
    pub fn devices(&mut self) -> Result<Vec<Device>, PradbErrors> {
        let devices = self._exec_cmd("host:devices", true, true)?;
        match devices {
            Response::Ok(rp) => {
                if rp == "0000" {
                    return Ok(vec![]);
                } else {
                    let devices = rp.split('\n').filter(|s| !s.is_empty()).map(|s| {
                        let device_data = s.split("\t").collect::<Vec<&str>>();
                        Device::new(device_data[0].to_string(), device_data[1].to_string()).unwrap()
                    }).collect::<Vec<Device>>();
                    Ok(devices)
                }
            }
            Response::Fail(rp) => {
                return Err(PradbErrors::ResponseRelated(Response::Fail(rp)));
            }
            Response::Unknown(ro) => {
                return Err(PradbErrors::Unknown(Some(ro)));
            }
        }
    }

    // /// Connects to a usb device
    // pub fn device(&mut self, serial_no: String) -> Result<&Device, AdbDeviceError> {
    //     // Ok(self._exec_cmd(&format!("host:transport:{}", serial_no))?)
    //     let devices = self.devices()?;
    //     let devices_clone = devices.clone();
    //     let vec_devices = devices.iter().filter(|s| s.serial_no == serial_no).collect::<Vec<&Device>>();
    //     if vec_devices.is_empty() {
    //         Err(AdbDeviceError::DeviceNotFound(String::from("Device not found")))
    //     } else {
    //         Ok(vec_devices[0])
    //     }

    // }

    /// Shutdown both channels of the socket
    pub fn close(&mut self) -> Result<(), PradbErrors> {
        Ok(self.s.shutdown(std::net::Shutdown::Both)?)
    }

    /// Returns version
    pub fn version(&mut self) -> Result<Response, PradbErrors> {
        Ok(self._exec_cmd("host:version", true, true)?)
    }
}
