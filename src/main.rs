#[macro_use]
extern crate structopt;
extern crate potnet;

use potnet::pot::{get_pot_conf_list, IPType, SystemConf};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short = "v")]
    verbose: bool,
    #[structopt(subcommand)]
    subcommand: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "show")]
    Show,
    #[structopt(name = "next")]
    Next,
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

fn octect_incr(a: &mut [u8; 4]) {
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
                println!("{},{}.{}.{} available", addr[0], addr[1], addr[2], addr[3]);
            } else {
                println!("{},{}.{}.{}", addr[0], addr[1], addr[2], addr[3]);
            }
            break;
        } else if verbose {
            println!(
                "{},{}.{}.{} already used",
                addr[0], addr[1], addr[2], addr[3]
            );
        }
    }
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
            ip_db.insert(v.ip_addr.unwrap(), Some(v.name.clone()));
        }
    }
}

fn main() {
    let opt = Opt::from_args();

    let mut verbosity = false;
    if opt.verbose {
        verbosity = true;
    }
    let conf = SystemConf::new();
    if !conf.is_valid() {
        println!("No valid configuration found");
        return;
    }
    let mut ip_db = BTreeMap::new();
    ip_db.insert(conf.network.unwrap(), None);
    ip_db.insert(
        conf.dns_ip.unwrap(),
        Some(conf.dns_name.as_ref().unwrap().to_string()),
    );
    ip_db.insert(conf.gateway.unwrap(), Some("GATEWAY".to_string()));
    init_ipdb(&conf, &mut ip_db);
    match opt.subcommand {
        Command::Show => {
            show(verbosity, &conf, &mut ip_db);
        }
        Command::Next => {
            get(verbosity, &conf, &ip_db);
        }
    }
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
