use env_logger;
use clap::{App, SubCommand};
use vpn_man::VPNMan;

pub mod vpn_man;

fn main() {
    env_logger::init();
    // let api_url = "https://api.protonmail.ch/vpn/logicals";        // harcoded in vpnman
    let ovpn_dir = "/home/marc/working/temp/pvpn/ovpns/*.ovpn";       // glob where .ovpn config files are stored
    let auth_file = "/home/marc/working/temp/pvpn/ovpns/login.conf";  // path to protonvpn credentials
    let pid_file = "/tmp/ovpn_pid_file";                              // place to store pid of spawned openvpn process

    let matches = App::new("ProtonVPN CLI")
        .version("1.0")
        .author("Your Name")
        .about("Manages VPN connections")
        .subcommand(SubCommand::with_name("connect")
            .about("Connects to the VPN"))
        .subcommand(SubCommand::with_name("status")
            .about("Checks VPN status"))
        .subcommand(SubCommand::with_name("disconnect")
            .about("Disconnects the VPN"))
        .get_matches();

    let vpnman = VPNMan::new(ovpn_dir.to_string(), auth_file.to_string(), pid_file.to_string());

    match matches.subcommand_name() {
        Some("connect") => {
            let server = vpnman.get_lowest_load_server().unwrap();
            vpnman.connect(server);
        }
        Some("status") => {
            vpnman.check_status();
        }
        Some("disconnect") => {
            vpnman.disconnect();
        }
        _ => println!("Unknown command"),
    }
}
