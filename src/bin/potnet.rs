use failure::{format_err, Error};
use ipnet::IpNet;
use log::{debug, error, info, trace};
use potnet::pot::{get_bridges_list, get_pot_conf_list, BridgeConf, NetType, SystemConf};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::net::IpAddr::{V4, V6};
use std::string::String;
use structopt::StructOpt;
use structopt_flags::{HostParam, LogLevel};

#[derive(Clone, Debug, StructOpt)]
struct Opt {
    #[structopt(flatten)]
    verbose: structopt_flags::QuietVerbose,
    #[structopt(subcommand)]
    subcommand: Command,
}

#[derive(Clone, Debug, StructOpt)]
enum Command {
    /// Show the pot virtual network status
    #[structopt(name = "show")]
    Show(BridgeOpt),
    /// Provides the next available IP address
    #[structopt(name = "next")]
    Next(BridgeOpt),
    /// Check the POT config
    #[structopt(name = "config-check")]
    ConfigCheck,
    /// Validate the IP address provided as parameter
    #[structopt(name = "validate")]
    Validate(ValidateOpt),
    /// Check if the argument is a valid ipv4 address
    #[structopt(name = "ip4check")]
    IP4(CheckOpt),
    /// Check if the argument is a valid ipv6 address
    #[structopt(name = "ip6check")]
    IP6(CheckOpt),
    /// Check if the argument is a valid ip address
    #[structopt(name = "ipcheck")]
    IP(CheckOpt),
    /// Provide the next available network
    #[structopt(name = "new-net")]
    NewNetwork(NewNetOpt),
    /// Generate the etc/hosts file with all know hosts in the specific bridge
    #[structopt(name = "etc-hosts")]
    EtcHosts(BridgeOpt),
}

#[derive(Clone, Debug, StructOpt)]
struct BridgeOpt {
    /// The name of a private bridge
    #[structopt(short = "-b", long = "--bridge-name")]
    bridge_name: Option<String>,
}

#[derive(Clone, Debug, StructOpt)]
struct ValidateOpt {
    #[structopt(flatten)]
    ip: HostParam,
    /// The name of the private bridge, if the IP belongs to it
    #[structopt(short = "-b", long = "--bridge-name")]
    bridge_name: Option<String>,
}

#[derive(Clone, Debug, StructOpt)]
struct CheckOpt {
    #[structopt(flatten)]
    ip: HostParam,
}

#[derive(Clone, Debug, StructOpt)]
struct NewNetOpt {
    /// The number of host to be included in the network (gateway excluded)
    #[structopt(short = "-s")]
    host_number: u16,
}

fn show(opt: &Opt, conf: &SystemConf, ip_db: &mut BTreeMap<IpAddr, Option<String>>) {
    println!("Network topology:");
    println!("\tnetwork : {}", conf.network.unwrap().trunc());
    println!("\tmin addr: {}", conf.network.unwrap().network());
    println!("\tmax addr: {}", conf.network.unwrap().broadcast());
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
    if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
        println!("\nDebug information\n{:#?}", conf);
    }
}

fn show_bridge(_opt: &Opt, conf: &SystemConf, bridge_name: &str) {
    let bridges_list = get_bridges_list(conf);
    if let Some(bridge) = bridges_list.iter().find(|x| x.name == bridge_name) {
        info!("bridge {} found", bridge.name);
        let mut ip_db = BTreeMap::new();
        init_bridge_ipdb(&bridge, conf, &mut ip_db);
        for (ip, opt_name) in ip_db.iter() {
            println!(
                "\t{}\t{}",
                ip,
                match opt_name {
                    Some(s) => s,
                    None => "",
                }
            );
        }
    } else {
        error!("bridge {} not found", bridge_name);
    }
}

fn get(opt: &Opt, conf: &SystemConf, ip_db: &BTreeMap<IpAddr, Option<String>>) {
    for addr in conf.network.unwrap().hosts() {
        if !ip_db.contains_key(&addr) {
            if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
                println!("{} available", addr);
            } else {
                println!("{}", addr);
            }
            break;
        } else if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
            println!("{} already used", addr);
        }
    }
}

