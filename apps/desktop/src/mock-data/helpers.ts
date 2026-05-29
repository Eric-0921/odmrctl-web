import type {
  Recipe,
  DryRunPlan,
  DryRunStep,
  SafetyReport,
  RunEvent,
  RawIndexEntry,
  RunManifest,
  RawArtifactSummary,
  RunSummary,
} from "./types";
import { recipe } from "./recipe";
import { dryRunPlan } from "./dryRunPlan";
import { safetyReport } from "./safetyReport";
import { runManifest } from "./runManifest";
import { events } from "./events";
import { indexEntries } from "./indexEntries";
import { rawArtifactSummary } from "./rawArtifactSummary";
import { runSummary } from "./runSummary";

/**
 * Return the bundled mock run summary.
 * All data is static and baked at build time.
 */
export function getRunSummary(): RunSummary {
  return runSummary;
}

export function getRecipe(): Recipe {
  return recipe;
}

export function getDryRunPlan(): DryRunPlan {
  return dryRunPlan;
}

export function getDryRunSteps(): DryRunStep[] {
  return dryRunPlan.steps;
}

export function getSafetyReport(): SafetyReport {
  return safetyReport;
}

export function getEvents(): RunEvent[] {
  return events;
}

export function getIndexEntries(): RawIndexEntry[] {
  return indexEntries;
}

export function getRawArtifactSummary(): RawArtifactSummary {
  return rawArtifactSummary;
}

export function getRunManifest(): RunManifest {
  return runManifest;
}
