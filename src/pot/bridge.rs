use crate::pot::error::PotError;
use crate::pot::Result;
use ipnet::IpNet;
use std::convert::TryFrom;
use std::net::IpAddr;
use std::str::FromStr;

use crate::pot::PotSystemConfig;
use std::path::PathBuf;

pub fn get_bridges_list(conf: &PotSystemConfig) -> Result<Vec<BridgeConf>> {
    let path_list = get_bridges_path_list(conf);
    let mut result = Vec::new();
    for f in path_list {
        if let Ok(conf_str) = std::fs::read_to_string(f.as_path()) {
            if let Ok(bridge_conf) = conf_str.parse() {
                result.push(bridge_conf);
            }
        }
    }
    Ok(result)
}

fn get_bridges_path_list(conf: &PotSystemConfig) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let bridges_path = std::path::Path::new(&conf.fs_root).join("bridges");
    walkdir::WalkDir::new(bridges_path)
        .max_depth(1)
        .min_depth(1)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|x| x.file_type().is_file())
        .for_each(|x| result.push(x.into_path()));
    result
}
#[derive(Debug)]
pub struct BridgeConf {
    pub name: String,
    pub network: IpNet,
    pub gateway: IpAddr,
}

impl FromStr for BridgeConf {
    type Err = crate::pot::error::PotError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let partial = PartialBridgeConf::from_str(s).unwrap();
        BridgeConf::try_from(partial)
    }
}

impl TryFrom<PartialBridgeConf> for BridgeConf {
    type Error = PotError;

    fn try_from(value: PartialBridgeConf) -> std::result::Result<Self, Self::Error> {
        if !value.is_valid() {
            Err(PotError::BridgeConfError)
        } else {
            let network = value.network.unwrap();
            let gateway = value.gateway.unwrap();
            if !network.contains(&gateway) {
                Err(PotError::BridgeConfError)
            } else {
                Ok(BridgeConf {
                    name: value.name.unwrap(),
                    network: value.network.unwrap(),
                    gateway: value.gateway.unwrap(),
                })
            }
        }
    }
}

#[derive(Default, Debug)]
struct PartialBridgeConf {
    name: Option<String>,
    network: Option<IpNet>,
    gateway: Option<IpAddr>,
}

fn get_value<T>(line: &str, key: &str) -> Option<T>
where
    T: FromStr,
{
    if line.starts_with(key) {
        match line.split('=').nth(1) {
            Some(value) => match value.split(' ').next() {
                Some(value) => value.parse().ok(),
                None => None,
            },
            None => None,
        }
    } else {
        None
    }
}

impl FromStr for PartialBridgeConf {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let lines: Vec<String> = s
            .to_string()
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.starts_with('#'))
            .collect();
        let mut result = PartialBridgeConf::default();
        for linestr in &lines {
            if linestr.starts_with("name=") {
                result.name = get_value(linestr, "name=");
            }
            if linestr.starts_with("net=") {
                result.network = get_value(linestr, "net=");
            }
            if linestr.starts_with("gateway=") {
                result.gateway = get_value(linestr, "gateway=");
            }
        }
        Ok(result)
    }
}

impl PartialBridgeConf {
    fn is_valid(&self) -> bool {
        self.name.is_some() && self.network.is_some() && self.gateway.is_some()
    }
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

    #[test]
    fn bridge_conf_fromstr_020() {
        let uut = BridgeConf::from_str("net=10.192.0.24/29\ngateway=10.192.0.25\nname=test-bridge");
        assert_eq!(uut.is_ok(), true);
    }
}
