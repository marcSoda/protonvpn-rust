use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use log::{info, warn};
use std::path::PathBuf;
use serde_json::Value;
use serde_json::from_str;
use glob::glob;

pub struct VPNMan {
    ovpn_dir: String,
    auth_file: String,
    pid_file: String, // File to store the PID of the OpenVPN process
}

impl VPNMan {
    pub fn new(ovpn_dir: String, auth_file: String, pid_file: String) -> Self {
        VPNMan {
            ovpn_dir,
            auth_file,
            pid_file,
        }
    }

    // span openvpn process and create pidfile so it can be stopped later
    // note: pidfile approach is not good. does not offer a way to check status
    pub fn connect(&self, ovpn_file: PathBuf) {
        if self.is_connected() {
            warn!("A VPN connection is already active.");
            return;
        }

        let child = Command::new("sudo")
            .arg("openvpn")
            .arg("--config")
            .arg(ovpn_file)
            .arg("--auth-user-pass")
            .arg(&self.auth_file)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to start OpenVPN process");

        File::create(&self.pid_file)
            .and_then(|mut file| {
                write!(file, "{}", child.id())
            })
            .expect("Failed to write PID file");

        info!("VPN connection initiated with PID {}", child.id());
    }

    // check the status of the vpn by determining if the pidfile exists
    // this is a bad approach
    pub fn check_status(&self) {
        if self.is_connected() {
            info!("VPN is currently connected.");
        } else {
            warn!("VPN is not connected.");
        }
    }

    // kill the process by the pid stored in the pidfile
    pub fn disconnect(&self) {
        if let Ok(pid) = self.read_pid() {
            Command::new("sudo")
                .arg("kill")
                .arg(pid.to_string())
                .status()
                .expect("Failed to kill OpenVPN process");

            fs::remove_file(&self.pid_file).expect("Failed to remove PID file");
            info!("VPN disconnected successfully.");
        } else {
            warn!("No active VPN connection to disconnect.");
        }
    }

    // check if pidfile exists
    fn is_connected(&self) -> bool {
        self.read_pid().is_ok()
    }

    // read the pid in the pidfile
    fn read_pid(&self) -> io::Result<u32> {
        let mut pid_str = String::new();
        File::open(&self.pid_file)
            .and_then(|mut file| {
                file.read_to_string(&mut pid_str)
            })
            .map(|_| pid_str.trim().parse::<u32>().unwrap())
    }

    // returns the path to the .ovpn file of the server that has the lowest load
    // queries protonvpn api which gives load information
    pub fn get_lowest_load_server(&self) -> Option<PathBuf> {
        let ovpn_files = self.get_ovpn_files().unwrap();
        let response = Self::fetch_server_data("https://api.protonmail.ch/vpn/logicals").unwrap();

        response["LogicalServers"].as_array().unwrap()
            .iter()
            // Filter out servers that are active
            .filter(|server| server["Status"].as_i64().unwrap() == 1)
            // Create an iterator of tuples (load, ovpn_file_path)
            .filter_map(|server| {
                let domain = server["Domain"].as_str().unwrap();
                let load = server["Load"].as_i64().unwrap();
                ovpn_files.iter()
                    .find(|path| path.to_str().unwrap().contains(domain))
                    .map(|path| (load, path.clone()))
            })
            // Find the tuple with the smallest load value
            .min_by_key(|&(load, _)| load)
            // Return only the ovpn_file_path part of the tuple
            .map(|(_, path)| path)
    }

    // grab the api data
    fn fetch_server_data(api_url: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let response = reqwest::blocking::get(api_url)?.text()?;
        from_str(&response).map_err(|e| e.into())
    }

    // get paths of all .ovpn files
    fn get_ovpn_files(&self) -> Result<Vec<PathBuf>, io::Error> {
        glob(self.ovpn_dir.as_str())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
            .and_then(|paths| {
                let files = paths
                    .filter_map(Result::ok)
                    .collect::<Vec<PathBuf>>();
                Ok(files)
            })
    }
}
