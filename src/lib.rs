use std::fs;
use std::process::Command as SystemCommand;
use zed_extension_api::{
    self as zed, settings::LspSettings, Architecture, Command, GithubReleaseOptions,
    LanguageServerId, LanguageServerInstallationStatus, Os, Result, Worktree,
};


struct SqlToolExtension {
    cached_executable_path: Option<String>,
}

impl SqlToolExtension {
  
    fn tool_executable_path(
        &mut self,
        tool_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_executable_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        let (os, _arch) = zed::current_platform();
        let binary_name = format!(
            "sql_tool{extension}",
            extension = match os {
                Os::Mac | Os::Linux => "",
                Os::Windows => ".exe",
            }
        );

        if let Some(path) = worktree.which(&binary_name) {
            self.cached_executable_path = Some(path.clone());
            return Ok(path);
        }

        zed::set_language_server_installation_status(
            tool_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "xNaCly/sqleibniz",
            GithubReleaseOptions {
                require_assets: false,
                pre_release: false,
            },
        )?;
        let version_dir = format!("sqleibniz-{}", release.version);
        let executable_path = format!("{}/{}", version_dir, binary_name);

        if !fs::metadata(&executable_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                tool_id,
                &LanguageServerInstallationStatus::Downloading,
            );
            let repo_url = "https://github.com/xNaCly/sqleibniz";
            Self::clone_repository(repo_url, &version_dir)?;
            zed::make_file_executable(&executable_path)?;
            SqlToolExtension::cleanup_old_versions(&version_dir)?;
        }

        self.cached_executable_path = Some(executable_path.clone());
        Ok(executable_path)
    }


    fn clone_repository(repo_url: &str, destination: &str) -> Result<()> {
        let status = SystemCommand::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(destination)
            .status()
            .map_err(|e| format!("Failed to execute git: {}", e))?;

        if !status.success() {
            return Err(format!(
                "Git clone failed with status: {}",
                status
            ).into());
        }
        Ok(())
    }


    fn cleanup_old_versions(current_version_dir: &str) -> Result<()> {
        if let Ok(entries) = fs::read_dir(".") {
            for entry in entries {
                if let Ok(entry) = entry {
                    if entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name != current_version_dir)
                    {
                        let _ = fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
        Ok(())
    }
}

impl zed::Extension for SqlToolExtension {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            cached_executable_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        tool_id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Command> {
        let mut command = None;
        let mut args = vec![];
        if let Some(binary) = LspSettings::for_worktree("sqleibniz", worktree)
            .ok()
            .and_then(|settings| settings.binary)
        {
            command = binary.path;
            if let Some(arguments) = binary.arguments {
                args = arguments;
            }
        }
        Ok(Command {
            command: if let Some(command) = command {
                command
            } else {
                self.tool_executable_path(tool_id, worktree)?
            },
            args,
            env: Default::default(),
        })
    }
}

zed::register_extension!(SqlToolExtension);
