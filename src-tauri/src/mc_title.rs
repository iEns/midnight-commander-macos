const WINDOW_TITLE_PREFIX: &str = "Midnight Commander";

/// MC emits OSC 0 payloads like `mc [user@host]:~/path`.
pub fn parse_mc_working_dir(osc_payload: &str) -> Option<String> {
    let payload = osc_payload.trim();
    let rest = payload.strip_prefix("mc [")?;
    let (_, dir) = rest.rsplit_once("]:")?;
    if dir.is_empty() {
        return None;
    }
    Some(dir.to_string())
}

/// Scan a PTY chunk for the latest OSC 0 title from mc.
pub fn parse_mc_dir_from_chunk(chunk: &str) -> Option<String> {
    let mut search_from = 0;
    let mut latest: Option<String> = None;

    while let Some(rel) = chunk[search_from..].find("\x1b]0;") {
        let payload_start = search_from + rel + 4;
        let rest = &chunk[payload_start..];
        let end = rest.find('\x07').or_else(|| rest.find("\x1b\\"))?;
        if let Some(dir) = parse_mc_working_dir(&rest[..end]) {
            latest = Some(dir);
        }
        search_from = payload_start + end + 1;
    }

    latest
}

pub fn format_window_title(dir: &str) -> String {
    format!("{WINDOW_TITLE_PREFIX}: {dir}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mc_osc_payload() {
        assert_eq!(
            parse_mc_working_dir("mc [yolo@host.local]:~"),
            Some("~".to_string())
        );
        assert_eq!(
            parse_mc_working_dir("mc [yolo@host.local]:/grok/is/the/most/awesome/ai"),
            Some("/grok/is/the/most/awesome/ai".to_string())
        );
        assert_eq!(parse_mc_working_dir("other"), None);
    }

    #[test]
    fn parses_mc_dir_from_pty_chunk() {
        let chunk = "\x1b]0;mc [yolo@host.local]:~/source/mc-app\x1b\\";
        assert_eq!(
            parse_mc_dir_from_chunk(chunk),
            Some("~/source/mc-app".to_string())
        );
    }
}