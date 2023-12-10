// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(nonstandard_style)]
#[allow(dead_code)]
#[allow(unused_variables)]
mod vvenc_bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
mod ffmpeg_utils;
use ffmpeg_utils::*;

use tauri::Manager;
use tauri::{AboutMetadata, CustomMenuItem, Menu, MenuItem, Submenu};

use libc::c_int;

use std::ffi::CStr;
use std::process::Command;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Write};
use std::process::Stdio;
use std::io::{BufReader, BufWriter};
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::vvenc_bindings::*;

pub struct VVLogger;

impl VVLogger {
    pub fn log(window: &tauri::Window, message: String) {
        window.emit("vvlogger", Some(&message)).unwrap();
        println!("{}", message);
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    Command::new("open").args(["-R", "/"]).spawn().unwrap();
    format!("Hello, {}! Thanks for downloading my program!", name)
}

#[tauri::command]
async fn check_vvc(window: tauri::Window) -> tauri::Result<String> {
    let version_string = unsafe {
        std::ffi::CStr::from_ptr(vvenc_get_version())
            .to_str()
            .unwrap_or("Unknown Version")
            .to_string()
    };

    VVLogger::log(&window, format!("VVenC version: {}", version_string));

    Ok(version_string)
}

async fn open_file(window: tauri::Window) {
    let _response = window.emit("filepicker", "");
}

#[tauri::command]
async fn get_app_version(window: tauri::Window) -> tauri::Result<String> {
    let version_string: String;
    version_string = window.app_handle().package_info().version.to_string();
    //VVLogger::log(&window, version_string.clone()); // Log the version
    Ok(version_string) // Return the version string
}

#[tauri::command]
async fn yuv_conversion(
    app_handle: tauri::AppHandle,
    window: tauri::Window,
    input_file: String,
    output_file: String,
    width: c_int,
    height: c_int,
    bitrate: c_int,
    framerate_num: c_int,
    framerate_denom: c_int,
    threads: c_int,
    frames: c_int,
) -> std::result::Result<(), String> {
    let qp: c_int = 36;
    let preset: vvencPresetMode = vvencPresetMode_VVENC_FASTER;

    let ffmpeg_path = ffmpeg_utils::resolve_executable_path(&app_handle, window.clone(), "ffmpeg").await?;

    // Create the command separately
    let mut cmd = Command::new(ffmpeg_path);

    // Add arguments to the command
    cmd.arg("-i")
    .arg(&input_file)
    .arg("-pix_fmt")
    .arg("yuv420p")
    .arg("-f")
    .arg("rawvideo")
    .arg("-");

    // Apply creation flags conditionally for Windows
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }

    // Set the standard output to be piped
    cmd.stdout(Stdio::piped());

    // Now use the `cmd` variable to spawn the process
    let mut ffmpeg_process = cmd.spawn()
        .map_err(|e| format!("Failed to start ffmpeg process: {}", e))?;

    let mut ffmpeg_output = BufReader::new(ffmpeg_process.stdout.take().unwrap());

    let mut params: vvenc_config = unsafe { std::mem::zeroed() };
    unsafe {
        vvenc_init_default(&mut params, width, height, framerate_num / framerate_denom, bitrate, qp, preset);
    }

    params.m_outputBitDepth[0] = 8;
    params.m_internalBitDepth[0] = 8;
    params.m_inputBitDepth[0] = 8;
    params.m_outputBitDepth[1] = 8;
    params.m_internalBitDepth[1] = 8;
    params.m_inputBitDepth[1] = 8;
    params.m_SourceWidth = width;
    params.m_SourceHeight = height;
    params.m_internChromaFormat = vvencChromaFormat_VVENC_CHROMA_420;
    params.m_HdrMode = vvencHDRMode_VVENC_HDR_OFF;
    params.m_numThreads = threads;
    params.m_FrameRate = framerate_num;
    params.m_FrameScale = framerate_denom;

    VVLogger::log(&window, format!("Encoding {} frames", frames));
    VVLogger::log(&window, "Begin Encoding..".to_string());
    let start_time = Instant::now();
    let encoder = unsafe { vvenc_encoder_create() };
    let res = unsafe { vvenc_encoder_open(encoder, &mut params) };
    if res != 0 {
        return Err(format!("Unable to open encoder"));
    }

    let yuvbuf = unsafe { vvenc_YUVBuffer_alloc() };
    unsafe {
        vvenc_YUVBuffer_alloc_buffer(yuvbuf, params.m_internChromaFormat, width, height);
    }

    let au = unsafe { vvenc_accessUnit_alloc() };
    unsafe {
        vvenc_accessUnit_alloc_payload(
            au,
            ((params.m_internChromaFormat as usize) * (width as usize) * (height as usize) + 1024)
                .try_into()
                .unwrap(),
        );
    }

    let output_file = File::create(&output_file)
        .map_err(|e| format!("Cannot create output file: {}", e))?;
    let mut output_writer = BufWriter::new(output_file);

