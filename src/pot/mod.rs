pub mod error;
mod system;

use ipnet::IpNet;
use std::convert::TryFrom;
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use walkdir::WalkDir;

pub type Result<T> = ::std::result::Result<T, error::PotError>;

#[derive(Debug, Clone)]
pub struct PotSystemConfig {
    pub zfs_root: String,
    pub fs_root: String,
    pub network: IpNet,
    pub netmask: IpAddr,
    pub gateway: IpAddr,
    pub ext_if: String,
    pub dns_name: String,
    pub dns_ip: IpAddr,
}

impl PotSystemConfig {
    pub fn from_system() -> Result<Self> {
        let psc = system::PartialSystemConf::new();
        PotSystemConfig::try_from(psc)
    }
}

impl TryFrom<system::PartialSystemConf> for PotSystemConfig {
    type Error = error::PotError;

    fn try_from(psc: system::PartialSystemConf) -> std::result::Result<Self, Self::Error> {
        if psc.is_valid() {
            Ok(PotSystemConfig {
                zfs_root: psc.zfs_root.unwrap(),
                fs_root: psc.fs_root.unwrap(),
                network: psc.network.unwrap(),
                netmask: psc.netmask.unwrap(),
                gateway: psc.gateway.unwrap(),
                ext_if: psc.ext_if.unwrap(),
                dns_name: psc.dns_name.unwrap(),
                dns_ip: psc.dns_ip.unwrap(),
            })
        } else {
            Err(error::PotError::IncompleteSystemConf)
        }
    }
}

#[derive(Debug)]
pub struct BridgeConf {
    pub name: String,
    pub network: IpNet,
    pub gateway: IpAddr,
}

impl BridgeConf {
    fn optional_new(
        o_name: Option<String>,
        o_network: Option<IpNet>,
        o_gateway: Option<IpAddr>,
    ) -> Option<BridgeConf> {
        if let Some(name) = o_name {
            if let Some(network) = o_network {
                if let Some(gateway) = o_gateway {
                    if network.contains(&gateway) {
                        return Some(BridgeConf {
                            name,
                            network,
                            gateway,
                        });
                    }
                }
            }
        }
        None
    }
}

impl FromStr for BridgeConf {
    type Err = error::PotError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let lines: Vec<String> = s
            .to_string()
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.starts_with('#'))
            .collect();
        let mut name = None;
        let mut network = None;
        let mut gateway = None;
        for linestr in &lines {
            if linestr.starts_with("name=") {
                name = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.split(' ').nth(0).unwrap().to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("net=") {
                let temp_string = match linestr.split('=').nth(1) {
                    Some(s) => s.split(' ').nth(0).unwrap().to_string(),
                    None => "".to_string(),
                };
                network = match temp_string.parse() {
                    Ok(n) => Some(n),
                    Err(_) => None,
                }
            }
            if linestr.starts_with("gateway=") {
                let temp_string = match linestr.split('=').nth(1) {
                    Some(s) => s.split(' ').nth(0).unwrap().to_string(),
                    None => "".to_string(),
                };
                gateway = match temp_string.parse() {
                    Ok(n) => Some(n),
                    Err(_) => None,
                }
            }
        }
        BridgeConf::optional_new(name, network, gateway).ok_or(error::PotError::BridgeConfError)
    }
}
pub fn get_bridges_path_list(conf: &PotSystemConfig) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let fsroot = conf.fs_root.clone();
    WalkDir::new(fsroot + "/bridges")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|x| x.file_type().is_file())
        .for_each(|x| result.push(x.into_path()));
    result
}

pub fn get_bridges_list(conf: &PotSystemConfig) -> Vec<BridgeConf> {
    let path_list = get_bridges_path_list(conf);
    let mut result = Vec::new();
    for f in path_list {
        let mut bridge_file = match File::open(f.as_path()) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let mut conf_str = String::new();
        match bridge_file.read_to_string(&mut conf_str) {
            Ok(_) => (),
            Err(_) => continue,
        }
        if let Ok(bridge_conf) = conf_str.parse() {
            result.push(bridge_conf);
        } else {
            continue;
        }
    }
    result
}

#[derive(Debug, PartialEq, Eq)]
pub enum NetType {
    Inherit,
    Alias,
    PublicBridge,
    PrivateBridge,
}

#[derive(Debug)]
pub struct PotConf {
    pub name: String,
    pub ip_addr: Option<IpAddr>,
    pub network_type: NetType,
}

#[derive(Debug, Default)]
pub struct PotConfVerbatim {
    pub vnet: Option<String>,
    pub ip4: Option<String>,
    pub ip: Option<String>,
    pub network_type: Option<String>,
}

impl Default for PotConf {
    fn default() -> PotConf {
        PotConf {
            name: String::default(),
            ip_addr: None,
            network_type: NetType::Inherit,
        }
    }
}

