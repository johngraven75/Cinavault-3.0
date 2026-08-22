export interface XtreamProfileForm {
  name: string;
  server_url: string;
  username: string;
  password: string;
}

export interface AddXtreamProfileArgs extends Record<string, unknown> {
  name: string;
  serverUrl: string;
  username: string;
  password: string;
}

const XTREAM_ENDPOINT_PATHS = new Set([
  "/player_api.php",
  "/get.php",
  "/xmltv.php",
]);

export function buildAddXtreamProfileArgs(
  profile: XtreamProfileForm,
): AddXtreamProfileArgs {
  const name = profile.name.trim();
  const username = profile.username.trim();
  const password = profile.password.trim();
  const serverUrl = normalizeXtreamServerUrl(profile.server_url);

  if (!name) throw new Error("Profile name is required.");
  if (!serverUrl) throw new Error("Server URL is required.");
  if (!username) throw new Error("Username is required.");
  if (!password) throw new Error("Password is required.");

  return {
    name,
    serverUrl,
    username,
    password,
  };
}

export function normalizeXtreamServerUrl(input: string): string {
  const raw = input.trim();
  if (!raw) return "";

  if (/^[a-z][a-z0-9+.-]*:\/\//i.test(raw) && !/^https?:\/\//i.test(raw)) {
    throw new Error("Server URL must start with http:// or https://.");
  }

  const urlText = /^https?:\/\//i.test(raw)
    ? raw
    : `http://${raw.replace(/^\/+/, "")}`;

  let url: URL;
  try {
    url = new URL(urlText);
  } catch {
    throw new Error(
      "Enter a valid Xtream server URL, for example http://provider.com:8080.",
    );
  }

  if (!url.hostname) {
    throw new Error(
      "Enter a valid Xtream server URL, for example http://provider.com:8080.",
    );
  }

  url.username = "";
  url.password = "";
  url.search = "";
  url.hash = "";

  const normalizedPath = url.pathname.replace(/\/+$/, "").toLowerCase();
  if (XTREAM_ENDPOINT_PATHS.has(normalizedPath)) {
    url.pathname = "/";
  }

  return url.toString().replace(/\/$/, "");
}
