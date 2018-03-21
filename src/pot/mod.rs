use walkdir::WalkDir;
use std::net::Ipv4Addr;
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::default::Default;

#[derive(Debug, Clone)]
pub struct SystemConf {
    zfs_root: Option<String>,
    pub fs_root: Option<String>,
    pub network: Option<Ipv4Addr>,
    pub netmask: Option<Ipv4Addr>,
    pub gateway: Option<Ipv4Addr>,
    ext_if: Option<String>,
    dns_name: Option<String>,
    pub dns_ip: Option<Ipv4Addr>,
}

#[derive(Debug)]
pub enum PotError {
    WhichError,
    PathError,
    FileError,
}

fn get_pot_prefix() -> Result<PathBuf, PotError> {
    let pathname = match Command::new("which").arg("pot").output() {
        Ok(s) => s,
        Err(_) => return Err(PotError::WhichError),
    };
    let pot_path = match String::from_utf8(pathname.stdout) {
        Ok(s) => s,
        Err(_) => return Err(PotError::WhichError),
    };
    let pot_path = Path::new(pot_path.trim_right());
    let pot_prefix = match pot_path.parent() {
        Some(i) => i,
        _ => return Err(PotError::PathError),
    };
    let pot_prefix = match pot_prefix.parent() {
        Some(i) => i,
        _ => return Err(PotError::PathError),
    };
    Ok(pot_prefix.to_path_buf())
}

fn get_conf_default() -> Result<String, PotError> {
    let mut pot_conf = match get_pot_prefix() {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.default.conf");

    let mut conf_file = match File::open(pot_conf.as_path()) {
        Ok(x) => x,
        Err(_) => return Err(PotError::FileError),
    };
    let mut conf_str = String::new();
    match conf_file.read_to_string(&mut conf_str) {
        Ok(_) => (),
        Err(_) => return Err(PotError::FileError),
    }
    Ok(conf_str)
}

fn get_conf() -> Result<String, PotError> {
    let mut pot_conf = match get_pot_prefix() {
        Ok(p) => p,
        Err(e) => return Err(e),
    };
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.conf");

    let mut conf_file = match File::open(pot_conf.as_path()) {
        Ok(x) => x,
        Err(_) => return Err(PotError::FileError),
    };
    let mut conf_str = String::new();
    match conf_file.read_to_string(&mut conf_str) {
        Ok(_) => (),
        Err(_) => return Err(PotError::FileError),
    }
    Ok(conf_str)
}

impl Default for SystemConf {
    fn default() -> SystemConf {
        SystemConf {
            zfs_root: None,
            fs_root: None,
            network: None,
            netmask: None,
            gateway: None,
            ext_if: None,
            dns_name: None,
            dns_ip: None,
        }
    }
}

impl FromStr for SystemConf {
    type Err = PotError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut default = SystemConf::default();
        let lines: Vec<String> = s.to_string()
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.starts_with('#'))
            .collect();
        for linestr in &lines {
            if linestr.starts_with("POT_ZFS_ROOT=") {
                default.zfs_root = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_FS_ROOT=") {
                default.fs_root = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_EXTIF=") {
                default.ext_if = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_DNS_NAME=") {
                default.dns_name = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_NETWORK=") {
                default.network = match linestr.split('=').nth(1) {
                    Some(s) => match s.split('/').nth(0) {
                        Some(s) => match s.to_string().parse::<Ipv4Addr>() {
                            Ok(ip) => Some(ip),
                            Err(_) => None,
                        },
                        None => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_NETMASK=") {
                default.netmask = match linestr.split('=').nth(1) {
                    Some(s) => match s.to_string().parse::<Ipv4Addr>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_GATEWAY=") {
                default.gateway = match linestr.split('=').nth(1) {
                    Some(s) => match s.to_string().parse::<Ipv4Addr>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_DNS_IP=") {
                default.dns_ip = match linestr.split('=').nth(1) {
                    Some(s) => match s.to_string().parse::<Ipv4Addr>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
        }
        Ok(default)
    }
}

impl SystemConf {
    pub fn new() -> SystemConf {
        let s = match get_conf_default() {
            Ok(s) => s,
            Err(_) => return SystemConf::default(),
        };

        let mut dconf = SystemConf::from_str(&s).ok().unwrap_or_default();
        let s = match get_conf() {
            Ok(s) => s,
            Err(_) => return dconf,
        };
        let pconf = SystemConf::from_str(&s).ok().unwrap_or_default();
        dconf.merge(pconf);
        dconf
    }
    pub fn is_valid(&self) -> bool {
        self.zfs_root != None && self.fs_root != None && self.network != None
            && self.netmask != None && self.gateway != None && self.ext_if != None
            && self.dns_name != None && self.dns_ip != None
    }
    fn merge(&mut self, rhs: SystemConf) {
        if rhs.zfs_root.is_some() {
            self.zfs_root = Some(rhs.zfs_root.unwrap());
        }
        if rhs.fs_root.is_some() {
            self.fs_root = Some(rhs.fs_root.unwrap());
        }
        self.network = match rhs.network {
            Some(s) => Some(s),
            None => self.network,
        };
        self.netmask = match rhs.netmask {
            Some(s) => Some(s),
            None => self.netmask,
        };
        self.gateway = match rhs.gateway {
            Some(s) => Some(s),
            None => self.gateway,
        };
        if rhs.ext_if.is_some() {
            self.ext_if = Some(rhs.ext_if.unwrap());
        }
        if rhs.dns_name.is_some() {
            self.dns_name = Some(rhs.dns_name.unwrap());
        }
        self.dns_ip = match rhs.dns_ip {
            Some(s) => Some(s),
            None => self.dns_ip,
        };
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum IPType {
    Inherit,
    Static,
    Vnet,
}

impl Default for IPType {
    fn default() -> IPType {
        IPType::Inherit
    }
}

#[derive(Debug)]
pub struct PotConf {
    pub ip_type: IPType,
    pub ip_addr: Ipv4Addr,
}

impl Default for PotConf {
    fn default() -> PotConf {
        PotConf {
            ip_type: IPType::Inherit,
            ip_addr: Ipv4Addr::from(0),
        }
    }
}

pub fn get_pot_conf_list(conf: SystemConf) -> Vec<PotConf> {
    let mut v: Vec<PotConf> = Vec::new();
    if !conf.is_valid() {
        return v;
    }

    for pot_dir in WalkDir::new(conf.fs_root.unwrap() + "/jails")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(|x| x.ok())
        .filter(|x| x.file_type().is_dir())
    {
        let mut dir_path = pot_dir.path().to_path_buf();
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
        let mut pot_conf = PotConf::default();
        for s in conf_str.lines() {
            if s.starts_with("ip4=") {
                let iptype = s.split('=').nth(1).unwrap();
                match iptype {
                    "inherit" => pot_conf.ip_type = IPType::Inherit,
                    ip => pot_conf.ip_addr = Ipv4Addr::from_str(ip).ok().unwrap(),
                }
            }
            if s.starts_with("vnet=") {
                let vnet = s.split('=').nth(1).unwrap();
                match vnet {
                    "true" => pot_conf.ip_type = IPType::Vnet,
                    _ => pot_conf.ip_type = IPType::Static,
                }
            }
        }
        v.push(pot_conf);
    }
    v
}
