use std::path::PathBuf;

use crate::{
    event::{
        CommandPayload,
        emit::{emit_command, emit_warning},
    },
    util::io,
};
use url::form_urlencoded;
use urlencoding::decode;

/// Handles external functions (such as through URL deep linkage)
/// Link is extracted value (link) in somewhat URL format, such as
/// subdomain1/subdomain2
/// (Does not include axolotl://)
pub async fn handle_url(sublink: &str) -> crate::Result<CommandPayload> {
    if sublink == "discovery" {
        return Ok(CommandPayload::OpenDiscovery);
    }

    if let Some(query) = sublink.strip_prefix("launch?") {
        let mut instance_id = None;
        let mut server = None;
        let mut singleplayer_world = None;

        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            match &*key {
                "instance_id" => instance_id = Some(value.into_owned()),
                "server" => server = Some(value.into_owned()),
                "singleplayer_world" => {
                    singleplayer_world = Some(value.into_owned());
                }
                _ => {}
            }
        }

        if server.is_some() && singleplayer_world.is_some() {
            return Err(crate::ErrorKind::InputError(
                "Cannot launch both a server and a singleplayer world"
                    .to_string(),
            )
            .into());
        }

        if let Some(id) = instance_id.filter(|id| !id.is_empty()) {
            return Ok(CommandPayload::LaunchInstance {
                id,
                server,
                singleplayer_world,
            });
        }
        return Err(crate::ErrorKind::InputError(
            "Launch command requires an instance_id query parameter"
                .to_string(),
        )
        .into());
    }

    // /seed-map?{query}   -    Opens the Lab seed map with a shared state
    if let Some(rest) = sublink.strip_prefix("seed-map")
        && (rest.is_empty() || rest.starts_with('?') || rest.starts_with('/'))
    {
        let query = rest.trim_start_matches('/').trim_start_matches('?');
        return Ok(CommandPayload::OpenSeedMap {
            query: query.to_string(),
        });
    }
    Ok(match sublink.split_once('/') {
        // /mod/{id}   -    Installs a mod of mod id
        Some(("mod", id)) => CommandPayload::InstallMod { id: id.to_string() },
        // /version/{id}   -    Installs a specific version of id
        Some(("version", id)) => {
            CommandPayload::InstallVersion { id: id.to_string() }
        }
        // /modpack/{id}   -    Installs a modpack of modpack id
        Some(("modpack", id)) => {
            CommandPayload::InstallModpack { id: id.to_string() }
        }
        // /server/{id}   -    Opens a server project page and triggers play flow
        Some(("server", id)) => {
            CommandPayload::InstallServer { id: id.to_string() }
        }
        // /launch/instance/{id}   -    Launches an instance
        Some(("launch", rest)) if rest.starts_with("instance/") => {
            let raw = rest.trim_start_matches("instance/");
            let (raw, query) = raw.split_once('?').unwrap_or((raw, ""));
            let mut server = None;
            let mut singleplayer_world = None;

            for (key, value) in form_urlencoded::parse(query.as_bytes()) {
                match &*key {
                    "server" => server = Some(value.into_owned()),
                    "singleplayer_world" => {
                        singleplayer_world = Some(value.into_owned());
                    }
                    _ => {}
                }
            }

            if server.is_some() && singleplayer_world.is_some() {
                emit_warning(
                    "Invalid command, cannot launch both a server and a singleplayer world",
                )
                .await?;
                return Err(crate::ErrorKind::InputError(
                    "Cannot launch both a server and a singleplayer world"
                        .to_string(),
                )
                .into());
            }

            match decode(raw) {
                Ok(decoded) => CommandPayload::LaunchInstance {
                    id: decoded.to_string(),
                    server,
                    singleplayer_world,
                },
                Err(e) => {
                    emit_warning(&format!(
                        "Invalid UTF-8 in instance path: {e}"
                    ))
                    .await?;
                    return Err(crate::ErrorKind::InputError(format!(
                        "Invalid UTF-8 in instance path: {e}"
                    ))
                    .into());
                }
            }
        }
        _ => {
            emit_warning(&format!(
                "Invalid command, unrecognized path: {sublink}"
            ))
            .await?;
            return Err(crate::ErrorKind::InputError(format!(
                "Invalid command, unrecognized path: {sublink}"
            ))
            .into());
        }
    })
}

pub async fn parse_command(
    command_string: &str,
) -> crate::Result<CommandPayload> {
    tracing::debug!("Parsing command: {}", &command_string);

    // axolotl://some-command
    // This occurs when following a web redirect link
    if let Some(sublink) = command_string.strip_prefix("axolotl://") {
        Ok(handle_url(sublink).await?)
    } else {
        // We assume anything else is a filepath to a modpack file; zip
        // archives are format-sniffed by the pack installer.
        let path = PathBuf::from(command_string);
        let path = io::canonicalize(path)?;
        if let Some(ext) = path.extension()
            && (ext == "mrpack" || ext == "zip")
        {
            return Ok(CommandPayload::RunMRPack { path });
        }
        emit_warning(&format!(
            "Invalid command, unrecognized filetype: {}",
            path.display()
        ))
        .await?;
        Err(crate::ErrorKind::InputError(format!(
            "Invalid command, unrecognized filetype: {}",
            path.display()
        ))
        .into())
    }
}

pub async fn parse_and_emit_command(command_string: &str) -> crate::Result<()> {
    let command = parse_command(command_string).await?;
    emit_command(command).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parses_launch_query_command() {
        let command =
            parse_command(
                "axolotl://launch?instance_id=example%20instance&server=example.org%3A25565",
            )
            .await
            .unwrap();
        assert!(matches!(
            command,
            CommandPayload::LaunchInstance { id, server: Some(server), singleplayer_world: None }
                if id == "example instance" && server == "example.org:25565"
        ));
    }

    #[tokio::test]
    async fn parses_discovery_command() {
        assert!(matches!(
            parse_command("axolotl://discovery").await.unwrap(),
            CommandPayload::OpenDiscovery
        ));
    }
}
