use std::path::Path;

use crate::error::{DistillError, Result};

pub fn export_markdown(content: &str, output_path: Option<&Path>) -> Result<()> {
    match output_path {
        Some(path) => {
            std::fs::write(path, content)
                .map_err(|e| DistillError::Export { cause: e.to_string() })?;
        }
        None => {
            print!("{content}");
        }
    }
    Ok(())
}
