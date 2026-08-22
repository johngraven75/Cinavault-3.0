# CinaVault Premium Build 166

Build 166 selects and persists the most applicable efficient Hugging Face model for CinaVault media-library automation while carrying every prior feature forward.

## AI model and authentication

- Selects `Qwen/Qwen3-4B-Instruct-2507` for multilingual title handling, instruction following, and structured metadata output.
- Verified through a real authenticated Hugging Face router request that returned valid structured JSON.
- Uses the smaller 4B instruction model so free inference credits last longer than comparable 7B/8B choices.
- Restores a valid token from `~/.cache/huggingface/token` when the CinaVault database and environment are empty.
- Seeds the recovered credential into CinaVault settings for stable subsequent startup.
- Migrates only the old Mistral default and preserves user-selected models.

## Carry-forward and packaging

- Retains all Build 165 AI real-work routing, source discovery, WD My Cloud and Synology access, adult metadata providers, poster sidecars, automatic media tools, cloud operations, casting, and plugin persistence.
- Produces a Windows MSI installer.
- Produces a Windows NSIS setup EXE.
- Publishes `SHA256SUMS.txt` with both installer hashes.
- Publishes immutable GitHub release `v1.6.6` from the verified `main` commit.
