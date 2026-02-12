//! Extract unit blueprint files from FAF gamedata (folder or .scd/.zip).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_EXTRACT_FILES: usize = 10_000;
const UNIT_BP_SUFFIX: &str = "_unit.bp";

/// Discover and extract all *_unit.bp files from gamedata path (directory or .scd/.zip) into out_dir.
/// Returns the number of blueprint files written.
pub fn extract_gamedata(gamedata_path: &Path, out_dir: &Path) -> Result<usize, String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let gamedata_path = gamedata_path.canonicalize().map_err(|e| e.to_string())?;

    let mut count = 0usize;
    if gamedata_path.is_file() {
        let ext = gamedata_path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();
        if ext == "scd" || ext == "zip" {
            extract_from_zip(&gamedata_path, out_dir, &mut count)?;
            return Ok(count);
        }
        return Err("gamedata path is a file but not .scd or .zip".to_string());
    }

    // Directory: look for gamedata/units or units at root
    let units_root = gamedata_path.join("units");
    if units_root.is_dir() {
        collect_and_copy_bps(&units_root, &units_root, out_dir, &mut count)?;
        return Ok(count);
    }
    // Maybe we were given FAF root: try gamedata/ inside it
    let gamedata_sub = gamedata_path.join("gamedata");
    if gamedata_sub.is_dir() {
        let u = gamedata_sub.join("units");
        if u.is_dir() {
            collect_and_copy_bps(&u, &u, out_dir, &mut count)?;
            return Ok(count);
        }
    }
    // Or maybe direct path to units folder
    if gamedata_path.file_name().map(|n| n == "units").unwrap_or(false) {
        collect_and_copy_bps(&gamedata_path, &gamedata_path, out_dir, &mut count)?;
        return Ok(count);
    }
    Err(format!(
        "no units folder found under {} (expected gamedata/units, or path to units, or .scd/.zip)",
        gamedata_path.display()
    ))
}

fn extract_from_zip(zip_path: &Path, out_dir: &Path, count: &mut usize) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        if *count >= MAX_EXTRACT_FILES {
            break;
        }
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if !name.ends_with(UNIT_BP_SUFFIX) {
            continue;
        }
        if name.contains("__MACOSX") || name.contains('\\') {
            continue;
        }
        let filename = Path::new(&name)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unit.bp");
        let out_path = out_dir.join(filename);
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        *count += 1;
    }
    Ok(())
}

fn collect_and_copy_bps(
    root: &Path,
    dir: &Path,
    out_dir: &Path,
    count: &mut usize,
) -> Result<(), String> {
    if *count >= MAX_EXTRACT_FILES {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for e in entries {
        let e = e.map_err(|e| e.to_string())?;
        let path = e.path();
        if path.is_dir() {
            collect_and_copy_bps(root, &path, out_dir, count)?;
        } else if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(UNIT_BP_SUFFIX))
            .unwrap_or(false)
        {
            let name = path.file_name().unwrap();
            let out_path = out_dir.join(name);
            fs::copy(&path, &out_path).map_err(|e| e.to_string())?;
            *count += 1;
        }
    }
    Ok(())
}

/// Resolve gamedata path: if path is a directory, check for gamedata.scd or gamedata/ inside it.
pub fn resolve_gamedata_path(path: &Path) -> PathBuf {
    if path.is_file() {
        return path.to_path_buf();
    }
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let scd = p.join("gamedata.scd");
    if scd.is_file() {
        return scd;
    }
    let sub = p.join("gamedata");
    if sub.is_dir() {
        return sub;
    }
    p
}
