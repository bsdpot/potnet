use failure::{format_err, Error};
use log::{debug, error, info, trace};
use potnet::pot::{get_pot_conf_list, NetType, SystemConf};
use std::collections::BTreeMap;
use std::net::IpAddr;
use std::process;
use std::string::String;
use structopt::StructOpt;
use structopt_flags::{HostParam, LogLevel};

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(flatten)]
    verbose: structopt_flags::QuietVerbose,
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
}

#[derive(Debug, StructOpt)]
struct ValidateOpt {
    #[structopt(flatten)]
    ip: HostParam,
}

#[derive(Debug, StructOpt)]
struct CheckOpt {
    #[structopt(flatten)]
    ip: HostParam,
}

fn show(verbose: bool, conf: &SystemConf, ip_db: &mut BTreeMap<IpAddr, Option<String>>) {
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
    if verbose {
        println!("\nDebug information\n{:#?}", conf);
    }
}

fn get(verbose: bool, conf: &SystemConf, ip_db: &BTreeMap<IpAddr, Option<String>>) {
    for addr in conf.network.unwrap().hosts() {
        if !ip_db.contains_key(&addr) {
            if verbose {
                println!("{} available", addr);
            } else {
                println!("{}", addr);
            }
            break;
        } else if verbose {
            println!("{} already used", addr);
        }
    }
}

fn validate(
    _verbose: bool,
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
        if v.network_type == NetType::PublicBridge {
            info!("Insert pot {:?}", v.ip_addr.unwrap());
            ip_db.insert(v.ip_addr.unwrap(), Some(v.name.clone()));
        }
    }
}

fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    opt.verbose.set_log_level();
    trace!("potnet start");

    let verbosity = if opt.verbose.get_level_filter() > log::LevelFilter::Warn {
        info!("Verbose output activated");
        true
    } else {
        false
    };
    let conf = SystemConf::new();
    if !conf.is_valid() {
        error!("No valid configuration found");
        println!("No valid configuration found");
        //dbg!(conf);
        return Ok(());
    }
    let mut ip_db = BTreeMap::new();
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
        Command::ConfigCheck => {
            if !conf.network.unwrap().contains(&conf.gateway.unwrap()) {
                error!(
                    "gateway IP ({}) outside the network range ({})",
                    conf.gateway.unwrap(),
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
                process::exit(1);
            }
        }
    }
    Ok(())
}
