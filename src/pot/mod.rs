use ipnet::IpNet;
use std::default::Default;
use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq)]
pub struct SystemConf {
    zfs_root: Option<String>,
    pub fs_root: Option<String>,
    pub network: Option<IpNet>,
    pub netmask: Option<IpAddr>,
    pub gateway: Option<IpAddr>,
    ext_if: Option<String>,
    pub dns_name: Option<String>,
    pub dns_ip: Option<IpAddr>,
}

#[derive(Debug, Clone, Copy)]
pub enum PotError {
    WhichError,
    PathError,
    FileError,
    JlsError,
}

// get pot prefix in the same way as pot does:
// find PREFIX/bin/pot and get the PREFIX
fn get_pot_prefix() -> Result<PathBuf, PotError> {
    let pathname = Command::new("which")
        .arg("pot")
        .output()
        .map_err(|_| PotError::WhichError)?;
    let pot_path =
        PathBuf::from(String::from_utf8(pathname.stdout).map_err(|_| PotError::WhichError)?);
    let pot_prefix = pot_path.parent().ok_or(PotError::PathError)?;
    let pot_prefix = pot_prefix.parent().ok_or(PotError::PathError)?;
    Ok(pot_prefix.to_path_buf())
}

fn get_conf_default() -> Result<String, PotError> {
    let mut pot_conf = get_pot_prefix()?;
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.default.conf");

    let mut conf_file = File::open(pot_conf.as_path()).map_err(|_| PotError::FileError)?;
    let mut conf_str = String::new();
    let _ = conf_file
        .read_to_string(&mut conf_str)
        .map_err(|_| PotError::FileError)?;
    Ok(conf_str)
}

