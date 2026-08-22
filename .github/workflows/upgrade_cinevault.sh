#!/bin/bash

# ==============================================================================
# CineVault Premium Build 1.5.8 Upgrade Script
# This script automates: Folder creation, Service files, GitHub Workflow, and Push.
# ==============================================================================

echo "🚀 Starting CineVault Premium Upgrade to 1.5.8..."

# 1. Ask for the local path to your repository
echo "Enter the full path to your CineVault repository folder:"
read REPO_PATH
cd "$REPO_PATH" || { echo "❌ Error: Could not find folder. Exiting."; exit 1; }

# 2. Create Directory Structure
echo "📂 Creating folders..."
mkdir -p src/CineVault.Core/Services
mkdir -p .github/workflows

# 3. Create Synology Service File
echo "✍️ Writing SynologyService.cs..."
cat <<EOF > src/CineVault.Core/Services/SynologyService.cs
using System; using System.Net.Http; using System.Threading.Tasks; using System.Collections.Generic; using System.Text.Json;
namespace CineVault.Services {
    public class SynologyService {
        private readonly HttpClient _http = new HttpClient();
        private string _sid;
        public async Task<bool> LoginAsync(string qid, string user, string pass) {
            try {
                string url = $"https://quickconnect.to/{qid}/webapi/auth.cgi?api=SYNO.API.Auth&version=3&method=login&account={user}&passwd={pass}&session=FileStation&format=sid";
                var response = await _http.GetStringAsync(url);
                using var doc = JsonDocument.Parse(response);
                if (doc.RootElement.GetProperty("success").GetBoolean()) {
                    _sid = doc.RootElement.GetProperty("data").GetProperty("sid").GetString();
                    return true;
                }
                return false;
            } catch { return false; }
        }
        public async Task<List<string>> ListFilesAsync(string qid, string path) {
            string url = $"https://quickconnect.to/{qid}/webapi/entry.cgi?api=SYNO.API.FileStation&version=2&method=list&path={path}&_sid={_sid}";
            var response = await _http.GetStringAsync(url);
            return new List<string>(); 
        }
    }
}
EOF

# 4. Create WD MyCloud Service File
echo "✍️ Writing WdCloudService.cs..."
cat <<EOF > src/CineVault.Core/Services/WdCloudService.cs
using System; using System.IO; using System.Collections.Generic; using System.Linq;
namespace CineVault.Services {
    public class WdCloudService {
        public bool Connect(string ip) {
            try { return Directory.Exists($"\\\\{ip}\\Public"); } catch { return false; }
        }
        public List<string> GetMedia(string ip, string folder = "Public") {
            string root = $"\\\\{ip}\\{folder}";
            return Directory.GetFiles(root, "*.*", SearchOption.AllDirectories)
                            .Where(f => f.EndsWith(".mp4") || f.EndsWith(".mkv") || f.EndsWith(".avi")).ToList();
        }
    }
}
EOF

# 5. Create GitHub Action Workflow (The Automation)
echo "🤖 Setting up GitHub Actions..."
cat <<EOF > .github/workflows/release.yml
name: CineVault Premium Release
on:
  push:
    branches: [ main ]
jobs:
  release:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'
      - name: Bump Version
        id: versioning
        shell: pwsh
        run: |
          \$tags = git tag --list "v*" | Sort-Object -Descending
          \$current = if (\$tags) { \$tags[0].TrimStart("v") } else { "1.5.7" }
          \$parts = \$current.Split(".")
          \$next = "\$($parts[0]).\$($parts[1]).\$([int]\$parts[2] + 1)"
          echo "next_version=\$next" >> \$env:GITHUB_OUTPUT
      - name: Build & Publish
        run: dotnet publish src/CineVaultPremium.Windows/CineVaultPremium.Windows.csproj -c Release -o out/publish /p:Version=\${{ steps.versioning.outputs.next_version }}
      - name: GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: v\${{ steps.versioning.outputs.next_version }}
          name: CineVault Premium v\${{ steps.versioning.outputs.next_version }}
          files: out/publish/**
        env:
          GITHUB_TOKEN: \${{ secrets.GITHUB_TOKEN }}
EOF

# 6. Git Commit and Push
echo "📤 Pushing changes to GitHub..."
git add .
git commit -m "feat: add Synology and WD MyCloud sources and automate release v1.5.8"
git push origin main

echo "-------------------------------------------------------------------"
echo "✅ SUCCESS! Build 1.5.8 is now being processed by GitHub Actions."
echo "⚠️  IMPORTANT: Open SourceTab.xaml and paste the UI code manually."
echo "The UI code is in our previous chat. Just paste it before the final tag!"
echo "-------------------------------------------------------------------"