    let buffer_size = (width as usize) * (height as usize) * 3 / 2; // YUV420p
    let mut buffer = vec![0u8; buffer_size];

    for frame_index in 0..frames {
        let progress = (frame_index as f64 / frames as f64) * 100.0;
        VVLogger::log(&window, format!("Encoding progress: {:.2}%", progress));

        let yuvbuf_mut = unsafe { yuvbuf.as_mut().ok_or("Failed to get mutable YUV buffer")? };
        read_frame(&mut ffmpeg_output, yuvbuf_mut, &mut buffer)?;

        if frame_index == frames - 1 {
            break;
        }

        encode_frame(encoder, yuvbuf_mut, au, &mut output_writer)?;
    }

    flush_encoder(encoder, au, &mut output_writer)?;

    // Cleanup
    unsafe {
        vvenc_encoder_close(encoder);
        vvenc_YUVBuffer_free(yuvbuf, true);
        vvenc_accessUnit_free(au, true);
    }
    let elapsed_time = start_time.elapsed();
    VVLogger::log(&window, format!("Encoding process took: {:?}", elapsed_time));

    Ok(())
}

fn read_frame<R: Read>(
    reader: &mut R,
    yuvbuf: &mut vvencYUVBuffer,
    buffer: &mut [u8],
) -> std::result::Result<(), String> {
    for plane in yuvbuf.planes.iter_mut() {
        let plane_buffer_size = (plane.width * plane.height) as usize;
        reader.read_exact(&mut buffer[..plane_buffer_size])
            .map_err(|e| format!("Failed to read ffmpeg output: {}", e))?;
        for (i, &byte) in buffer[..plane_buffer_size].iter().enumerate() {
            unsafe {
                *plane.ptr.add(i) = byte as i16;
            }
        }
    }
    Ok(())
}

fn encode_frame(
    encoder: *mut vvencEncoder,
    yuvbuf: &mut vvencYUVBuffer,
    au: *mut vvencAccessUnit,
    writer: &mut BufWriter<File>,
) -> std::result::Result<(), String> {
    let mut enc_done = false;
    let ret = unsafe { vvenc_encode(encoder, yuvbuf, au, &mut enc_done) };
    if ret != ErrorCodes_VVENC_OK {
        let last_error = unsafe {
            let err = vvenc_get_last_error(encoder);
            CStr::from_ptr(err).to_string_lossy().into_owned()
        };
        return Err(format!("Encode error: {}", last_error));
    }

    if unsafe { (*au).payloadUsedSize } > 0 {
        let payload_size = unsafe { (*au).payloadUsedSize as usize };
        let payload_slice = unsafe { std::slice::from_raw_parts((*au).payload, payload_size) };
        writer.write_all(&payload_slice[0..payload_size])
              .map_err(|e| format!("Error writing to file: {}", e))?;
    }

    Ok(())
}

fn flush_encoder(
    encoder: *mut vvencEncoder,
    au: *mut vvencAccessUnit,
    writer: &mut BufWriter<File>,
) -> std::result::Result<(), String> {
    let mut enc_done = false;
    while !enc_done {
        let ret = unsafe { vvenc_encode(encoder, std::ptr::null_mut(), au, &mut enc_done) };
        if ret != ErrorCodes_VVENC_OK {
            let last_error = unsafe {
                let err = vvenc_get_last_error(encoder);
                CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            return Err(format!("Encode error during flushing: {}", last_error));
        }

        if unsafe { (*au).payloadUsedSize } > 0 {
            let payload_size = unsafe { (*au).payloadUsedSize as usize };
            let payload_slice = unsafe { std::slice::from_raw_parts((*au).payload, payload_size) };
            writer.write_all(&payload_slice[0..payload_size])
                  .map_err(|e| format!("Error writing to file during flushing: {}", e))?;
        }
    }

    Ok(())
}

fn create_main_menu() -> Menu {
    let mut menu = Menu::new();
    #[cfg(target_os = "macos")]
    {
        let about_metadata = AboutMetadata::new().copyright("Alec Carter 2023");

        menu = menu.add_submenu(Submenu::new(
            "VVConvert",
            Menu::new()
                .add_native_item(MenuItem::About("VVConvert".to_string(), about_metadata))
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Services)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Hide)
                .add_native_item(MenuItem::HideOthers)
                .add_native_item(MenuItem::ShowAll)
                .add_native_item(MenuItem::Separator)
                .add_native_item(MenuItem::Quit),
        ));
    }

    // Add other menus
    menu = menu
        .add_submenu(create_submenu(
            "File",
            vec![
                CustomMenuItem::new("new".to_string(), "New"),
                CustomMenuItem::new("open".to_string(), "Open"),
                // CustomMenuItem::new("save".to_string(), "Save"),
                // CustomMenuItem::new("save_as".to_string(), "Save As"),
                // CustomMenuItem::new("export".to_string(), "Export"),
                CustomMenuItem::new("exit".to_string(), "Exit"),
            ],
        ))
        // .add_submenu(create_submenu(
        //     "View",
        //     vec![
        //         CustomMenuItem::new("zoom_in".to_string(), "Zoom In"),
        //         CustomMenuItem::new("zoom_out".to_string(), "Zoom Out"),
        //         CustomMenuItem::new("reset_zoom".to_string(), "Reset Zoom"),
        //     ],
        // ))
        .add_submenu(create_submenu(
            "Tools",
            vec![
                CustomMenuItem::new("check_ffmpeg".to_string(), "Check FFmpeg"),
                CustomMenuItem::new("check_vvc".to_string(), "Check VVC"),
            ],
        ))
        .add_submenu(create_submenu(
            "Help",
            vec![
                CustomMenuItem::new("about".to_string(), "About"),
                CustomMenuItem::new("website".to_string(), "Official Website"),
                CustomMenuItem::new("check_updates".to_string(), "Check for Updates"),
            ],
        ));

    menu
}