fn get_network_size(host_number: u16) -> Option<u8> {
    if host_number == 0 {
        return None;
    }
    let mut max_hosts = 4u16;
    let mut result = 2;
    loop {
        if host_number <= max_hosts - 2 {
            break;
        }
        max_hosts <<= 1;
        result += 1;
    }
    Some(result)
}

fn get_prefix_length(host_number: u16, ip_addr: &IpAddr) -> Option<u8> {
    if let Some(network_size) = get_network_size(host_number) {
        Some(
            match ip_addr {
                V4(_) => 32,
                V6(_) => 128,
            } - network_size,
        )
    } else {
        None
    }
}

fn is_subnet_usable(subnet: IpNet, ip_db: &BTreeMap<IpAddr, Option<String>>) -> bool {
    for ip in ip_db.keys() {
        if subnet.contains(ip) {
            return false;
        }
    }
    true
}

fn new_net(host_number: u16, conf: &SystemConf, ip_db: &BTreeMap<IpAddr, Option<String>>) {
    if let Some(prefix_length) = get_prefix_length(host_number, &conf.gateway.unwrap()) {
        info!("Subnet prefix length {}", prefix_length);
        if let Ok(subnets) = conf.network.unwrap().subnets(prefix_length) {
            //info!("{} subnets to evaluate", subnets.count());
            for s in subnets {
                if is_subnet_usable(s, ip_db) {
                    println!("net={}", s);
                    println!("gateway={}", s.hosts().nth(0).unwrap());
                    break;
                } else {
                    debug!("{} not usable", s);
                }
            }
        }
    }
}

fn get_next_from_bridge(opt: &Opt, conf: &SystemConf, bridge_name: &str) {
    let bridges_list = get_bridges_list(conf);
    if let Some(bridge) = bridges_list.iter().find(|x| x.name == bridge_name) {
        info!("bridge {} found", bridge.name);
        let mut ip_db = BTreeMap::new();
        init_bridge_ipdb(&bridge, conf, &mut ip_db);
        for addr in bridge.network.hosts() {
            if !ip_db.contains_key(&addr) {
                if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
                    println!("{} available", addr);
                } else {
                    println!("{}", addr);
                }
                break;
            }
        }
    } else {
        error!("bridge {} not found", bridge_name);
    }
}

fn get_hosts_from_bridge(_opt: &Opt, conf: &SystemConf, bridge_name: &str) {
    let bridges_list = get_bridges_list(conf);
    if let Some(bridge) = bridges_list.iter().find(|x| x.name == bridge_name) {
        info!("bridge {} found", bridge.name);
        let mut ip_db = BTreeMap::new();
        info!("Evaluating bridge {:?}", bridge);
        for v in &get_pot_conf_list(conf.clone()) {
            if v.network_type == NetType::PrivateBridge
                && bridge.network.contains(&v.ip_addr.unwrap())
            {
                ip_db.insert(v.ip_addr.unwrap(), v.name.clone());
            }
        }
        for (ip, hostname) in ip_db {
            println!("{} {}", ip, hostname);
        }
    }
}

fn get_hosts_for_public_bridge(_opt: &Opt, conf: &SystemConf) {
    let mut ip_db = BTreeMap::new();
    for v in &get_pot_conf_list(conf.clone()) {
        if v.network_type == NetType::PublicBridge {
            ip_db.insert(v.ip_addr.unwrap(), v.name.clone());
        }
    }
    for (ip, hostname) in ip_db {
        println!("{} {}", ip, hostname);
    }
}

fn validate_with_bridge(conf: &SystemConf, bridge_name: &str, ip: IpAddr) -> Result<(), Error> {
    let bridges_list = get_bridges_list(conf);
    if let Some(bridge) = bridges_list.iter().find(|x| x.name == bridge_name) {
        info!("bridge {} found", bridge.name);
        let mut ip_db = BTreeMap::new();
        init_bridge_ipdb(&bridge, conf, &mut ip_db);
        // the ip address is in the bridge network
        if !bridge.network.contains(&ip) {
            error!("ip {} not in the bridge network {}", ip, bridge.network);
            return Err(format_err!("Ip outside the bridge network"));
        }
        // the ip is already in use
        if ip_db.contains_key(&ip) {
            error!("ip {} already in use", ip);
            return Err(format_err!("Ip already used"));
        }
    } else {
        return Err(format_err!("bridge {} not found", bridge_name));
    }
    Ok(())
}

