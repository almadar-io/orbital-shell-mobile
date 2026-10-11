//! `orb-shell-mobile`: the React Native mobile shell as a program (`orbital_core::shell_protocol`).

use orbital_core::project::write_project;
use orbital_core::shell_protocol::{
    description, written_files, GenerateRequest, GenerateResponse, ShellDescription, ShellFailure, ShellProgram,
};
use std::path::Path;

pub struct MobileProgram;

impl ShellProgram for MobileProgram {
    fn describe(&self) -> ShellDescription {
        description("mobile", env!("CARGO_PKG_VERSION"), false, Vec::new())
    }

    fn generate(&self, request: &GenerateRequest, output: &Path) -> Result<GenerateResponse, ShellFailure> {
        let generated = write_project(&request.module, output, &crate::MobileShell::new(), &crate::MobileBackend::new(), &request.write)?;
        Ok(GenerateResponse {
            files: written_files(&generated),
            warnings: generated.warnings,
            next_steps: vec![format!("cd {}", output.display()), "npm install".to_string(), "npx expo start".to_string()],
        })
    }
}
