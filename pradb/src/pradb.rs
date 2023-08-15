use regex::Regex;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use thiserror::Error;

use log::{debug, error as err, info};

const NO_PREFIX: bool = false;
const HAVE_PREFIX: bool = true;

const NO_RESPONSE: bool = false;
const HAVE_RESPONSE: bool = true;

const NO_LENGTH: bool = false;
const HAVE_LENGTH: bool = true;

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

#[derive(Debug, Error)]
pub enum AdbShellError {
    #[error("socket error")]
    SocketError(#[from] AdbSocketError),
    #[error("the server returned FAIL or unkown")]
    ResponseError(String),
    #[error("unknown error")]
    Unknown,
    #[error("could not parse the exit code")]
    InvalidExitCode(#[from] std::num::ParseIntError),
}

pub struct CommandResult {
    output: String,
    exit_code: i32,
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

#[derive(Debug, Error)]
pub enum SinglePropertyErrors {
    #[error("the adb socket caused an error")]
    ServerError(#[from] AdbSocketError),
    #[error("unknown property")]
    UnknownProperty(String),
}

#[derive(Debug, Error)]
pub enum PackagesError {
    #[error("the adb socket caused an error")]
    ServerError(#[from] AdbSocketError),
    #[error("the server returned FAIL or unkown")]
    ResponseError(String),
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("the adb socket caused an error")]
    ServerError(#[from] AdbShellError),
    #[error("the server returned FAIL or unkown")]
    ResponseError(String),
    #[error("cannot install package")]
    PackageNotInstalled(String),
    #[error("package don't exist")]
    FileNotFound,
}

/// Future device class
#[derive(Debug)]
pub struct Device {
    adb: Adb,
    serial_no: String,
    model: String,
    using_device: bool,
}

impl Device {
    pub fn new(sn: String, model: String) -> Result<Self, PradbErrors> {
        Ok(Device {
            adb: Adb::new()?,
            serial_no: sn,
            model,
            using_device: false,
        })
    }

    pub fn use_device(&mut self) -> Result<Response, AdbSocketError> {
        let response = self.adb._exec_cmd(
            &format!("host:transport:{}", self.serial_no),
            HAVE_PREFIX,
            NO_RESPONSE,
            NO_LENGTH,
        )?;
        self.using_device = true;
        Ok(response)
    }

    /// Sends a shell command to the server
    pub fn shell(&mut self, cmd: &str) -> Result<String, AdbShellError> {
        let cmd_output = self.adb._exec_cmd(
            &format!("shell:{}", cmd),
            HAVE_PREFIX,
            HAVE_RESPONSE,
            NO_LENGTH,
        )?;
        match cmd_output {
            Response::Ok(output) => Ok(output),
            Response::Fail(err) | Response::Unknown(err) => Err(AdbShellError::ResponseError(err)),
        }
    }

    pub fn get_properties(&mut self) -> Result<HashMap<String, String>, PropertiesErrors> {
        let unformatted_props =
            self.adb
                ._exec_cmd("shell:getprop:", HAVE_PREFIX, HAVE_RESPONSE, NO_LENGTH)?;
        match unformatted_props {
            Response::Ok(props) => {
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
            }
            Response::Fail(err) | Response::Unknown(err) => {
                Err(PropertiesErrors::ResponseError(err))
            }
        }
    }

    pub fn get_property(&mut self, property: &str) -> Result<Response, SinglePropertyErrors> {
        Ok(self.adb._exec_cmd(
            &format!("shell:getprop:{}", property),
            HAVE_PREFIX,
            NO_RESPONSE,
            NO_LENGTH,
        )?)
    }

    pub fn list_packages(&mut self) -> Result<Vec<String>, PackagesError> {
        let unformatted_pkgs = self.adb._exec_cmd(
            "shell:pm list packages 2> /dev/null",
            HAVE_PREFIX,
            HAVE_RESPONSE,
            NO_LENGTH,
        )?;
        match unformatted_pkgs {
            Response::Ok(pkgs) => {
                let lines = pkgs
                    .split('\n')
                    .filter(|line| !line.is_empty())
                    .map(|line| line.to_string().drain(8..).collect::<String>());

                Ok(lines.collect::<Vec<String>>())
            }
            Response::Fail(err) | Response::Unknown(err) => Err(PackagesError::ResponseError(err)),
        }
    }

    pub fn install_package(&mut self, package: PathBuf) -> Result<(), InstallError> {
        if !package.exists() {
            return Err(InstallError::FileNotFound);
        }

        let pkg_output = self.shell(&format!(
            "pm install {}",
            package.as_os_str().to_str().unwrap()
        ))?;
        if pkg_output.contains("Error:") {
            Err(InstallError::PackageNotInstalled(pkg_output))
        } else {
            Ok(())
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
                Ok(Adb { s: sock })
            }
            Err(err) => {
                err!("Adb::new(): Couldn't connect to the server. Error: {}", err);
                err!("Adb::new(): Trying to run server...");
                Err(AdbSocketError::ServerError(err.to_string()))
            }
        }
    }

    /// Private method; Sends the gived command to the server
    fn _exec_cmd(
        &mut self,
        cmd: &str,
        has_prefix: bool,
        read_response: bool,
        has_length: bool,
    ) -> Result<Response, AdbSocketError> {
        debug!(
            "[Adb::_exec_cmd()]: _exec_cmd called with command '{}'...",
            cmd
        );
        debug!("[Adb::_exec_cmd()]: Preparing header...");
        let hex_length = format!("{:04X}{}", cmd.len(), cmd);

        debug!("[Adb::_exec_cmd()]: Sending request...");
        self.s.write_all(hex_length.as_bytes())?;

        let rp = {
            if has_prefix {
                debug!("[Adb::_exec_cmd()]: Recieving response...");
                let mut buffer = vec![0; 4];
                self.s.read_exact(&mut buffer)?;
                String::from_utf8(buffer.to_vec())?
            } else {
                String::new()
            }
        };

        debug!("[Adb::_exec_cmd()]: Creating buffer...");
        let mut string_buff = String::new();
        if read_response {
            if has_length {
                // Recieving the response length
                let mut response_length = [0; 4];
                self.s.read_exact(&mut response_length)?;
                let hex_length =
                    i32::from_str_radix(&String::from_utf8(response_length.to_vec())?, 16)?;

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
            Ok(Response::Ok(string_buff))
        } else if &rp == "FAIL" {
            Ok(Response::Fail(string_buff))
        } else {
            Ok(Response::Unknown(string_buff))
        }
    }

    /// Return a list of devices
    pub fn devices(&mut self) -> Result<Vec<Device>, PradbErrors> {
        let devices = self._exec_cmd("host:devices", HAVE_PREFIX, HAVE_RESPONSE, HAVE_LENGTH)?;
        match devices {
            Response::Ok(rp) => {
                if rp == "0000" {
                    Ok(vec![])
                } else {
                    let devices = rp
                        .split('\n')
                        .filter(|s| !s.is_empty())
                        .map(|s| {
                            let device_data = s.split('\t').collect::<Vec<&str>>();
                            Device::new(device_data[0].to_string(), device_data[1].to_string())
                                .unwrap()
                        })
                        .collect::<Vec<Device>>();
                    Ok(devices)
                }
            }
            Response::Fail(rp) => Err(PradbErrors::ResponseRelated(Response::Fail(rp))),
            Response::Unknown(ro) => Err(PradbErrors::Unknown(Some(ro))),
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
        Ok(self._exec_cmd("host:version", HAVE_PREFIX, HAVE_RESPONSE, HAVE_LENGTH)?)
    }
}

// fn is_normal<T: Sized + Send + Sync + Unpin>() {}

// fn test() {
//     is_normal::<Device>();
// }
