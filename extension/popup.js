const $ = (id) => document.getElementById(id);
const setStatus = (msg, cls) => {
  const el = $("status");
  el.innerHTML = msg;
  el.className = cls || "";
};

let currentBlob = null;

async function loadServerUrl() {
  const { serverUrl = "" } = await chrome.storage.local.get("serverUrl");
  $("server-url").value = serverUrl;
  return serverUrl;
}

async function saveServerUrl() {
  const url = $("server-url").value.trim().replace(/\/+$/, "");
  await chrome.storage.local.set({ serverUrl: url });
  return url;
}

async function readClipboardImage() {
  const items = await navigator.clipboard.read();
  const types = ["image/png", "image/jpeg", "image/gif", "image/webp"];
  for (const item of items) {
    for (const t of types) {
      if (item.types.includes(t)) return await item.getType(t);
    }
  }
  return null;
}

function showPreview(blob) {
  $("preview").src = URL.createObjectURL(blob);
  $("upload").disabled = false;
  currentBlob = blob;
}

async function upload() {
  const serverUrl = await saveServerUrl();
  if (!serverUrl) {
    setStatus("Configure the server URL first.", "error");
    $("settings").classList.remove("hidden");
    return;
  }
  if (!currentBlob) {
    setStatus("No image to upload.", "error");
    return;
  }
  $("upload").disabled = true;
  setStatus("Uploading…");
  try {
    const res = await fetch(serverUrl + "/upload", {
      method: "POST",
      headers: { "Content-Type": currentBlob.type },
      body: currentBlob,
    });
    if (!res.ok) throw new Error("HTTP " + res.status);
    const { url } = await res.json();
    if (!url) throw new Error("no url in response");
    await navigator.clipboard.writeText(url);
    setStatus('Copied <span class="url">' + url + "</span>", "success");
    setTimeout(() => window.close(), 1200);
  } catch (e) {
    setStatus("Upload failed: " + e.message, "error");
    $("upload").disabled = false;
  }
}

async function init() {
  $("settings-toggle").addEventListener("click", () =>
    $("settings").classList.toggle("hidden"),
  );
  $("server-url").addEventListener("blur", saveServerUrl);
  $("upload").addEventListener("click", upload);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !$("upload").disabled) upload();
  });

  const serverUrl = await loadServerUrl();
  if (!serverUrl) {
    $("settings").classList.remove("hidden");
    setStatus("Set the server URL to begin.");
    return;
  }

  try {
    const blob = await readClipboardImage();
    if (!blob) {
      setStatus("No image on clipboard.", "error");
      return;
    }
    showPreview(blob);
    setStatus("Ready — press Enter or click Upload.");
  } catch (e) {
    setStatus("Clipboard read failed: " + e.message, "error");
  }
}

init();