fn validate(
    ip: IpAddr,
    conf: &SystemConf,
    ip_db: &BTreeMap<IpAddr, Option<String>>,
) -> Result<(), Error> {
    if ip_db.contains_key(&ip) {
        return Err(format_err!("Address already in use"));
    }
    if !conf.network.unwrap().contains(&ip) {
        return Err(format_err!("Address outside the network"));
    }
    Ok(())
}

fn init_bridge_ipdb(
    bridge: &BridgeConf,
    conf: &SystemConf,
    ip_db: &mut BTreeMap<IpAddr, Option<String>>,
) {
    info!("Evaluating bridge {:?}", bridge);
    // add the network address
    let mut description = String::from(bridge.name.as_str());
    description.push_str(" bridge - network ");
    ip_db.insert(bridge.network.network(), Some(description));
    // add the broadcast address
    let mut description = String::from(bridge.name.as_str());
    description.push_str(" bridge - broadcast ");
    ip_db.insert(bridge.network.broadcast(), Some(description));
    // add the broadcast address
    let mut description = String::from(bridge.name.as_str());
    description.push_str(" bridge - gateway ");
    ip_db.insert(bridge.gateway, Some(description));
    for v in &get_pot_conf_list(conf.clone()) {
        if (v.network_type == NetType::PublicBridge || v.network_type == NetType::PrivateBridge)
            && bridge.network.contains(&v.ip_addr.unwrap())
        {
            ip_db.insert(v.ip_addr.unwrap(), Some(v.name.clone()));
        }
    }
}

