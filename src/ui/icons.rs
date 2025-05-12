use crate::config::Program;
use image::EncodableLayout;
use std::fs::File;
use std::io;
use std::io::Read;
use tray_icon::Icon;

const ICON_ON: &[u8] = include_bytes!("../../resources/on.png");
const ICON_OFF: &[u8] = include_bytes!("../../resources/off.png");

#[derive(Clone)]
pub struct Icons {
    pub on: Icon,
    pub off: Icon,
}

pub fn load_icons(program: &Program) -> io::Result<Icons> {
    load_icons0(program.get_icon_on_path(), program.get_icon_off_path())
}

fn load_icons0(on_icon_path: Option<&str>, off_icon_path: Option<&str>) -> io::Result<Icons> {
    Ok(Icons {
        on: load_icon(on_icon_path, ICON_ON)?,
        off: load_icon(off_icon_path, ICON_OFF)?,
    })
}

fn load_icon(path: Option<&str>, default: &[u8]) -> io::Result<Icon> {
    let data = path.map_or(Ok(default.to_vec()), |p| load_binary(p))?;

    let img = image::load_from_memory(data.as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn load_binary(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn load_defaults() -> io::Result<()> {
        let _ = load_icons0(None, None)?;
        Ok(())
    }

    #[test]
    fn load_invalid_path() {
        let res = load_icons0(None, Some("invalid.png"));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn load_invalid_data() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();
        temp_file.as_file().write_all(br#"garbage"#)?;

        let res = load_icons0(None, Some(path));
        assert!(res.is_err());
        assert_eq!(res.err().unwrap().kind(), io::ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn load_icons() -> io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();
        temp_file.as_file().write_all(ICON_ON)?;

        let _ = load_icons0(Some(path), Some(path))?;
        Ok(())
    }

}