use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Instant;
use thiserror::Error;

use log::{debug, error as err, info};

/// Adb errors
#[derive(Debug, Error)]
pub enum AdbSocketError {
    #[error("Server not running")]
    ServerError(String),
    #[error("Cannot retrieve the server response")]
    RecvError(String),
    #[error("Socket writing error")]
    WriteError(#[from] io::Error),
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
    fn _exec_cmd(&mut self, cmd: &str) -> Result<Response, AdbSocketError> {
        debug!(
            "[Adb::_exec_cmd()]: _exec_cmd called with command '{}'...",
            cmd
        );
        debug!("[Adb::_exec_cmd()]: Preparing header...");
        let hex_length = format!("{:04X}{}", cmd.len(), cmd);
        debug!("[Adb::_exec_cmd()]: Sending request...");

        let send_time: Instant = Instant::now();
        self.s.write_all(hex_length.as_bytes())?;
        debug!(
            "[Adb::_exec_cmd()]: Took {}s",
            send_time.elapsed().as_secs_f32()
        );

        debug!("[Adb::_exec_cmd()]: Recieving response...");
        let recv_time: Instant = Instant::now();
        let mut buffer = [0; 4];
        self.s.read_exact(&mut buffer)?;
        debug!(
            "[Adb::_exec_cmd()]: Took {}s",
            recv_time.elapsed().as_secs_f32()
        );

        match String::from_utf8(buffer.to_vec()) {
            Ok(rp) => {
                let mut string_buff = String::new();

                if let Err(err) = self.s.read_to_string(&mut string_buff) {
                    err!("[Adb::_exec_cmd()]: Error while trying to read the server response. Error: {err}");
                }

                debug!("{:?}", string_buff);
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
    }

    /// Return a list of devices
    pub fn devices(&mut self) -> Result<Vec<Device>, PradbErrors> {
        let devices = self._exec_cmd("host:devices")?;
        match devices {
            Response::Ok(mut rp) => {
                if rp == "0000" {
                    return Ok(vec![]);
                } else {
                    let response = rp.drain(4..).collect::<String>();
                    let devices = response.split('\n').filter(|s| !s.is_empty()).map(|s| {
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
        Ok(self._exec_cmd("host:version")?)
    }
}
