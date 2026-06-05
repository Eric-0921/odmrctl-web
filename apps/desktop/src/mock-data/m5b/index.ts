import recipeJson from "./recipe.json";
import resolvedJson from "./resolved.json";
import safetyReportJson from "./safety_report.json";
import dryRunPlanJson from "./dry_run_plan.json";
import stationJson from "./station.json";
import deviceProfilesJson from "./deviceProfiles.json";

import type {
  M5bRecipe,
  M5bResolvedRecipe,
  M5bSafetyReport,
  M5bDryRunPlan,
  M5bStation,
  M5bDeviceProfile,
} from "../../types/m5b";

export const m5bRecipe: M5bRecipe = recipeJson as unknown as M5bRecipe;
export const m5bResolved: M5bResolvedRecipe = resolvedJson as unknown as M5bResolvedRecipe;
export const m5bSafetyReport: M5bSafetyReport = safetyReportJson as unknown as M5bSafetyReport;
export const m5bDryRunPlan: M5bDryRunPlan = dryRunPlanJson as unknown as M5bDryRunPlan;
export const m5bStation: M5bStation = stationJson as unknown as M5bStation;
export const m5bDeviceProfiles: M5bDeviceProfile[] = deviceProfilesJson as unknown as M5bDeviceProfile[];
