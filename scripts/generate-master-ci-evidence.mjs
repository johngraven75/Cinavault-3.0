import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outputDirectory = path.join(root, "master-evidence");
fs.mkdirSync(outputDirectory, { recursive: true });

const build = JSON.parse(fs.readFileSync(path.join(root, "build-version.json"), "utf8"));
const verification = JSON.parse(
  fs.readFileSync(path.join(root, "build-verification", "current-build.json"), "utf8"),
);
const contractManifest = JSON.parse(
  fs.readFileSync(path.join(root, "contracts", "v1", "contract-manifest.json"), "utf8"),
);

const stepNames = [
  "masterGate",
  "contracts",
  "preventiveScan",
  "install",
  "typescript",
  "tests",
  "frontend",
  "rust",
  "liveMetadata",
  "packaging",
];
const steps = Object.fromEntries(
  stepNames.map((name) => [name, process.env[`EVIDENCE_${name.toUpperCase()}`] || "not_run"]),
);
const successfulOutcomes = new Set(["success", "skipped_not_applicable"]);
const executableSteps = Object.entries(steps).filter(([, outcome]) => outcome !== "not_run");
const allExecutedStepsPassed =
  executableSteps.length > 0 &&
  executableSteps.every(([, outcome]) => successfulOutcomes.has(outcome));

const acceptance = verification.acceptanceCriteria || {};
const acceptanceSummary = Object.fromEntries(
  Object.entries(acceptance).map(([name, entry]) => [name, entry.status]),
);
const unresolvedAcceptance = Object.entries(acceptanceSummary)
  .filter(([, status]) => !["verified", "fixed_and_verified", "not_applicable", "external_blocker"].includes(status))
  .map(([name]) => name);

const riskPath = path.join(outputDirectory, "preventive-risk-findings.json");
const riskReport = fs.existsSync(riskPath)
  ? JSON.parse(fs.readFileSync(riskPath, "utf8"))
  : { build: build.displayName, findings: [{ severity: "unknown", reason: "Risk scan did not produce output" }] };

const evidence = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  repository: process.env.GITHUB_REPOSITORY || "local",
  commit: process.env.GITHUB_SHA || "local",
  workflow: process.env.GITHUB_WORKFLOW || "local",
  runId: process.env.GITHUB_RUN_ID || "local",
  build,
  masterPrompt: {
    status: verification.status,
    releaseAuthorized: verification.releaseAuthorized,
    phaseStatuses: Object.fromEntries(
      Object.entries(verification.phases || {}).map(([name, entry]) => [name, entry.status]),
    ),
    acceptanceStatuses: acceptanceSummary,
    unresolvedAcceptance,
    criticalDefects: verification.criticalDefects || [],
    externalBlockers: verification.externalBlockers || [],
  },
  contracts: {
    version: contractManifest.contractVersion,
    canonicalRepository: contractManifest.canonicalRepository,
    fixtureHashes: Object.fromEntries(
      Object.entries(contractManifest.fixtures).map(([name, entry]) => [name, entry.sha256]),
    ),
  },
  steps,
  preventiveFindings: riskReport.findings,
  allExecutedStepsPassed,
  releaseEligible:
    allExecutedStepsPassed &&
    verification.status === "verified" &&
    verification.releaseAuthorized === true &&
    unresolvedAcceptance.length === 0 &&
    (verification.criticalDefects || []).length === 0 &&
    riskReport.findings.length === 0,
};

const json = `${JSON.stringify(evidence, null, 2)}\n`;
const evidenceHash = crypto.createHash("sha256").update(json).digest("hex");
fs.writeFileSync(path.join(outputDirectory, "master-ci-evidence.json"), json);
fs.writeFileSync(path.join(outputDirectory, "master-ci-evidence.sha256"), `${evidenceHash}  master-ci-evidence.json\n`);

const statusIcon = (status) =>
  ["success", "verified", "fixed_and_verified"].includes(status) ? "✅" :
    ["external_blocker", "not_applicable", "skipped_not_applicable"].includes(status) ? "⚠️" :
      "❌";
const lines = [
  `# Master Build Evidence — ${build.displayName}`,
  "",
  `- Commit: \`${evidence.commit}\``,
  `- Evidence SHA-256: \`${evidenceHash}\``,
  `- Master status: **${verification.status}**`,
  `- Release authorized: **${verification.releaseAuthorized}**`,
  `- Release eligible from this run: **${evidence.releaseEligible}**`,
  "",
  "## CI steps",
  "",
  ...Object.entries(steps).map(([name, outcome]) => `- ${statusIcon(outcome)} ${name}: \`${outcome}\``),
  "",
  "## Build-specific acceptance",
  "",
  ...Object.entries(acceptanceSummary).map(([name, status]) => `- ${statusIcon(status)} ${name}: \`${status}\``),
  "",
  "## Preventive findings",
  "",
  ...(riskReport.findings.length
    ? riskReport.findings.map(
        (finding) => `- ❌ **${finding.severity}** ${finding.file || "repository"}: ${finding.reason}`,
      )
    : ["- ✅ No preventive findings"]),
  "",
];
fs.writeFileSync(path.join(outputDirectory, "master-ci-evidence.md"), `${lines.join("\n")}\n`);

console.log(`Master CI evidence generated: ${evidenceHash}`);
