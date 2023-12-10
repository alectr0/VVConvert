const { invoke, convertFileSrc } = window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;
const { dialog, confirm, save, open } = window.__TAURI__.dialog;
const { BaseDirectory } = window.__TAURI__;

let greetInputEl;
let greetMsgEl;
let vidEl;
let vidInfoEl;
let inPath;
let widthEl;
let heightEl;
let resolutionEl;
let bitrateKbpsEl;
let bitrateMbpsEl;
let framerateEl;
let fracFramerateEl;
let threads;
let totalFrames;
let vvencCmdEl;
let outPathEl;
let appVersionEl;
let lastKbpsValue = null;
let lastMbpsValue = null;

window.addEventListener("DOMContentLoaded", () => {
  //document.addEventListener('contextmenu', event => event.preventDefault());
  vidEl = document.querySelector("video");
  vidInfoEl = document.querySelector("#vidinfo");
  widthEl = document.querySelector("#width");
  heightEl = document.querySelector("#height");
  resolutionEl = document.querySelector("#resolution");
  bitrateKbpsEl = document.querySelector("#bitrate-kbps");
  bitrateMbpsEl = document.querySelector("#bitrate-mbps");
  framerateEl = document.querySelector("#framerate");
  fracFramerateEl = document.querySelector("#fractional-fr");
  threads = document.querySelector("#threads");
  vvencCmdEl = document.querySelector("#vvencCmd");
  outPathEl = document.querySelector("#out-path");
  appVersionEl = document.querySelector("#app-version");

  document.getElementById("filePickerButton").addEventListener("click", function () {
    console.log("Picking File...");
    filePicker();
  });
  document.querySelector("#convert-form").addEventListener("submit", (e) => {
    preventDefault(e);
    console.log("Encoding Video...");
    convert();
  });
  document.querySelector('.toggleSpoiler').addEventListener('click', function (event) {
    toggleAdvSettings(event);
  });

  outPathEl.addEventListener('change', function() {
    adjustFontSize();
});

  resolutionEl.addEventListener("change", updateSize);
  updateSize();

  widthEl.addEventListener("blur", updateOption);
  heightEl.addEventListener("blur", updateOption);

  bitrateKbpsEl.addEventListener("blur", updateBitrate);
  bitrateMbpsEl.addEventListener("blur", updateBitrate);
  //resolutionEl.onchange = updateSize;

  framerateEl.addEventListener("blur", updateFractionalFramerate);

  getAppVersion();

  // resolutionEl.addEventListener("change", vvencSampleCMD);
  // widthEl.addEventListener("blur", vvencSampleCMD);
  // heightEl.addEventListener("blur", vvencSampleCMD);
  // bitrateKbpsEl.addEventListener("blur", vvencSampleCMD);
  // bitrateMbpsEl.addEventListener("blur", vvencSampleCMD);
  // framerateEl.addEventListener("blur", vvencSampleCMD);

  // vvencSampleCMD();
  // document.querySelector("#preset").addEventListener('change', updateAndDisplayVvencCommandLine); // Assuming you have a dropdown for presets
});

function toggleAdvSettings(event) {
  let element = event.target;
  let isShown = element.getAttribute('data-show') === 'true';
  if (isShown) {
    element.textContent = "^ Advanced ^";
    element.setAttribute('data-show', 'false');
  } else {
    element.textContent = "˅ Hide ˅";
    element.setAttribute('data-show', 'true');
  }
}

function adjustFontSize() {
  const maxWidth = outPathEl.offsetWidth;
  let fontSize = parseFloat(window.getComputedStyle(outPathEl, null).getPropertyValue('font-size'));
  const initialFontSize = fontSize;

  if (outPathEl.scrollWidth > maxWidth) {
      while (outPathEl.scrollWidth > maxWidth && fontSize > 10) {
          fontSize -= 0.5;
          outPathEl.style.fontSize = fontSize + 'px';
      }
  } else {
      while (outPathEl.scrollWidth < maxWidth * 0.8 && fontSize < 24) {
          fontSize += 0.5;
          outPathEl.style.fontSize = fontSize + 'px';
      }
  }

  if (outPathEl.scrollWidth > maxWidth) {
      outPathEl.style.fontSize = initialFontSize + 'px';
  }
}


async function getAppVersion() {
  appVersionEl.textContent = await invoke("get_app_version");
}

// async function greet() {
//   const confirmed = await confirm("Are you sure?", "Tauri");
//   greetMsgEl.textContent = await invoke("greet", { name: greetInputEl.value });
// }

