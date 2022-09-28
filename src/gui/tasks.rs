use super::Message;
use crate::mods::Mod;
use anyhow::{Context, Result};
use fs_err as fs;
use std::{io::BufReader, path::Path};
use uk_mod::unpack::ModReader;

fn is_probably_a_mod(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default();
    if ext != "zip" && ext != "7z" {
        false
    } else if ext == "7z" {
        true
    } else {
        match fs::File::open(path)
            .context("")
            .and_then(|f| zip::ZipArchive::new(BufReader::new(f)).context(""))
        {
            Ok(zip) => zip.file_names().any(|n| {
                [
                    "content",
                    "aoc",
                    "romfs",
                    "RomFS",
                    "atmosphere",
                    "contents",
                    "01007EF00011E000",
                    "01007EF00011F001",
                    "BreathOfTheWild",
                ]
                .into_iter()
                .any(|root| n.starts_with(root))
            }),
            Err(_) => false,
        }
    }
}

pub fn open_mod(path: &Path) -> Result<Message> {
    log::info!("Opening mod at {}", path.display());
    let mod_ = match ModReader::open(path, vec![]) {
        Ok(reader) => Mod::from_reader(reader),
        Err(err) => {
            log::warn!("Could not open mod, let's find out why");
            let err_msg = err.to_string();
            if (err_msg.contains("meta file") || err_msg.contains("invalid Zip"))
                && is_probably_a_mod(path)
            {
                log::info!("Maybe it's not a UKMM mod, let's to convert it");
                let converted_path = crate::mods::convert_gfx(path)?;
                Mod::from_reader(
                    ModReader::open(&converted_path, vec![])
                        .context("Failed to open converted mod")?,
                )
            } else {
                return Err(err.context("Failed to open mod"));
            }
        }
    };
    Ok(Message::HandleMod(mod_))
}