fn get_conf() -> Result<String, PotError> {
    let mut pot_conf = get_pot_prefix()?;
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.conf");

    let mut conf_file = File::open(pot_conf.as_path()).map_err(|_| PotError::FileError)?;
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
    /// Create a pot System configuration from a string
    ///
    /// # Examples
    ///
    /// ```
    /// use std::net::Ipv4Addr;
    /// use std::str::FromStr;
    /// let uut = potnet::pot::SystemConf::from_str("POT_GATEWAY=192.168.0.1\nPOT_DNS_NAME=test-dns");
    /// assert_eq!(uut.is_ok(), true);
    /// let uut = uut.unwrap();
    /// assert_eq!(uut.is_valid(), false);
    /// assert_eq!(uut.gateway.is_some(), true);
    /// assert_eq!(
    ///     uut.gateway.unwrap(),
    ///     "192.168.0.1".parse::<Ipv4Addr>().unwrap()
    /// );
    /// assert_eq!(uut.dns_name.is_some(), true);
    /// assert_eq!(uut.dns_name.unwrap(), "test-dns".to_string());
    /// assert_eq!(uut.dns_ip.is_none(), true);
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut default = SystemConf::default();
        let lines: Vec<String> = s
            .to_string()
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.starts_with('#'))
            .collect();
        for linestr in &lines {
            if linestr.starts_with("POT_ZFS_ROOT=") {
                default.zfs_root = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.split(' ').nth(0).unwrap().to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_FS_ROOT=") {
                default.fs_root = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.split(' ').nth(0).unwrap().to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_EXTIF=") {
                default.ext_if = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.split(' ').nth(0).unwrap().to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_DNS_NAME=") {
                default.dns_name = match linestr.split('=').nth(1) {
                    Some(s) => Some(s.split(' ').nth(0).unwrap().to_string()),
                    None => None,
                }
            }
            if linestr.starts_with("POT_NETWORK=") {
                default.network = match linestr.split('=').nth(1) {
                    Some(s) => match s.split(' ').nth(0).unwrap().to_string().parse::<IpNet>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_NETMASK=") {
                default.netmask = match linestr.split('=').nth(1) {
                    Some(s) => match s.split(' ').nth(0).unwrap().to_string().parse::<IpAddr>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_GATEWAY=") {
                default.gateway = match linestr.split('=').nth(1) {
                    Some(s) => match s.split(' ').nth(0).unwrap().to_string().parse::<IpAddr>() {
                        Ok(ip) => Some(ip),
                        Err(_) => None,
                    },
                    None => None,
                };
            }
            if linestr.starts_with("POT_DNS_IP=") {
                default.dns_ip = match linestr.split('=').nth(1) {
                    Some(s) => match s.split(' ').nth(0).unwrap().to_string().parse::<IpAddr>() {
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
        self.zfs_root != None
            && self.fs_root != None
            && self.network != None
            && self.netmask != None
            && self.gateway != None
            && self.ext_if != None
            && self.dns_name != None
            && self.dns_ip != None
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

#[derive(Debug)]
pub struct BridgeConf {
    pub name: String,
    pub network: IpNet,
    pub gateway: IpAddr,
}

impl FromStr for BridgeConf {
    type Err = PotError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
        if name.is_some() && network.is_some() && gateway.is_some() {
            Ok(BridgeConf {
                name: name.unwrap(),
                network: network.unwrap(),
                gateway: gateway.unwrap(),
            })
        } else {
            Err(PotError::FileError)
        }
    }
}
pub fn get_bridges_path_list(conf: &SystemConf) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let fsroot = conf.fs_root.clone().unwrap();
    WalkDir::new(fsroot + "/bridges")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|x| x.file_type().is_file())
        .for_each(|x| result.push(x.into_path()));
    result
}

pub fn get_bridges_list(conf: &SystemConf) -> Vec<BridgeConf> {
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

fn get_pot_path_list(conf: &SystemConf) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let fsroot = conf.fs_root.clone().unwrap();
    WalkDir::new(fsroot + "/jails")
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|x| x.file_type().is_dir())
        .for_each(|x| result.push(x.into_path()));
    result
}

pub fn get_pot_list(conf: &SystemConf) -> Vec<String> {
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

fn is_pot_running(pot_name: &str) -> Result<bool, PotError> {
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
        Err(PotError::JlsError)
    }
}

pub fn get_running_pot_list(conf: &SystemConf) -> Vec<String> {
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

pub fn get_pot_conf_list(conf: SystemConf) -> Vec<PotConf> {
    let mut v: Vec<PotConf> = Vec::new();
    if !conf.is_valid() {
        return v;
    }

    let fsroot = conf.fs_root.clone().unwrap();
    let pdir = fsroot.clone() + "/jails/";
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
                _ => continue,
            };
            if pot_conf.network_type == NetType::Alias
                || pot_conf.network_type == NetType::PublicBridge
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
    fn system_conf_default() {
        let uut = SystemConf::default();
        assert_eq!(uut.is_valid(), false);
        assert_eq!(uut.dns_ip, None);
        assert_eq!(uut.dns_name, None);
        assert_eq!(uut.ext_if, None);
        assert_eq!(uut.fs_root, None);
        assert_eq!(uut.gateway, None);
        assert_eq!(uut.netmask, None);
        assert_eq!(uut.network, None);
        assert_eq!(uut.zfs_root, None);
    }

    #[test]
    fn system_conf_fromstr_001() {
        let uut = SystemConf::from_str("");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_eq!(uut, SystemConf::default());
    }

    #[test]
    fn system_conf_fromstr_002() {
        let uut = SystemConf::from_str("# Comment 1\n # Comment with space");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_eq!(uut, SystemConf::default());
    }

    #[test]
    fn system_conf_fromstr_003() {
        let uut = SystemConf::from_str(" # POT_GATEWAY=192.168.0.1");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_eq!(uut, SystemConf::default());
    }

    #[test]
    fn system_conf_fromstr_004() {
        let uut = SystemConf::from_str("POT_GATEWAY=192.168.0.1");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.gateway.is_some(), true);
        assert_eq!(
            uut.gateway.unwrap(),
            "192.168.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn system_conf_fromstr_005() {
        let uut = SystemConf::from_str("POT_NETWORK=192.168.0.0");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_eq!(uut.network.is_some(), false);
    }

    #[test]
    fn system_conf_fromstr_006() {
        let uut = SystemConf::from_str("POT_NETWORK=192.168.0.0/24");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.network.is_some(), true);
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/24".parse::<IpNet>().unwrap()
        );
    }

    #[test]
    fn system_conf_fromstr_007() {
        let uut = SystemConf::from_str("POT_DNS_NAME=FOO_DNS");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.dns_name.is_some(), true);
        assert_eq!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn system_conf_fromstr_008() {
        let uut = SystemConf::from_str("POT_DNS_NAME=\"FOO_DNS\"");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.dns_name.is_some(), true);
        assert_ne!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn system_conf_fromstr_009() {
        let uut = SystemConf::from_str("POT_DNS_NAME=FOO_DNS # dns pot name");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.dns_name.is_some(), true);
        assert_eq!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn system_conf_fromstr_010() {
        let uut = SystemConf::from_str("POT_DNS_IP=192.168.240.240 # dns pot ip");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.dns_ip.is_some(), true);
        assert_eq!(
            uut.dns_ip.unwrap(),
            "192.168.240.240".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn system_conf_fromstr_011() {
        let uut = SystemConf::from_str("POT_NETWORK=192.168.0.0/22 # pots internal network");
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), false);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.network.is_some(), true);
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/22".parse::<IpNet>().unwrap()
        );
    }

    #[test]
    fn system_conf_fromstr_050() {
        let uut = SystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        );
        assert_eq!(uut.is_ok(), true);
        let uut = uut.unwrap();
        assert_eq!(uut.is_valid(), true);
        assert_ne!(uut, SystemConf::default());
        assert_eq!(uut.network.is_some(), true);
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/24".parse::<IpNet>().unwrap()
        );
        assert_eq!(uut.netmask.is_some(), true);
        assert_eq!(
            uut.netmask.unwrap(),
            "255.255.255.0".parse::<IpAddr>().unwrap()
        );
        assert_eq!(uut.gateway.is_some(), true);
        assert_eq!(
            uut.gateway.unwrap(),
            "192.168.0.1".parse::<IpAddr>().unwrap()
        );
        assert_eq!(uut.dns_ip.is_some(), true);
        assert_eq!(
            uut.dns_ip.unwrap(),
            "192.168.0.2".parse::<IpAddr>().unwrap()
        );
        assert_eq!(uut.zfs_root.is_some(), true);
        assert_eq!(uut.zfs_root.unwrap(), "zroot/pot".to_string());
        assert_eq!(uut.fs_root.is_some(), true);
        assert_eq!(uut.fs_root.unwrap(), "/opt/pot".to_string());
        assert_eq!(uut.ext_if.is_some(), true);
        assert_eq!(uut.ext_if.unwrap(), "em0".to_string());
        assert_eq!(uut.dns_name.is_some(), true);
        assert_eq!(uut.dns_name.unwrap(), "bar_dns".to_string());
    }

    #[test]
    fn system_conf_merge_001() {
        let mut uut = SystemConf::default();
        let uut2 = SystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        )
        .unwrap();
        uut.merge(uut2.clone());
        assert_eq!(uut, uut2);
    }

    #[test]
    fn system_conf_merge_002() {
        let mut uut = SystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        )
        .unwrap();
        let uut2 = SystemConf::from_str("POT_DNS_NAME=foo_dns").unwrap();
        uut.merge(uut2);
        assert_eq!(
            uut,
            SystemConf::from_str(
                "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=foo_dns"
            )
            .unwrap()
        );
    }
}
