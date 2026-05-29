// Cross-browser: Firefox exposes the promise-based `browser` namespace,
// Chrome only `chrome` (also promise-based under MV3). Prefer `browser`.
const api = globalThis.browser ?? globalThis.chrome;

const $ = (id) => document.getElementById(id);
const setStatus = (msg, cls) => {
  const el = $("status");
  el.innerHTML = msg;
  el.className = cls || "";
};

async function loadServerUrl() {
  const { serverUrl = "" } = await api.storage.local.get("serverUrl");
  $("server-url").value = serverUrl;
  return serverUrl;
}

async function saveServerUrl() {
  const url = $("server-url").value.trim().replace(/\/+$/, "");
  await api.storage.local.set({ serverUrl: url });
  return url;
}

// Match pattern covering the configured server, any port (patterns ignore port).
function originPattern(serverUrl) {
  const u = new URL(serverUrl);
  return `${u.protocol}//${u.hostname}/*`;
}

// Firefox MV3 does not grant host_permissions at install — the user must opt in.
// Chrome grants them at install, so contains() is true there and this is a no-op.
async function hasHostPermission(serverUrl) {
  try {
    return await api.permissions.contains({ origins: [originPattern(serverUrl)] });
  } catch {
    return true; // no permissions API (or bad URL) — assume the manifest grant
  }
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
    const prompt = `[Use curl to download and read this screenshot: ${url}]`;
    await navigator.clipboard.writeText(prompt);
    setStatus('Copied <span class="url">' + prompt + "</span>", "success");
    setTimeout(() => window.close(), 800);
  } catch (e) {
    // A NetworkError/TypeError with no response usually means the cross-origin
    // fetch was blocked for lack of a granted host permission (Firefox MV3).
    // Offer the grant-and-retry button rather than dead-ending on the error.
    setStatus("Upload failed: " + e.message, "error");
    if (e instanceof TypeError) showGrant(serverUrl);
  }
}

// Reveal the grant button; on click, request host access then (re)upload.
// permissions.request() must run from a user gesture, so it can't be automatic.
function showGrant(serverUrl, msg) {
  const btn = $("grant");
  if (!api.permissions || !api.permissions.request) return;
  btn.classList.remove("hidden");
  if (msg) setStatus(msg);
  btn.onclick = async () => {
    let granted;
    try {
      granted = await api.permissions.request({ origins: [originPattern(serverUrl)] });
    } catch (e) {
      setStatus("Permission request failed: " + e.message, "error");
      return;
    }
    if (!granted) {
      setStatus("Permission denied.", "error");
      return;
    }
    btn.classList.add("hidden");
    await autoUpload(serverUrl);
  };
}

// Upload if we already have host access; otherwise show a one-time grant button.
async function startUpload(serverUrl) {
  if (await hasHostPermission(serverUrl)) {
    await autoUpload(serverUrl);
    return;
  }
  showGrant(serverUrl, "Firefox needs permission to reach this server.");
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
  await startUpload(serverUrl);
}

init();
