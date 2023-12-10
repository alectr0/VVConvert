const { invoke, convertFileSrc } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;
//const { emit } = window.__TAURI__;
const { dialog } = window.__TAURI__.dialog;
const { BaseDirectory, emit } = window.__TAURI__;

window.addEventListener("DOMContentLoaded", () => {

  document.getElementById("getffmpeg").addEventListener("click", function () {
    console.log("Getting FFmpeg...");
    getffmpeg();
  });
  document.getElementById("close").addEventListener("click", function () {
    console.log("Closing FFmpeg Resolver");
    close();
  });

  listen('ffmpeg-info', event => {
    const [ffmpegLocation, ffprobeLocation] = event.payload;

    document.getElementById('ffmpeg-info-display').textContent = ffmpegLocation;
    document.getElementById('ffprobe-info-display').textContent = ffprobeLocation;

    console.log("Received event:", event);
  });

  window.__TAURI__.event.emit("window-ready");
});

async function getffmpeg() {
  await invoke("get_ffmpeg");
}

function close() {
  window.__TAURI__.event.emit("window-close");
}