fn create_submenu(title: &str, items: Vec<CustomMenuItem>) -> Submenu {
    let mut submenu = Menu::new();
    for item in items {
        submenu = submenu.add_item(item);
    }
    Submenu::new(title, submenu)
}

fn handle_updater_event(window: tauri::Window, event: tauri::UpdaterEvent) {
    match event {
        tauri::UpdaterEvent::UpdateAvailable {
            body,
            date,
            version,
        } => {
            VVLogger::log(
                &window,
                format!("Update available {} {:?} {}", body, date, version),
            );
        }
        tauri::UpdaterEvent::Pending => {
            VVLogger::log(&window, "Update is pending!".to_string());
        }
        tauri::UpdaterEvent::DownloadProgress {
            chunk_length,
            content_length,
        } => {
            VVLogger::log(
                &window,
                format!("Downloaded {} of {:?}", chunk_length, content_length),
            );
        }
        tauri::UpdaterEvent::Downloaded => {
            VVLogger::log(&window, "Update has been downloaded!".to_string());
        }
        tauri::UpdaterEvent::Updated => {
            VVLogger::log(&window, "VVConvert has been updated".to_string());
        }
        tauri::UpdaterEvent::AlreadyUpToDate => {
            VVLogger::log(&window, "Up to date!".to_string());
        }
        tauri::UpdaterEvent::Error(error) => {
            VVLogger::log(&window, format!("Failed to update: {}", error));
        }
        _ => (),
    }
}

fn main() {
    let menu = create_main_menu();
    // tauri::async_runtime::set(tokio::runtime::Handle::current());
    let app: tauri::App = tauri::Builder::default()
        .menu(menu)
        .on_menu_event(|event| {
            let app_handle = event.window().app_handle();
            match event.menu_item_id() {
                "new" => {
                    tauri::async_runtime::spawn(async move {
                        let _new_main_window = tauri::WindowBuilder::new(
                            &app_handle,
                            "Main", 
                            tauri::WindowUrl::App("index.html".into()),
                        )
                        .title("VVConvert")
                        .build()
                        .expect("Failed to create new main window");
                    });
                }

                "open" => {
                    tauri::async_runtime::spawn(async move {
                        let _response = open_file(event.window().clone()).await;
                    });
                }

                "website" => {
                    tauri::async_runtime::spawn(async move {
                        let _response = tauri::api::shell::open(
                            &app_handle.shell_scope(),
                            "https://vvconvert.app",
                            None,
                        )
                        .unwrap();
                    });
                }

                "check_ffmpeg" => {
                    tauri::async_runtime::spawn(async move {
                        let _response =
                            ffmpeg_utils::check_ffmpeg(app_handle, event.window().clone()).await;
                    });
                }

                "check_vvc" => {
                    tauri::async_runtime::spawn(async move {
                        let _response = check_vvc(event.window().clone()).await;
                    });
                }

                "about" => {
                    std::thread::spawn(move || {
                        let _about_window = tauri::WindowBuilder::new(
                            &app_handle,
                            "About",
                            tauri::WindowUrl::App("about.html".into()),
                        )
                        .title("About")
                        .build();
                    });
                }
                "check_updates" => {
                    tauri::async_runtime::spawn(async move {
                        let _response = app_handle.updater().check().await;
                    });
                }

                "exit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        })
        .on_window_event(|event| {
            match event.event() {
                tauri::WindowEvent::Focused(focused) => {
                    // hide window whenever it loses focus
                    if *focused && event.window().get_window("ffmpeg").is_some() {
                        event
                            .window()
                            .get_window("ffmpeg")
                            .unwrap()
                            .set_focus()
                            .unwrap();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            get_video_metadata,
            get_ffmpeg,
            check_ffmpeg,
            get_app_version,
            get_ffmpeg_version,
            check_vvc,
            yuv_conversion
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|_app_handle, event| {
        let window = _app_handle.get_window("main").unwrap();
        match event {
            tauri::RunEvent::Updater(updater_event) => handle_updater_event(window, updater_event),
            _ => {}
        }
    });
}