fn get_pot_path_list(conf: &PotSystemConfig) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let fsroot = conf.fs_root.clone();
    WalkDir::new(fsroot + "/jails")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|x| x.file_type().is_dir())
        .for_each(|x| result.push(x.into_path()));
    result
}

pub fn get_pot_list(conf: &PotSystemConfig) -> Vec<String> {
    let mut result = Vec::new();
    for pot_dir in get_pot_path_list(conf) {
        if let Some(pot_name) = pot_dir.file_name() {
            if let Some(pot_name_str) = pot_name.to_str() {
                result.push(pot_name_str.to_string());
            }
        }
    }
    result
}

fn is_pot_running(pot_name: &str) -> Result<bool> {
    let status = Command::new("/usr/sbin/jls")
        .arg("-j")
        .arg(pot_name)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .status();
    if let Ok(status) = status {
        Ok(status.success())
    } else {
        Err(error::PotError::JlsError)
    }
}

pub fn get_running_pot_list(conf: &PotSystemConfig) -> Vec<String> {
    let mut result = Vec::new();
    for pot in get_pot_list(conf) {
        if let Ok(status) = is_pot_running(&pot) {
            if status {
                result.push(pot);
            }
        }
    }
    result
}

pub fn get_pot_conf_list(conf: PotSystemConfig) -> Vec<PotConf> {
    let mut v: Vec<PotConf> = Vec::new();

    let fsroot = conf.fs_root.clone();
    let pdir = fsroot + "/jails/";
    for mut dir_path in get_pot_path_list(&conf) {
        let mut pot_conf = PotConf::default();
        pot_conf.name = dir_path
            .clone()
            .strip_prefix(&pdir)
            .ok()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        dir_path.push("conf");
        dir_path.push("pot.conf");
        let mut conf_file = match File::open(dir_path.as_path()) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let mut conf_str = String::new();
        match conf_file.read_to_string(&mut conf_str) {
            Ok(_) => (),
            Err(_) => continue,
        }
        let mut temp_pot_conf = PotConfVerbatim::default();
        for s in conf_str.lines() {
            if s.starts_with("ip4=") {
                temp_pot_conf.ip4 = Some(s.split('=').nth(1).unwrap().to_string());
            }
            if s.starts_with("ip=") {
                temp_pot_conf.ip = Some(s.split('=').nth(1).unwrap().to_string());
            }
            if s.starts_with("vnet=") {
                temp_pot_conf.vnet = Some(s.split('=').nth(1).unwrap().to_string());
            }
            if s.starts_with("network_type=") {
                temp_pot_conf.network_type = Some(s.split('=').nth(1).unwrap().to_string());
            }
        }
        if let Some(network_type) = temp_pot_conf.network_type {
            pot_conf.network_type = match network_type.as_str() {
                "inherit" => NetType::Inherit,
                "alias" => NetType::Alias,
                "public-bridge" => NetType::PublicBridge,
                "private-bridge" => NetType::PrivateBridge,
                _ => continue,
            };
            if pot_conf.network_type == NetType::Alias {
                continue;
            }
            if pot_conf.network_type == NetType::PublicBridge
                || pot_conf.network_type == NetType::PrivateBridge
            {
                if let Some(ip_addr) = temp_pot_conf.ip {
                    pot_conf.ip_addr = Some(IpAddr::from_str(&ip_addr).ok().unwrap())
                } else {
                    // Error !
                    continue;
                }
            }
        } else if let Some(ip4) = temp_pot_conf.ip4 {
            // Old pot version - compatibility mode
            if &ip4 == "inherit" {
                pot_conf.network_type = NetType::Inherit;
            } else {
                pot_conf.ip_addr = Some(IpAddr::from_str(&ip4).ok().unwrap());
                if let Some(vnet) = temp_pot_conf.vnet {
                    if &vnet == "true" {
                        pot_conf.network_type = NetType::PublicBridge;
                    } else {
                        pot_conf.network_type = NetType::Alias;
                    }
                } else {
                    // Error
                    continue;
                }
            }
        } else {
            // Error !
            continue;
        }
        v.push(pot_conf);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_conf_fromstr_001() {
        let uut = BridgeConf::from_str("");
        assert_eq!(uut.is_ok(), false);
    }

    #[test]
    fn bridge_conf_fromstr_002() {
        let uut = BridgeConf::from_str("net=10.192.0.24/29");
        assert_eq!(uut.is_ok(), false);
    }

    #[test]
    fn bridge_conf_fromstr_003() {
        let uut = BridgeConf::from_str("gateway=10.192.0.24");
        assert_eq!(uut.is_ok(), false);
    }

    #[test]
    fn bridge_conf_fromstr_004() {
        let uut = BridgeConf::from_str("name=test-bridge");
        assert_eq!(uut.is_ok(), false);
    }

    #[test]
    fn bridge_conf_fromstr_005() {
        let uut = BridgeConf::from_str("net=10.192.0.24/29\ngateway=10.192.1.25\nname=test-bridge");
        assert_eq!(uut.is_ok(), false);
    }
}
