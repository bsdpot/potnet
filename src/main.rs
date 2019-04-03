use failure::{format_err, Error};
use log::{debug, error, info, trace};
use potnet::pot::{get_pot_conf_list, IPType, SystemConf};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::process;
use std::string::String;
use structopt::StructOpt;
use structopt_flags::{HostParam, HostV4Param};

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(flatten)]
    verbose: structopt_flags::SimpleVerbose,
    #[structopt(subcommand)]
    subcommand: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Show the pot virtual network status
    #[structopt(name = "show")]
    Show,
    /// Provides the next available IP address
    #[structopt(name = "next")]
    Next,
    /// Validate the IP address provided as parameter
    #[structopt(name = "validate")]
    Validate(ValidateOpt),
    /// Check if the arguemnt is a valid ipv4 address
    #[structopt(name = "ip4check")]
    IP4(CheckOpt),
    /// Check if the arguemnt is a valid ipv6 address
    #[structopt(name = "ip6check")]
    IP6(CheckOpt),
    /// Check if the arguemnt is a valid ip address
    #[structopt(name = "ipcheck")]
    IP(CheckOpt),
}

#[derive(Debug, StructOpt)]
struct ValidateOpt {
    #[structopt(flatten)]
    ip: HostV4Param,
}

#[derive(Debug, StructOpt)]
struct CheckOpt {
    #[structopt(flatten)]
    ip: HostParam,
}

fn show(verbose: bool, conf: &SystemConf, ip_db: &mut BTreeMap<Ipv4Addr, Option<String>>) {
    let netmask = conf.netmask.unwrap().octets();
    let net_min = conf.network.unwrap().octets();
    let mut net_max = net_min;
    //    net_max |= netmask;
    net_max[3] |= !netmask[3];
    net_max[2] |= !netmask[2];
    net_max[1] |= !netmask[1];
    net_max[0] |= !netmask[0];
    let max_addr = Ipv4Addr::from(net_max);
    println!("Network topology:");
    println!("\tnetwork : {}", conf.network.unwrap());
    println!("\tmin addr: {}", conf.network.unwrap());
    println!("\tmax addr: {:?}", max_addr);
    println!("\nAddresses already taken:");
    for (ip, opt_name) in ip_db.iter() {
        println!(
            "\t{}\t{}",
            ip,
            match *opt_name {
                Some(ref s) => s,
                None => "",
            }
        );
    }
    if verbose {
        println!("\nDebug information\n{:?}", conf);
    }
}

/// Increment an IPv4 address, give as a slice of 4 octets
///
/// # Examples
///
/// ```
/// let mut a = [192u8, 168u8, 1u8, 255];
/// octect_incr(&mut a);
/// assert_eq!(a, [192u8, 168u8, 2u8, 0u8]);
/// ```
pub fn octect_incr(a: &mut [u8; 4]) {
    for idx in (0..4).rev() {
        if a[idx] == 255 {
            a[idx] = 0;
        } else {
            a[idx] += 1;
            break;
        }
    }
}

