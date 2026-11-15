import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const dist = path.join(root, "dist");
const info = JSON.parse(fs.readFileSync(path.join(dist, "package-info.json"), "utf8"));

const version = process.env.VERSION || info.version;
const tag = process.env.TAG || `v${version}`;
const repo = process.env.GITHUB_REPOSITORY || "ycyz2505/seewo-splash-switcher";
const zipName = info.zip;
const directUrl = `https://github.com/${repo}/releases/download/${tag}/${zipName}`;

const mirrorTemplates = [
  { id: "ghproxy", name: "ghproxy.com", template: "https://ghproxy.com/{url}", enabled: true },
  { id: "ghfast", name: "ghfast.top", template: "https://ghfast.top/{url}", enabled: true },
  { id: "gh-proxy", name: "gh-proxy.com", template: "https://gh-proxy.com/{url}", enabled: true },
  { id: "gitmirror", name: "hub.gitmirror.com", template: "https://hub.gitmirror.com/{url}", enabled: true },
];

const mirrors = mirrorTemplates.map((mirror) => mirror.template.replace("{url}", directUrl));

const latest = {
  schemaVersion: 1,
  version,
  pubDate: new Date().toISOString(),
  notes: process.env.RELEASE_NOTES || "见 GitHub Release 页面。",
  minAppVersion: process.env.MIN_APP_VERSION || "0.1.0",
  asset: {
    name: zipName,
    url: directUrl,
    mirrors,
    size: info.size,
    sha256: info.sha256,
  },
};

const mirrorsFile = [
  { id: "direct", name: "GitHub 直连", template: "{url}", enabled: true },
  ...mirrorTemplates,
];

fs.writeFileSync(path.join(dist, "latest.json"), JSON.stringify(latest, null, 2) + "\n");
fs.writeFileSync(path.join(dist, "mirrors.json"), JSON.stringify(mirrorsFile, null, 2) + "\n");
console.log(`latest.json written for ${version} (${zipName})`);
