// Cross-browser: Firefox exposes the promise-based `browser` namespace,
// Chrome only `chrome` (also promise-based under MV3). Prefer `browser`.
const api = globalThis.browser ?? globalThis.chrome;

// Matches the server's insecure default; override it (both sides) in settings.
const DEFAULT_PASSWORD = "changeme";

const $ = (id) => document.getElementById(id);
const setStatus = (msg, cls) => {
  const el = $("status");
  el.innerHTML = msg;
  el.className = cls || "";
};

async function loadSettings() {
  const cfg = await api.storage.local.get({
    serverUrl: "",
    password: DEFAULT_PASSWORD,
  });
  $("server-url").value = cfg.serverUrl;
  $("password").value = cfg.password;
  return cfg;
}

async function saveSettings() {
  const serverUrl = $("server-url").value.trim().replace(/\/+$/, "");
  const password = $("password").value;
  await api.storage.local.set({ serverUrl, password });
  return { serverUrl, password };
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

async function autoUpload(cfg) {
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
    const headers = { "Content-Type": blob.type };
    if (cfg.password) headers["Authorization"] = "Basic " + btoa("user:" + cfg.password);
    const res = await fetch(cfg.serverUrl + "/upload", {
      method: "POST",
      headers,
      body: blob,
    });
    if (res.status === 401) {
      setStatus("Unauthorized — check the password in settings.", "error");
      $("settings").classList.remove("hidden");
      return;
    }
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
    if (e instanceof TypeError) showGrant(cfg);
  }
}

// Reveal the grant button; on click, request host access then (re)upload.
// permissions.request() must run from a user gesture, so it can't be automatic.
function showGrant(cfg, msg) {
  const btn = $("grant");
  if (!api.permissions || !api.permissions.request) return;
  btn.classList.remove("hidden");
  if (msg) setStatus(msg);
  btn.onclick = async () => {
    let granted;
    try {
      granted = await api.permissions.request({ origins: [originPattern(cfg.serverUrl)] });
    } catch (e) {
      setStatus("Permission request failed: " + e.message, "error");
      return;
    }
    if (!granted) {
      setStatus("Permission denied.", "error");
      return;
    }
    btn.classList.add("hidden");
    await autoUpload(cfg);
  };
}

// Upload if we already have host access; otherwise show a one-time grant button.
async function startUpload(cfg) {
  if (await hasHostPermission(cfg.serverUrl)) {
    await autoUpload(cfg);
    return;
  }
  showGrant(cfg, "Firefox needs permission to reach this server.");
}

async function init() {
  $("settings-toggle").addEventListener("click", () =>
    $("settings").classList.toggle("hidden"),
  );
  $("server-url").addEventListener("blur", saveSettings);
  $("password").addEventListener("blur", saveSettings);

  const cfg = await loadSettings();
  if (!cfg.serverUrl) {
    $("settings").classList.remove("hidden");
    setStatus("Set the server URL to begin.");
    return;
  }
  await startUpload(cfg);
}

init();
