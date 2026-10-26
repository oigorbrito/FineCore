use tokio::sync::mpsc;

use crate::msg::UiMsg;

/// FineCore has no configured release channel yet. Do not invoke Orcacode's
/// installer from a FineCore binary or query Orcacode's update endpoint.
pub(crate) fn run() -> Result<(), String> {
    Err("FineCore self-update is not configured; use an official FineCore release source when available".into())
}

/// Intentionally disabled until FineCore has its own release metadata.
pub(crate) fn check_in_background(_ui: mpsc::UnboundedSender<UiMsg>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_fails_closed_without_a_finecore_release_channel() {
        let error = run().unwrap_err();
        assert!(error.contains("FineCore self-update is not configured"));
    }
}
