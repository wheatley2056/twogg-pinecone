// Desktop-Benachrichtigungen (F-37). Ohne Berechtigung oder außerhalb von Tauri bleibt nur das Panel.
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";

let granted: Promise<boolean> | null = null;

function permission(): Promise<boolean> {
  granted ??= (async () => {
    if (await isPermissionGranted()) return true;
    return (await requestPermission()) === "granted";
  })().catch(() => false);
  return granted;
}

export async function osNotify(title: string, body?: string) {
  if (await permission()) {
    try {
      sendNotification({ title, body });
    } catch {
      // Panel-Eintrag gibt es trotzdem.
    }
  }
}
