use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(serde::Serialize, Clone)]
pub struct InstallerProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<u32>,
}

/// Download an installer from `url` to the system temp directory,
/// emit `installer-progress` events during the download, then launch
/// the installer and exit the app so it can replace the running binary.
#[tauri::command]
pub async fn download_and_run_installer(url: String, app: AppHandle) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let total = response.content_length();
    let temp_path = std::env::temp_dir().join("scrapstation-installer.exe");

    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("Failed to create temp file: {e}"))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Stream error: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("Write error: {e}"))?;
        downloaded += chunk.len() as u64;

        let percent = total.map(|t| ((downloaded as f64 / t as f64) * 100.0) as u32);
        let _ = app.emit(
            "installer-progress",
            InstallerProgress {
                downloaded,
                total,
                percent,
            },
        );
    }

    file.flush()
        .await
        .map_err(|e| format!("Flush error: {e}"))?;
    drop(file);

    // Launch installer — NSIS/Inno handles closing the running instance
    std::process::Command::new(&temp_path)
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {e}"))?;

    // Give the installer process a moment to start before we exit
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
    app.exit(0);

    Ok(())
}