async function filePicker() {
  const options = {
    //defaultPath: './',
    multiple: false,
    directory: false,
    filters: [
      {
        name: "Video File (.mp4, .mov, etc.)",
        extensions: ["yuv", "mp4", "mov", "mkv", "avi", "flv", "wmv", "webm"],
      },
    ],
  };

  const result = await open(options);

  if (Array.isArray(result)) {
    // multiple files
    console.log(result);
  } else if (result === null) {
    // user cancelled
    console.log("No file was selected.");
  } else {
    // single file
    //console.log(result);
    // const inFilePath = result;
    //const inFileName = inFilePath.split("/").pop();
    getVideoMetadata(result);
    inPath = result;
    vvlogger("Selected File: " + result);
  }
}

async function convert() {
  const [numerator, denominator] = fracFramerateEl.innerText
    .split("/")
    .map(Number); // Extract the fraction and convert to numbers

  // Commented out file path logic for brevity
  if (!outPathEl.value) {
    const filePath = await save({
      filters: [
        {
          name: "VVC (h.266)",
          extensions: ["266", "vvc"],
        },
      ],
    });
    outPathEl.value = filePath;
  }

  await invoke("yuv_conversion", {
    inputFile: inPath,
    outputFile: outPathEl.value,
    width: parseInt(widthEl.value),
    height: parseInt(heightEl.value),
    bitrate: parseInt(bitrateKbpsEl.value * 1000),
    framerateNum: parseInt(numerator),
    framerateDenom: parseInt(denominator),
    threads: parseInt(threads.value),
    frames: parseInt(totalFrames)
  })
    .then((response) => {
      vvlogger(response);
    })
    .catch((error) => {
      vvlogger(error);
    });
}

async function checkffmpeg() {
  await invoke("check_ffmpeg");
}

async function checkvvc() {
  await invoke("check_vvc");
}
//function updateSize(width, height) { }

listen("tauri://file-drop", async (event) => {
  getVideoMetadata(event.payload[0]);
});

async function getVideoMetadata(videoInFile) {
  //const videoInFile = event.payload[0];
  vvlogger("Getting metadata for file " + videoInFile);
  const result = await invoke("get_video_metadata", { filePath: videoInFile });
  const format = result.streams[0].codec_name;
  const width = result.streams[0].width;
  const height = result.streams[0].height;
  const size = `${width}x${height}`;
  const bitrate = result.streams[0].bit_rate;
  const framerate = result.streams[0].r_frame_rate;
  const duration = result.streams[0].duration;

  const [numerator, denominator] = framerate.split("/").map(Number);
  const calculatedFramerate = numerator / denominator;
  totalFrames = duration * calculatedFramerate;

  var customOption = document.createElement("option");

  customOption.value = size;
  customOption.innerText = size;

  resolutionEl.appendChild(customOption);
  customOption.selected = true;
  bitrateKbpsEl.value = bitrate / 1000;
  framerateEl.value = calculatedFramerate.toFixed(2);
  fracFramerateEl.innerText = framerate;
  widthEl.value = width;
  heightEl.value = height;

  updateBitrate({ target: bitrateKbpsEl });

  vvlogger(
    `Format: ${format} | Size: ${size} | Bitrate: ${bitrate} | FPS: ${framerate} | Duration: ${duration}`
  );

  vidInfoEl.innerText = `Format: ${format} | Size: ${size} | Bitrate: ${bitrate} | FPS: ${framerate} | Duration: ${duration}`;
  
  // const source = document.createElement('source');
  // source.setAttribute('src', videoInFile);
  // source.setAttribute('type', "video/mp4");
  // vidEl.appendChild(source);
  // vidEl.play();
  const url = convertFileSrc(videoInFile);
  const fileExtension = videoInFile.split(".").pop().toLowerCase();
  let mimeType;
  switch (fileExtension) {
    case "webm":
      mimeType = "video/webm";
      break;
    case "ogg":
      mimeType = "video/ogg";
      break;
    case "mp4":
      mimeType = "video/mp4";
      break;
    case "mov":
      mimeType = "video/quicktime";
      break;
    case "avi":
      mimeType = "video/x-msvideo";
      break;
    case "wmv":
      mimeType = "video/x-ms-wmv";
      break;
    case "flv":
      mimeType = "video/x-flv";
      break;
    case "3gp":
      mimeType = "video/3gpp";
      break;
    case "3g2":
      mimeType = "video/3gpp2";
      break;
    default:
      mimeType = "video/mp4";
      break;
  }

  // fs.readBinaryFile(inPath).then(arrayBuffer => {
  // const blob = new Blob([arrayBuffer], { type: mimeType });
  // const url = URL.createObjectURL(blob);

  let source = vidEl.querySelector("source");
  if (source) {
    source.src = url;
    source.type = mimeType;
  } else {
    source = document.createElement("source");
    source.src = url;
    source.type = mimeType;
    vidEl.appendChild(source);
  }

  vidEl.load();
  vidEl.controls = true;
  vidEl.play();

  outPathEl.value = videoInFile.replace(/\.[^/.]+$/, ".266");
  adjustFontSize();
  inPath = videoInFile;
  //vvencSampleCMD();
}

