// use tauri::window;

use tauri::window;

use serde_json::Value;

use tauri::api::dialog::confirm;
use tauri::async_runtime::spawn;
// use once_cell::sync::Lazy;

use futures::stream::{ FuturesUnordered, StreamExt };
// use libc::c_int;
use std::io::{ Read, Write };
use std::path::{ Path, PathBuf };

use tokio::process::Command;
use tokio::fs;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
//use tokio::io::{ AsyncReadExt as _, AsyncWriteExt as _ };
use which::which;

use super::VVLogger;

#[tauri::command]
pub async fn get_video_metadata(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    file_path: String
) -> std::result::Result<Value, String> {
    // Ensure the file path is valid
    if !std::path::Path::new(&file_path).exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let ffprobe_path = match resolve_executable_path(&app_handle, window.clone(), "ffprobe").await {
        Ok(path) => path,
        Err(e) => {
            confirm(
                Some(&window.clone()),
                "Install FFmpeg?",
                "Couldn't Find FFmpeg, would you like to install?",
                move |answer| {
                    if answer {
                        spawn(async move {
                            get_ffmpeg(app_handle.clone(), window.clone()).await;
                            let _ = check_ffmpeg(app_handle.clone(), window.clone()).await;
                        });
                    } else {
                        spawn(async move {
                            let _ = check_ffmpeg(app_handle.clone(), window.clone()).await;
                        });
                    }
                }
            );
            return Err(e);
        }
    };

    // VVLogger::log(
    //     &window,
    //     format!("running get metadata from location: {}", ffprobe_path.display().to_string())
    // );

    // Run ffprobe and get its output
    let mut command = Command::new(ffprobe_path);
    command.arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(file_path);

    // Apply creation flags conditionally for Windows
    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000);
    }

    // Now use the `command` variable to execute and get the output
    let output = command.output().await
                    .map_err(|e| e.to_string())?;

    // Ensure ffprobe ran successfully
    if !output.status.success() {
        return Err(format!("ffprobe failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Parse output as JSON
    let metadata: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    // VVLogger::log(&window, metadata.to_string());

    Ok(metadata)
}

// Separate the ZIP extraction logic into a new synchronous function
// use tokio::fs::File as TokioFile; // Don't forget to import this at the top
// use tokio::fs;
// use tokio::io::{self as tokio_io, AsyncReadExt as _, AsyncWriteExt as _}; // For using Tokio's async read and write methods
pub async fn download_extract_and_cleanup(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    file_url: &str,
    file_name: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Download part
    let url = reqwest::Url::parse(file_url)?;
    let response = reqwest::get(url).await?;

    if !response.status().is_success() {
        return Err(
            Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to download from URL: {}", file_url)
                )
            )
        );
    }

    // Verify Content-Length
    let expected_length = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());
    let content = response.bytes().await?;

    if let Some(expected) = expected_length {
        if content.len() != expected {
            return Err(
                Box::new(
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!(
                            "Incomplete download. Expected {} bytes but got {} bytes.",
                            expected,
                            content.len()
                        )
                    )
                )
            );
        }
    }

    let app_data_dir = app_handle
        .path_resolver()
        .app_local_data_dir()
        .ok_or_else(|| "Unable to get app data directory".to_string())?;
    let file_path = app_data_dir.join(file_name);

    // Log the save path
    VVLogger::log(&window, format!("Saving file to: {:?}", file_path));

    let mut dest = TokioFile::create(&file_path).await?;
    VVLogger::log(&window, format!("File creation status: {:?}", dest));

    // Write the already fetched content to the file
    dest.write_all(&content).await?;
    dest.flush().await?;
    drop(dest);

    if Path::new(&file_path).exists() {
        VVLogger::log(&window, "File downloaded successfully".to_string());
    } else {
        VVLogger::log(&window, "File does not exist after download".to_string());
    }

    // Perform extraction in a blocking task
    let path_buf = file_path.clone(); // Clone PathBuf to move into the blocking task
    let window_clone = window.clone();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&path_buf)?;
        let mut archive = zip::read::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = path_buf.parent().unwrap().join(file.sanitized_name());

            VVLogger::log(&window_clone, format!("Processing file: {}", file.name()));

            if file.name().ends_with('/') {
                VVLogger::log(&window_clone, "Creating directory...".to_string());
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        VVLogger::log(&window_clone, format!("Creating parent directory: {:?}", p));
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                VVLogger::log(&window_clone, "Writing to output file...".to_string());
                std::fs::write(&outpath, buffer)?;

                // Set permissions if on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Some(mode) = file.unix_mode() {
                        std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                    }
                }
            }
        }
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }).await??;

    // Remove ZIP file
    if fs::remove_file(file_path).await.is_ok() {
        VVLogger::log(&window, format!("{} zip file removed successfully!", file_name));
    }

    Ok(())
}

