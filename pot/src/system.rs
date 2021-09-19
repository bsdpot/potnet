use crate::error::PotError;
use crate::Result;
use ipnet::IpNet;
use std::default::Default;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

#[derive(Default, Debug, Clone, PartialEq)]
pub(crate) struct PartialSystemConf {
    pub(crate) zfs_root: Option<String>,
    pub(crate) fs_root: Option<String>,
    pub(crate) network: Option<IpNet>,
    pub(crate) netmask: Option<IpAddr>,
    pub(crate) gateway: Option<IpAddr>,
    pub(crate) ext_if: Option<String>,
    pub(crate) dns_name: Option<String>,
    pub(crate) dns_ip: Option<IpAddr>,
}

impl PartialSystemConf {
    pub fn new() -> PartialSystemConf {
        let s = match get_conf_default() {
            Ok(s) => s,
            Err(_) => return PartialSystemConf::default(),
        };

        let mut dconf = PartialSystemConf::from_str(&s).ok().unwrap_or_default();
        let s = match get_conf() {
            Ok(s) => s,
            Err(_) => return dconf,
        };
        let pconf = PartialSystemConf::from_str(&s).ok().unwrap_or_default();
        dconf.merge(pconf);
        dconf
    }

    pub fn is_valid(&self) -> bool {
        self.zfs_root.is_some()
            && self.fs_root.is_some()
            && self.network.is_some()
            && self.netmask.is_some()
            && self.gateway.is_some()
            && self.ext_if.is_some()
            && self.dns_name.is_some()
            && self.dns_ip.is_some()
    }

    fn merge(&mut self, rhs: PartialSystemConf) {
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

impl FromStr for PartialSystemConf {
    type Err = PotError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        use crate::util::get_value;
        let mut default = PartialSystemConf::default();
        let lines: Vec<String> = s
            .to_string()
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.starts_with('#'))
            .collect();
        for linestr in &lines {
            if linestr.starts_with("POT_ZFS_ROOT=") {
                default.zfs_root = get_value(linestr);
            }
            if linestr.starts_with("POT_FS_ROOT=") {
                default.fs_root = get_value(linestr);
            }
            if linestr.starts_with("POT_EXTIF=") {
                default.ext_if = get_value(linestr);
            }
            if linestr.starts_with("POT_DNS_NAME=") {
                default.dns_name = get_value(linestr);
            }
            if linestr.starts_with("POT_NETWORK=") {
                default.network = get_value(linestr);
            }
            if linestr.starts_with("POT_NETMASK=") {
                default.netmask = get_value(linestr);
            }
            if linestr.starts_with("POT_GATEWAY=") {
                default.gateway = get_value(linestr);
            }
            if linestr.starts_with("POT_DNS_IP=") {
                default.dns_ip = get_value(linestr);
            }
        }
        Ok(default)
    }
}

pub(crate) fn get_conf_default() -> Result<String> {
    let mut pot_conf = get_pot_prefix()?;
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.default.conf");

    let result = std::fs::read_to_string(pot_conf)?;
    Ok(result)
}

pub(crate) fn get_conf() -> Result<String> {
    let mut pot_conf = get_pot_prefix()?;
    pot_conf.push("etc");
    pot_conf.push("pot");
    pot_conf.push("pot.conf");

    let result = std::fs::read_to_string(pot_conf)?;
    Ok(result)
}

// get pot prefix in the same way as pot does:
// find PREFIX/bin/pot and get the PREFIX
fn get_pot_prefix() -> Result<PathBuf> {
    let pathname = Command::new("which")
        .arg("pot")
        .output()
        .map_err(|_| PotError::WhichError("pot".to_string()))?;
    if !pathname.status.success() {
        return Err(PotError::WhichError("pot".to_string()));
    }
    let pot_path = PathBuf::from(String::from_utf8(pathname.stdout)?);
    let pot_prefix = pot_path
        .parent()
        .ok_or_else(|| PotError::PathError(format!("{}", pot_path.display())))?;
    let pot_prefix = pot_prefix
        .parent()
        .ok_or_else(|| PotError::PathError(format!("{}", pot_prefix.display())))?;
    Ok(pot_prefix.to_path_buf())
}
#[cfg(test)]