function updateSize() {
  const [width, height] = resolutionEl.value.split("x");
  if (width && height) {
    [widthEl.value, heightEl.value] = [width, height];
  } else {
    console.error(`Invalid resolution format: ${resolutionEl.value}`);
  }
}

function updateOption() {
  if (widthEl.value && heightEl.value) {
    const newSize = `${widthEl.value}x${heightEl.value}`;
    let exists = false;

    for (const option of resolutionEl.options) {
      if (option.value === newSize) {
        exists = true;
        break;
      }
    }

    if (!exists) {
      const newOption = new Option(newSize, newSize);
      resolutionEl.add(newOption);
    }
    resolutionEl.value = newSize;
  }
}

function updateBitrate(event) {
  const target = event.target;

  if (target === bitrateKbpsEl) {
    const kbps = parseFloat(bitrateKbpsEl.value);
    if (!isNaN(kbps) && kbps !== lastKbpsValue) {
      const mbps = kbps / 1000;
      bitrateMbpsEl.value = mbps.toFixed(2);
      lastKbpsValue = kbps;
      lastMbpsValue = parseFloat(bitrateMbpsEl.value);
    }
  } else if (target === bitrateMbpsEl) {
    const mbps = parseFloat(bitrateMbpsEl.value);
    if (!isNaN(mbps) && mbps !== lastMbpsValue) {
      const kbps = mbps * 1000;
      bitrateKbpsEl.value = kbps.toFixed(0);
      lastMbpsValue = mbps;
      lastKbpsValue = parseFloat(bitrateKbpsEl.value);
    }
  }
}

function updateFractionalFramerate(event) {
  const target = event.target;
  if (target === framerateEl) {
    const framerate = parseFloat(framerateEl.value);
    if (!isNaN(framerate)) {
      const commonFractions = {
        11.988: "12000/1001",
        14.985: "15000/1001",
        23.976: "24000/1001",
        29.970: "30000/1001",
        59.940: "60000/1001",
      };

      const gcd = (a, b) => {
        if (b === 0) return a;
        return gcd(b, a % b);
      };

      for (let rate in commonFractions) {
        if (Math.abs(framerate - parseFloat(rate)) < 0.001) {
          fracFramerateEl.innerText = commonFractions[rate];
          return;
        }
      }

      let [whole, fraction = ""] = framerate.toString().split(".");
      let denominator = Math.pow(10, fraction.length);
      let numerator = parseInt(whole + fraction);

      let divisor = gcd(numerator, denominator);

      let simpleNumerator = numerator / divisor;
      let simpleDenominator = denominator / divisor;

      const fractionalFramerate = `${simpleNumerator}/${simpleDenominator}`;
      fracFramerateEl.innerText = fractionalFramerate;
    }
  }
}

function vvencSampleCMD() {
  //const videoPath = vidEl.source;
  const width = parseInt(widthEl.value);
  const height = parseInt(heightEl.value);
  const framerate = parseFloat(fracFramerateEl.innerText);
  const bitrate = parseInt(bitrateKbpsEl.value);
  // const preset = document.querySelector("#preset").value; // Assuming you have a dropdown for presets

  let cmd = "vvencapp "; // Use 'vvencFFapp' for the expert mode

  // Add preset
  // if (preset) {
  //   cmd += `--preset ${preset} `;
  // }

  if (inPath) {
    cmd += `-i ${inPath} `;
  }

  if (width && height) {
    cmd += `-s ${width}x${height} `;
  }

  if (framerate) {
    cmd += `-r ${framerate} `;
  }

  if (bitrate) {
    const bitrateInBps = bitrate * 1000;
    cmd += `-b ${bitrateInBps} `;
  }

  // Add number of passes for rate control (assuming 2 passes here)
  // cmd += `-p 2 `;

  vvencCmdEl.innerText = cmd;
  //vvlogger(`Sample vvenc command line: ${cmd}`);
}

// lastKbpsValue = parseFloat(bitrateKbpsEl.value) || null;
// lastMbpsValue = parseFloat(bitrateMbpsEl.value) || null;

listen("vvlogger", (event) => {
  vvlogger(event.payload);
});

listen("filepicker", (event) => {
  filePicker();
});

function vvlogger(message) {
  var consoleDiv = document.getElementById("console");
  var newLine = document.createElement("p");
  newLine.textContent = message;
  consoleDiv.appendChild(newLine);
  consoleDiv.scrollTop = consoleDiv.scrollHeight;
  console.log(message);
}

function preventDefault(e) {
  e.preventDefault();
  e.stopPropagation();
}
