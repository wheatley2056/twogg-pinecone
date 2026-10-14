import { t } from "./i18n.svelte";
import type { CrashCause } from "./types";

/** Erklärender Satz zur erkannten Absturzursache (AK-35.3). */
export function crashText(cause: CrashCause): string {
  switch (cause.kind) {
    case "missingDependency":
      return t("crash.missingDependency", { mod: cause.mod, dependency: cause.dependency });
    case "javaVersion":
      return cause.required ? t("crash.javaVersion", { required: cause.required }) : t("crash.javaVersionUnknown");
    case "outOfMemory":
      return t("crash.outOfMemory");
    case "duplicateMod":
      return t("crash.duplicateMod", { id: cause.id });
    default:
      return t("crash.unknown");
  }
}