fn get(verbose: bool, conf: &SystemConf, ip_db: &BTreeMap<Ipv4Addr, Option<String>>) {
    let netmask = conf.netmask.unwrap().octets();
    let net_min = conf.network.unwrap().octets();
    let mut net_max = net_min;
    net_max[3] |= !netmask[3];
    net_max[2] |= !netmask[2];
    net_max[1] |= !netmask[1];
    net_max[0] |= !netmask[0];
    let mut addr: [u8; 4] = net_min;
    loop {
        octect_incr(&mut addr);
        if !ip_db.contains_key(&(Ipv4Addr::from(addr))) {
            if verbose {
                println!("{}.{}.{}.{} available", addr[0], addr[1], addr[2], addr[3]);
            } else {
                println!("{}.{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
            }
            break;
        } else if verbose {
            println!(
                "{}.{}.{}.{} already used",
                addr[0], addr[1], addr[2], addr[3]
            );
        }
    }
}

fn validate(
    _verbose: bool,
    ip: Ipv4Addr,
    conf: &SystemConf,
    ip_db: &BTreeMap<Ipv4Addr, Option<String>>,
) -> Result<(), Error> {
    if ip_db.contains_key(&ip) {
        return Err(format_err!("Address already in use"));
    }

    if ip < conf.network.unwrap() {
        return Err(format_err!("Address outside the network"));
    }
    let netmask = conf.netmask.unwrap().octets();
    let net_min = conf.network.unwrap().octets();
    let mut net_max = net_min;
    net_max[3] |= !netmask[3];
    net_max[2] |= !netmask[2];
    net_max[1] |= !netmask[1];
    net_max[0] |= !netmask[0];
    let max_addr = Ipv4Addr::from(net_max);
    if ip > max_addr {
        return Err(format_err!("Address outside the network"));
    }
    Ok(())
}

fn init_ipdb(conf: &SystemConf, ip_db: &mut BTreeMap<Ipv4Addr, Option<String>>) {
    let netmask = conf.netmask.unwrap().octets();
    let net_min = conf.network.unwrap().octets();
    let mut net_max = net_min;
    net_max[3] |= !netmask[3];
    net_max[2] |= !netmask[2];
    net_max[1] |= !netmask[1];
    net_max[0] |= !netmask[0];
    let max_addr = Ipv4Addr::from(net_max);
    ip_db.insert(max_addr, None);
    for v in &get_pot_conf_list(conf.clone()) {
        if v.ip_type == IPType::Vnet {
            info!("Insert dns {:?}", v.ip_addr);
            ip_db.insert(v.ip_addr.unwrap(), Some(v.name.clone()));
        }
    }
}

fn main() -> Result<(), Error> {
    env_logger::try_init()?;
    trace!("potnet start");

    let opt = Opt::from_args();

    let verbosity = if opt.verbose.verbose {
        info!("Verbose output activated");
        true
    } else {
        false
    };
    let conf = SystemConf::new();
    if !conf.is_valid() {
        error!("No valid configuration found");
        println!("No valid configuration found");
        return Ok(());
    }
    let mut ip_db = BTreeMap::new();
    info!("Insert network {:?}", conf.network);
    ip_db.insert(conf.network.unwrap(), None);
    info!("Insert dns {:?}", conf.dns_ip);
    ip_db.insert(
        conf.dns_ip.unwrap(),
        Some(conf.dns_name.as_ref().unwrap().to_string()),
    );
    info!("Insert gateway {:?}", conf.gateway);
    ip_db.insert(conf.gateway.unwrap(), Some("default gateway".to_string()));
    init_ipdb(&conf, &mut ip_db);
    match opt.subcommand {
        Command::Show => {
            show(verbosity, &conf, &mut ip_db);
        }
        Command::Next => {
            get(verbosity, &conf, &ip_db);
        }
        Command::Validate(_vopt) => {
            return validate(verbosity, _vopt.ip.host_addr, &conf, &ip_db);
        }
        Command::IP4(_x) => {
            if !_x.ip.host_addr.is_ipv4() {
                process::exit(1);
            }
        }
        Command::IP6(_x) => {
            if !_x.ip.host_addr.is_ipv6() {
                process::exit(1);
            }
        }
        Command::IP(_x) => {
            debug!("{} is a valid IP address", _x.ip.host_addr);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octect_intr_001() {
        let mut a = [0u8, 0u8, 0u8, 0u8];
        octect_incr(&mut a);
        assert_eq!(a, [0u8, 0u8, 0u8, 1u8]);
    }

    #[test]
    fn octect_intr_002() {
        let mut a = [0u8, 0u8, 0u8, 255];
        octect_incr(&mut a);
        assert_eq!(a, [0u8, 0u8, 1u8, 0u8]);
    }

    #[test]
    fn octect_intr_003() {
        let mut a = [0u8, 0u8, 255u8, 255u8];
        octect_incr(&mut a);
        assert_eq!(a, [0u8, 1u8, 0u8, 0u8]);
    }

    #[test]
    fn octect_intr_004() {
        let mut a = [0u8, 255u8, 255u8, 255u8];
        octect_incr(&mut a);
        assert_eq!(a, [1u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn octect_intr_005() {
        let mut a = [255u8, 255u8, 255u8, 255u8];
        octect_incr(&mut a);
        assert_eq!(a, [0u8, 0u8, 0u8, 0u8]);
    }

    #[test]
    fn octect_intr_006() {
        let mut a = [0u8, 10u8, 255u8, 255u8];
        octect_incr(&mut a);
        assert_eq!(a, [0u8, 11u8, 0u8, 0u8]);
    }
}