mod tests {
    use super::*;

    #[test]
    fn partial_system_conf_default() {
        let uut = PartialSystemConf::default();
        assert!(!uut.is_valid());
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
    fn partial_system_conf_fromstr_001() {
        let uut = PartialSystemConf::from_str("");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_eq!(uut, PartialSystemConf::default());
    }

    #[test]
    fn partial_system_conf_fromstr_002() {
        let uut = PartialSystemConf::from_str("# Comment 1\n # Comment with space");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_eq!(uut, PartialSystemConf::default());
    }

    #[test]
    fn partial_system_conf_fromstr_003() {
        let uut = PartialSystemConf::from_str(" # POT_GATEWAY=192.168.0.1");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_eq!(uut, PartialSystemConf::default());
    }

    #[test]
    fn partial_system_conf_fromstr_004() {
        let uut = PartialSystemConf::from_str("POT_GATEWAY=192.168.0.1");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.gateway.is_some());
        assert_eq!(
            uut.gateway.unwrap(),
            "192.168.0.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn partial_system_conf_fromstr_005() {
        let uut = PartialSystemConf::from_str("POT_NETWORK=192.168.0.0");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert!(!uut.network.is_some());
    }

    #[test]
    fn partial_system_conf_fromstr_006() {
        let uut = PartialSystemConf::from_str("POT_NETWORK=192.168.0.0/24");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.network.is_some());
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/24".parse::<IpNet>().unwrap()
        );
    }

    #[test]
    fn partial_system_conf_fromstr_007() {
        let uut = PartialSystemConf::from_str("POT_DNS_NAME=FOO_DNS");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.dns_name.is_some());
        assert_eq!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn partial_system_conf_fromstr_008() {
        let uut = PartialSystemConf::from_str("POT_DNS_NAME=\"FOO_DNS\"");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.dns_name.is_some());
        assert_ne!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn partial_system_conf_fromstr_009() {
        let uut = PartialSystemConf::from_str("POT_DNS_NAME=FOO_DNS # dns pot name");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.dns_name.is_some());
        assert_eq!(uut.dns_name.unwrap(), "FOO_DNS".to_string());
    }

    #[test]
    fn partial_system_conf_fromstr_010() {
        let uut = PartialSystemConf::from_str("POT_DNS_IP=192.168.240.240 # dns pot ip");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.dns_ip.is_some());
        assert_eq!(
            uut.dns_ip.unwrap(),
            "192.168.240.240".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn partial_system_conf_fromstr_011() {
        let uut = PartialSystemConf::from_str("POT_NETWORK=192.168.0.0/22 # pots internal network");
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.network.is_some());
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/22".parse::<IpNet>().unwrap()
        );
    }

    #[test]
    fn partial_system_conf_fromstr_012() {
        let uut = PartialSystemConf::from_str(
            "POT_NETWORK=fdf1:186e:49e6:76d8::/64 # pots internal network",
        );
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(!uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.network.is_some());
        assert_eq!(
            uut.network.unwrap(),
            "fdf1:186e:49e6:76d8::/64".parse::<IpNet>().unwrap()
        );
    }

    #[test]
    fn partial_system_conf_fromstr_050() {
        let uut = PartialSystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        );
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.network.is_some());
        assert_eq!(
            uut.network.unwrap(),
            "192.168.0.0/24".parse::<IpNet>().unwrap()
        );
        assert!(uut.netmask.is_some());
        assert_eq!(
            uut.netmask.unwrap(),
            "255.255.255.0".parse::<IpAddr>().unwrap()
        );
        assert!(uut.gateway.is_some());
        assert_eq!(
            uut.gateway.unwrap(),
            "192.168.0.1".parse::<IpAddr>().unwrap()
        );
        assert!(uut.dns_ip.is_some());
        assert_eq!(
            uut.dns_ip.unwrap(),
            "192.168.0.2".parse::<IpAddr>().unwrap()
        );
        assert!(uut.zfs_root.is_some());
        assert_eq!(uut.zfs_root.unwrap(), "zroot/pot".to_string());
        assert!(uut.fs_root.is_some());
        assert_eq!(uut.fs_root.unwrap(), "/opt/pot".to_string());
        assert!(uut.ext_if.is_some());
        assert_eq!(uut.ext_if.unwrap(), "em0".to_string());
        assert!(uut.dns_name.is_some());
        assert_eq!(uut.dns_name.unwrap(), "bar_dns".to_string());
    }

    #[test]
    fn partial_system_conf_fromstr_051() {
        let uut = PartialSystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=fdf1:186e:49e6:76d8::/64\nPOT_NETMASK=ffff:ffff:ffff:ffff::\nPOT_GATEWAY=fdf1:186e:49e6:76d8::1\n
            POT_DNS_IP=fdf1:186e:49e6:76d8::2\nPOT_DNS_NAME=bar_dns",
        );
        assert!(uut.is_ok());
        let uut = uut.unwrap();
        assert!(uut.is_valid());
        assert_ne!(uut, PartialSystemConf::default());
        assert!(uut.network.is_some());
        assert_eq!(
            uut.network.unwrap(),
            "fdf1:186e:49e6:76d8::/64".parse::<IpNet>().unwrap()
        );
        assert!(uut.netmask.is_some());
        assert_eq!(
            uut.netmask.unwrap(),
            "ffff:ffff:ffff:ffff::".parse::<IpAddr>().unwrap()
        );
        assert!(uut.gateway.is_some());
        assert_eq!(
            uut.gateway.unwrap(),
            "fdf1:186e:49e6:76d8::1".parse::<IpAddr>().unwrap()
        );
        assert!(uut.dns_ip.is_some());
        assert_eq!(
            uut.dns_ip.unwrap(),
            "fdf1:186e:49e6:76d8::2".parse::<IpAddr>().unwrap()
        );
        assert!(uut.zfs_root.is_some());
        assert_eq!(uut.zfs_root.unwrap(), "zroot/pot".to_string());
        assert!(uut.fs_root.is_some());
        assert_eq!(uut.fs_root.unwrap(), "/opt/pot".to_string());
        assert!(uut.ext_if.is_some());
        assert_eq!(uut.ext_if.unwrap(), "em0".to_string());
        assert!(uut.dns_name.is_some());
        assert_eq!(uut.dns_name.unwrap(), "bar_dns".to_string());
    }

    #[test]
    fn partial_system_conf_merge_001() {
        let mut uut = PartialSystemConf::default();
        let uut2 = PartialSystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        )
        .unwrap();
        uut.merge(uut2.clone());
        assert_eq!(uut, uut2);
    }

    #[test]
    fn partial_system_conf_merge_002() {
        let mut uut = PartialSystemConf::from_str(
            "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=bar_dns",
        )
        .unwrap();
        let uut2 = PartialSystemConf::from_str("POT_DNS_NAME=foo_dns").unwrap();
        uut.merge(uut2);
        assert_eq!(
            uut,
            PartialSystemConf::from_str(
                "POT_ZFS_ROOT=zroot/pot\nPOT_FS_ROOT=/opt/pot\nPOT_EXTIF=em0\n
            POT_NETWORK=192.168.0.0/24\nPOT_NETMASK=255.255.255.0\nPOT_GATEWAY=192.168.0.1\n
            POT_DNS_IP=192.168.0.2\nPOT_DNS_NAME=foo_dns"
            )
            .unwrap()
        );
    }
}
