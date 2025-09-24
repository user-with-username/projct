use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct FileUtils;

impl FileUtils {
    pub fn is_text_file(filepath: &Path) -> bool {
        let mut file = match File::open(filepath) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut chunk = vec![0u8; 1024];
        let len = match file.read(&mut chunk) {
            Ok(l) => l,
            Err(_) => return false,
        };
        if chunk[..len].contains(&0) {
            return false;
        }
        drop(file);
        let mut file = match File::open(filepath) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut content = String::new();
        file.read_to_string(&mut content).is_ok()
    }
}
