// Laufende Installationen (Modpacks, Inhalte aus „Entdecken“) für das Aktivitäts-Panel. Der
// Fortschritt einzelner Dateien kommt nicht aus dem Backend; das Panel zeigt, was gerade lädt.

export interface Install {
  id: number;
  title: string;
  /** Logo-URL oder `null` (dann Anfangsbuchstaben). */
  iconUrl: string | null;
  /** Wohin installiert wird, z. B. „Neue Instanz“ oder der Instanzname. */
  target: string;
}

export const installs: Install[] = $state([]);

let next = 0;

/** Führt `work` aus und zeigt es so lange im Aktivitäts-Panel. */
export async function tracked<T>(entry: Omit<Install, "id">, work: () => Promise<T>): Promise<T> {
  const id = ++next;
  installs.push({ ...entry, id });
  try {
    return await work();
  } finally {
    const i = installs.findIndex((x) => x.id === id);
    if (i >= 0) installs.splice(i, 1);
  }
}
