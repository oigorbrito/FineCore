//! Fail closed until a FineCore update channel is explicitly established.

pub(crate) fn run() -> Result<(), String> {
    Err("update unavailable: no FineCore update channel is configured".into())
}
