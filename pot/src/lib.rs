pub mod bridge;
pub mod error;
mod system;
pub(crate) mod util;

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
pub struct PotDnsConfig {
    pub pot_name: String,
    pub ip: IpAddr,
}

#[derive(Debug, Clone)]
pub struct PotSystemConfig {
    pub zfs_root: String,
    pub fs_root: String,
    pub network: IpNet,
    pub netmask: IpAddr,
    pub gateway: IpAddr,
    pub ext_if: String,
    pub dns: Option<PotDnsConfig>,
}

impl Default for PotSystemConfig {
    fn default() -> Self {
        use std::net::Ipv4Addr;
        PotSystemConfig {
            zfs_root: String::default(),
            fs_root: String::default(),
            network: IpNet::default(),
            netmask: IpAddr::V4(Ipv4Addr::new(255, 255, 255, 0)),
            gateway: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            ext_if: String::default(),
            dns: None,
        }
    }
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
                dns: match psc.dns_ip {
                  Some(ip) => Some(PotDnsConfig{pot_name: psc.dns_name.unwrap(), ip}),
                  None => None
                },
            })
        } else {
            Err(error::PotError::IncompleteSystemConf)
        }
    }
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
    pub aliases: Option<Vec<String>>,
}

#[derive(Debug, Default)]
pub struct PotConfVerbatim {
    pub vnet: Option<String>,
    pub ip4: Option<String>,
    pub ip: Option<String>,
    pub network_type: Option<String>,
    pub aliases: Option<Vec<String>>,
}

impl Default for PotConf {
    fn default() -> PotConf {
        PotConf {
            name: String::default(),
            ip_addr: None,
            network_type: NetType::Inherit,
            aliases: None,
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
        let mut pot_conf = PotConf {
            name: dir_path
                .clone()
                .strip_prefix(&pdir)
                .ok()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            ..Default::default()
        };
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
            if s.starts_with("pot.aliases=") {
                temp_pot_conf.aliases.get_or_insert(Vec::new()).push(s.split('=')
                    .nth(1).unwrap().to_string())
            }
        }
        if let Some(network_type) = temp_pot_conf.network_type {
            pot_conf.aliases = temp_pot_conf.aliases;
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
