use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::{env, path::PathBuf};
use uuid::Uuid;
#[derive(Deserialize)]
struct BotConfig {
    token: String,
    guild_id: u64,
    prefix: char,
}

#[derive(Deserialize)]
struct MiscConfig {
    internal_log_level: String,
    external_log_level: String,
}

#[derive(Deserialize)]
struct Config {
    bot: BotConfig,
    misc: MiscConfig,
}

macro_rules! try_op {
    ($op:expr, $msg:literal) => {
        $op.map_err(|e| format!("{}: (caused by: {})", $msg, e))?
    };
}

enum ValueModifier {
    Const,
    Static,
}
fn write_value_to_file(
    file: &mut std::fs::File,
    modifier: ValueModifier,
    static_key: &str,
    value_type: impl AsRef<str>,
    value: impl AsRef<str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let modifier = match modifier {
        ValueModifier::Const => "const",
        ValueModifier::Static => "static",
    };
    let value_type = value_type.as_ref();
    let value = value.as_ref();

    try_op!(
        writeln!(file, "pub {modifier} {static_key}: {value_type} = {value};"),
        "Failed to write value to file"
    );
    Ok(())
}

fn build_main() -> Result<(), Box<dyn std::error::Error>> {
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR should be present"));

    // Generate UUID
    let new_uuid = Uuid::new_v4();
    let dest_path = out_path.join("ruscord.uuid");
    try_op!(
        std::fs::write(dest_path, new_uuid.to_bytes_le()),
        "Failed to write UUID to file"
    );

    let config = try_op!(
        std::fs::read_to_string("ruscord.toml"),
        "Failed to read config file"
    );
    let config: Config = try_op!(toml::from_str(&config), "Failed to parse config");

    let value_out_path = out_path.join("ruscord_values.rs");
    let mut file = try_op!(File::create(value_out_path), "Failed to create file");

    try_op!(
        write_value_to_file(
            &mut file,
            ValueModifier::Const,
            "TOKEN",
            format!("[u8; {}]", config.bot.token.len()),
            format!(
                "[{}]",
                config
                    .bot
                    .token
                    .as_bytes()
                    .iter()
                    .map(|b| format!("{b},"))
                    .collect::<Vec<_>>()
                    .join("")
            )
        ),
        "Failed to write token to file"
    );
    try_op!(
        write_value_to_file(
            &mut file,
            ValueModifier::Static,
            "GUILD_ID",
            "poise::serenity_prelude::GuildId",
            format!(
                "poise::serenity_prelude::GuildId::new({})",
                config.bot.guild_id
            )
        ),
        "Failed to write guild ID to file"
    );
    try_op!(
        write_value_to_file(
            &mut file,
            ValueModifier::Const,
            "PREFIX",
            "char",
            format!("'{}'", config.bot.prefix)
        ),
        "Failed to write prefix to file"
    );

    try_op!(
        write_value_to_file(
            &mut file,
            ValueModifier::Const,
            "EXTERNAL_LOG_LEVEL",
            "crate::utils::logging::LoggingLevel",
            format!(
                "crate::utils::logging::LoggingLevel::from_static(\"{}\")",
                config.misc.external_log_level
            )
        ),
        "Failed to write external log level to file"
    );

    try_op!(
        write_value_to_file(
            &mut file,
            ValueModifier::Const,
            "INTERNAL_LOG_LEVEL",
            "crate::utils::logging::LoggingLevel",
            format!(
                "crate::utils::logging::LoggingLevel::from_static(\"{}\")",
                config.misc.internal_log_level
            )
        ),
        "Failed to write internal log level to file"
    );

    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=ruscord.toml");
    println!("cargo:rerun-if-changed=build.rs");
    if let Err(why) = build_main() {
        panic!("{}", why);
    }
}
