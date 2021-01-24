use crate::pot::error::PotError;
use crate::pot::Result;
use std::path::PathBuf;
use std::process::Command;

// get pot prefix in the same way as pot does:
// find PREFIX/bin/pot and get the PREFIX
fn get_pot_prefix() -> Result<PathBuf> {
    let pathname = Command::new("which")
        .arg("pot")
        .output()
        .map_err(|_| PotError::WhichError("pot".to_string()))?;
    let pot_path = PathBuf::from(String::from_utf8(pathname.stdout)?);
    let pot_prefix = pot_path
        .parent()
        .ok_or(PotError::PathError(format!("{}", pot_path.display())))?;
    let pot_prefix = pot_prefix
        .parent()
        .ok_or(PotError::PathError(format!("{}", pot_prefix.display())))?;
    Ok(pot_prefix.to_path_buf())
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
