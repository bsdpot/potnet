# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [0.5.0] 2023-12-30
### Changed
- Adopt anyhow and thiserror instead of failure
- Start a modularization work to build a pot crate
- Make DNS configuration optional (#51)

## [0.4.4] 2020-03-31
### Fixed
- pot::get_pot_conf_list(): ignore alias network type, not useful for the analysis

## [0.4.3] 2020-03-08
### Fixed
- potcpu: help messages were victims of copypasta (#1)

## [0.4.2] 2019-10-25
### Added
- show: add support for bridges

### Fixed
- potcpu: application name in structopt messages

## [0.4.1] 2019-10-23
### Added
- etc-hosts: add a subcomand to generate /etc/hosts entries for the public bridge or a private one

## [0.4.0] 2019-10-11
### Added
- new-net: add a command to get a subnet of some specified size
- next: add support for bridges
- validate: add support for bridges

## [0.3.2] 2019-08-14
### Fixed
- structopt-flags: update crates to integrate a fix

## [0.3.1] 2019-08-13
### Added
- potnet config-check: check the dns IP as well

### Fixed
- potcpu: rebalance triggered when not needed

## [0.3.0] 2019-08-02
### Added
- New utility, called potcpu, designed to manage cpusets for pot
- potcpu: rebalance command, to distribute allocation all over the CPUs

## [0.2.0] 2019-07-10
### Added
- Add support for the new network configuration format introduced in pot 0.8
- config-check : new command to validate the network configuration

### Changed
- Adopted the crate IpNet to perform operations on IP addresses
- Add IPv6 support everywhere

## [0.1.3] 2019-04-03
### Added
- Add a cli method to identify if an argument is a valid ip[46] address
- Add LTO step for release build
- Add the bsd-ci file

## [0.1.2] 2018-11-03
### Added
- adopt failure crate
- validate: new subcommand to validate the usability of an address

### Changed
- update creates and adopt structopt-flags
- Use the FreeBSD email address

## [0.1.1] 2018-07-18
### Changed
- next: renamed get subcommand with next
- adopt structopt

## [0.1.0] 2018-05-21
### Added
- show: it shows the whole network status
- get: gets the next available IP address
