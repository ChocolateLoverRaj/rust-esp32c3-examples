// 31 bytes for advertising, minus 2 for idk, minus 16 for service uuid
pub const SHORT_NAME_MAX_LENGTH: usize = 31 - 2 - 16;

pub fn validate_short_name(short_name: &str) -> Result<(), String> {
    let short_name = short_name.trim();
    if !short_name.is_empty() {
        if short_name.len() <= SHORT_NAME_MAX_LENGTH {
            Ok(())
        } else {
            Err(format!(
                "New short name too long: {:#?}. Not changing short name.",
                short_name
            ))
        }
    } else {
        Err("New short name is empty".into())
    }
}
