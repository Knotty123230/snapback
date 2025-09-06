use std::{fs::File, io::Read};

use sha2::{Digest, Sha256};

pub(crate) fn calculate_file_hash(path: &std::path::Path) -> anyhow::Result<String> {
      let mut file = File::open(path)?;
      let mut hasher = Sha256::new();
      let mut buffer = [0; 8192];

      loop {
          let bytes_read = file.read(&mut buffer)?;
          if bytes_read == 0 {
              break;
          }
          hasher.update(&buffer[..bytes_read]);
      }

      Ok(format!("{:x}", hasher.finalize()))
  }