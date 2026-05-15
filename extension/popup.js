const $ = (id) => document.getElementById(id);
const setStatus = (msg, cls) => {
  const el = $("status");
  el.innerHTML = msg;
  el.className = cls || "";
};

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

async function autoUpload(serverUrl) {
  setStatus("Reading clipboard…");
  let blob;
  try {
    blob = await readClipboardImage();
  } catch (e) {
    setStatus("Clipboard read failed: " + e.message, "error");
    return;
  }
  if (!blob) {
    setStatus("No image on clipboard.", "error");
    return;
  }
  setStatus("Uploading…");
  try {
    const res = await fetch(serverUrl + "/upload", {
      method: "POST",
      headers: { "Content-Type": blob.type },
      body: blob,
    });
    if (!res.ok) throw new Error("HTTP " + res.status);
    const { url } = await res.json();
    if (!url) throw new Error("no url in response");
    await navigator.clipboard.writeText(url);
    setStatus('Copied <span class="url">' + url + "</span>", "success");
    setTimeout(() => window.close(), 800);
  } catch (e) {
    setStatus("Upload failed: " + e.message, "error");
  }
}

async function init() {
  $("settings-toggle").addEventListener("click", () =>
    $("settings").classList.toggle("hidden"),
  );
  $("server-url").addEventListener("blur", saveServerUrl);

  const serverUrl = await loadServerUrl();
  if (!serverUrl) {
    $("settings").classList.remove("hidden");
    setStatus("Set the server URL to begin.");
    return;
  }
  await autoUpload(serverUrl);
}

init();
