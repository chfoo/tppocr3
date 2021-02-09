use anyhow::bail;
use slog_scope::error;

pub fn get_stream_url(webpage_link: &str, quality_format: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new("youtube-dl")
        .arg("--format")
        .arg(quality_format)
        .arg("--get-url")
        .arg(webpage_link)
        .output()?;

    if !output.status.success() {
        let error_message = std::str::from_utf8(&output.stderr)?;
        error!("youtube-dl error"; "error" => &error_message);
        bail!("youtube-dl exit status {}", output.status.code().unwrap());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}