#[tauri::command]
pub async fn get_ffmpeg(app_handle: tauri::AppHandle, window: tauri::Window) {
    let mut files_to_download: Vec<(&str, &str)> = vec![];
    #[cfg(target_os = "macos")]
    {
        files_to_download = vec![
            ("https://vvctools.com/ffmpeg/mac/ffmpeg.zip", "ffmpeg.zip"),
            ("https://vvctools.com/ffmpeg/mac/ffprobe.zip", "ffprobe.zip")
        ];
    }
    #[cfg(windows)]
    {
        files_to_download = vec![("https://vvctools.com/ffmpeg/win/ffmpeg.zip", "ffmpeg.zip")];
    }
    #[cfg(target_os = "linux")]
    {
        files_to_download = vec![
            ("https://vvctools.com/ffmpeg/lin/ffmpeg.zip", "ffmpeg.zip"),
            ("https://vvctools.com/ffmpeg/lin/ffprobe.zip", "ffprobe.zip")
        ];
    }

    let mut tasks = FuturesUnordered::new();

    for (url, file_name) in &files_to_download {
        let app_handle = app_handle.clone();
        let url = url;
        let file_name = file_name;

        let task = download_extract_and_cleanup(app_handle, window.clone(), url, file_name);
        tasks.push(task);
    }

    let mut results = Vec::new();

    while let Some(result) = tasks.next().await {
        results.push(result);
    }

    for (index, result) in results.iter().enumerate() {
        match result {
            Ok(_) =>
                VVLogger::log(
                    &window,
                    format!("Successfully downloaded and extracted {}", files_to_download[index].1)
                ),
            Err(e) =>
                VVLogger::log(
                    &window,
                    format!(
                        "Error while downloading and extracting {}: {}",
                        files_to_download[index].1,
                        e
                    )
                ),
        }
    }
}

#[tauri::command]
pub async fn check_ffmpeg(handle: tauri::AppHandle, window: tauri::Window) -> Result<(), String> {
    // Helper function to find path (assuming this is defined elsewhere)

    let ffmpeg_window = tauri::WindowBuilder
        ::new(&handle, "ffmpeg", tauri::WindowUrl::App("ffmpeg.html".into()))
        .inner_size(400.0, 320.0)
        .max_inner_size(600.0, 400.0)
        .center()
        .title("FFmpeg Resolver")
        .closable(true)
        .build()
        .map_err(|_| "Failed to open FFmpeg window".to_string())?;

    // Retrieve FFmpeg and ffprobe paths
    let ffmpeg_path_result = resolve_executable_path(&handle, window.clone(), "ffmpeg").await;
    let ffprobe_path_result = resolve_executable_path(&handle, window.clone(), "ffprobe").await;

    // Convert results to strings for display
    let ffmpeg_display = match &ffmpeg_path_result {
        Ok(path) => path.display().to_string(),
        Err(_) => "FFmpeg not found".to_string(),
    };

    let ffprobe_display = match &ffprobe_path_result {
        Ok(path) => path.display().to_string(),
        Err(_) => "FFprobe not found".to_string(),
    };

    let ffmpeg_window_clone = ffmpeg_window.clone();

    ffmpeg_window.listen("window-ready", move |_| {
        ffmpeg_window_clone
            .emit("ffmpeg-info", (ffmpeg_display.clone(), ffprobe_display.clone()))
            .expect("Failed to emit event");
    });

    let ffmpeg_window_clone2 = ffmpeg_window.clone();
    ffmpeg_window.listen("window-close", move |_| {
        ffmpeg_window_clone2.close().unwrap();
    });

    Ok(())
}

pub async fn resolve_executable_path(
    app_handle: &tauri::AppHandle,
    window: tauri::Window,
    cmd: &str
) -> Result<PathBuf, String> {
    let is_windows = cfg!(target_os = "windows");
    let cmd_with_extension = if is_windows { format!("{}.exe", cmd) } else { cmd.to_string() };

    // Check environment paths first
    if let Ok(system_path) = which(&cmd_with_extension) {
        VVLogger::log(&window, format!("Found {} at {}", cmd, system_path.display()));
        return Ok(system_path);
    }

    // Check common locations
    let common_locations = (
        match cmd {
            "ffmpeg" | "ffprobe" =>
                vec![
                    format!("C:/ffmpeg/bin/{}.exe", cmd), // Windows
                    format!("/usr/bin/{}", cmd), // Linux
                    format!("/usr/local/bin/{}", cmd) // macOS and Linux
                ],
            _ => vec![],
        }
    )
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<PathBuf>>();

    for path in common_locations {
        if path.exists() {
            VVLogger::log(&window, format!("Found {} at {}", cmd, path.display()));
            return Ok(path);
        }
    }

    // Finally, check the application root directory
    let app_data_dir = app_handle
        .path_resolver()
        .app_local_data_dir()
        .ok_or_else(|| "Unable to get app data directory".to_string())?;
    let app_root_cmd = if is_windows {
        app_data_dir.join(format!("{}.exe", cmd))
    } else {
        app_data_dir.join(cmd)
    };
    if app_root_cmd.exists() {
        VVLogger::log(&window, format!("Found {} at {}", cmd, app_root_cmd.display()));
        return Ok(app_root_cmd);
    }

    Err(format!("{} not found", cmd))
}

#[tauri::command]
pub async fn get_ffmpeg_version(
    app_handle: tauri::AppHandle,
    window: tauri::Window
) -> Result<String, String> {
    let ffmpeg_path = resolve_executable_path(&app_handle, window.clone(), "ffmpeg").await?;

    let mut command = Command::new(&ffmpeg_path);
    command.arg("-version");

    // No terminal flash on windows
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);

    let output = command.output().await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(
            format!("Failed to get FFmpeg version: {}", String::from_utf8_lossy(&output.stderr))
        );
    }

    let version_info = String::from_utf8_lossy(&output.stdout);
    let first_line = version_info.lines().next().ok_or("Failed to read version info")?;

    Ok(first_line.to_string())
}