fn init_ipdb(conf: &SystemConf, ip_db: &mut BTreeMap<IpAddr, Option<String>>) {
    info!("Insert network {:?}", conf.network);
    ip_db.insert(conf.network.unwrap().network(), None);
    info!("Insert broadcast {:?}", conf.network);
    ip_db.insert(conf.network.unwrap().broadcast(), None);
    info!("Insert gateway {:?}", conf.gateway);
    ip_db.insert(conf.gateway.unwrap(), Some("default gateway".to_string()));
    info!("Insert dns {:?}", conf.dns_ip);
    ip_db.insert(
        conf.dns_ip.unwrap(),
        Some(conf.dns_name.as_ref().unwrap().to_string()),
    );
    for v in &get_pot_conf_list(conf.clone()) {
        if v.network_type == NetType::PublicBridge || v.network_type == NetType::PrivateBridge {
            info!("Insert pot {:?}", v.ip_addr.unwrap());
            ip_db.insert(v.ip_addr.unwrap(), Some(v.name.clone()));
        }
    }
    for b in &get_bridges_list(conf) {
        info!("Evaluating bridge {:?}", b);
        // add the network address
        let mut description = String::from(b.name.as_str());
        description.push_str(" bridge - network ");
        ip_db.insert(b.network.network(), Some(description));
        // add the broadcast address
        let mut description = String::from(b.name.as_str());
        description.push_str(" bridge - broadcast ");
        ip_db.insert(b.network.broadcast(), Some(description));
        // add the broadcast address
        let mut description = String::from(b.name.as_str());
        description.push_str(" bridge - gateway ");
        ip_db.insert(b.gateway, Some(description));
        // add all the not yet allocated hosts
        let mut description = String::from(b.name.as_str());
        description.push_str(" bridge - allocated address");
        for host in b.network.hosts() {
            ip_db
                .entry(host)
                .or_insert_with(|| Some(description.clone()));
        }
    }
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    opt.verbose.set_log_level();
    trace!("potnet start");

    let conf = SystemConf::new();
    if !conf.is_valid() {
        error!("No valid configuration found");
        println!("No valid configuration found");
        return Ok(());
    }
    let mut ip_db = BTreeMap::new();
    init_ipdb(&conf, &mut ip_db);
    let opt_clone = opt.clone();
    match opt.subcommand {
        Command::Show(bopt) => {
            if let Some(bridge_name) = bopt.bridge_name {
                show_bridge(&opt_clone, &conf, &bridge_name);
            } else {
                show(&opt_clone, &conf, &mut ip_db);
            }
        }
        Command::Next(nopt) => {
            if let Some(bridge_name) = nopt.bridge_name {
                debug!("get an ip for the bridge {}", bridge_name);
                get_next_from_bridge(&opt_clone, &conf, &bridge_name);
            } else {
                get(&opt_clone, &conf, &ip_db);
            }
        }
        Command::Validate(vopt) => {
            if let Some(bridge_name) = vopt.bridge_name {
                debug!(
                    "validate the ip {} for the bridge {}",
                    &vopt.ip.host_addr, bridge_name
                );
                return validate_with_bridge(&conf, &bridge_name, vopt.ip.host_addr);
            } else {
                return validate(vopt.ip.host_addr, &conf, &ip_db);
            }
        }
        Command::IP4(x) => {
            if !x.ip.host_addr.is_ipv4() {
                std::process::exit(1);
            }
        }
        Command::IP6(x) => {
            if !x.ip.host_addr.is_ipv6() {
                std::process::exit(1);
            }
        }
        Command::IP(x) => {
            debug!("{} is a valid IP address", x.ip.host_addr);
        }
        Command::ConfigCheck => {
            if !conf.network.unwrap().contains(&conf.gateway.unwrap()) {
                error!(
                    "gateway IP ({}) outside the network range ({})",
                    conf.gateway.unwrap(),
                    conf.network.unwrap()
                );
            }
            if !conf.network.unwrap().contains(&conf.dns_ip.unwrap()) {
                error!(
                    "DNS IP ({}) outside the network range ({})",
                    conf.dns_ip.unwrap(),
                    conf.network.unwrap()
                );
            }
            if conf.network.unwrap().netmask() != conf.netmask.unwrap() {
                error!(
                    "netmask ({}) different from the network one ({})",
                    conf.netmask.unwrap(),
                    conf.network.unwrap()
                );
            }
            if !conf.network.unwrap().contains(&conf.gateway.unwrap())
                || conf.network.unwrap().netmask() != conf.netmask.unwrap()
            {
                std::process::exit(1);
            }
        }
        Command::NewNetwork(x) => {
            if x.host_number <= 1 {
                error!("A network with size {} is too small", x.host_number);
                std::process::exit(1);
            }
            new_net(x.host_number, &conf, &ip_db);
        }
        Command::EtcHosts(ehopt) => {
            if let Some(bridge_name) = ehopt.bridge_name {
                debug!("get an ip for the bridge {}", bridge_name);
                get_hosts_from_bridge(&opt_clone, &conf, &bridge_name);
            } else {
                get_hosts_for_public_bridge(&opt_clone, &conf);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn get_network_size_000() {
        let uut = get_network_size(2);
        assert_eq!(uut, Some(2));
    }
    #[test]
    fn get_network_size_001() {
        let uut = get_network_size(5);
        assert_eq!(uut, Some(3));
    }
    #[test]
    fn get_network_size_002() {
        let uut = get_network_size(7);
        assert_eq!(uut, Some(4));
    }

    #[test]
    fn get_prefix_length_000() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let uut = get_prefix_length(2, &ip_addr);
        assert_eq!(uut, Some(30));
    }
    #[test]
    fn get_prefix_length_001() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let uut = get_prefix_length(5, &ip_addr);
        assert_eq!(uut, Some(29));
    }
    #[test]
    fn get_prefix_length_002() {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let uut = get_prefix_length(9, &ip_addr);
        assert_eq!(uut, Some(28));
    }
    #[test]
    fn get_prefix_length_010() {
        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        let uut = get_prefix_length(2, &ip_addr);
        assert_eq!(uut, Some(126));
    }
    #[test]
    fn get_prefix_length_011() {
        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        let uut = get_prefix_length(5, &ip_addr);
        assert_eq!(uut, Some(125));
    }
    #[test]
    fn get_prefix_length_012() {
        let ip_addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1));
        let uut = get_prefix_length(9, &ip_addr);
        assert_eq!(uut, Some(124));
    }
}
