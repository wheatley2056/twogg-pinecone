// Nachrichten zwischen Pinecone (Elternfenster) und dem eingebetteten Skin-Editor (iframe).

export const SOURCE = "pinecone-skin-editor";

export type SkinModel = "classic" | "slim";

/** Pinecone → Editor */
export type ToEditor =
  | { source: typeof SOURCE; type: "load"; name: string; model: SkinModel; dataUrl: string }
  | { source: typeof SOURCE; type: "new"; name: string; model: SkinModel }
  | { source: typeof SOURCE; type: "export" };

/** Editor → Pinecone */
export type FromEditor =
  | { source: typeof SOURCE; type: "ready" }
  | { source: typeof SOURCE; type: "png"; dataUrl: string; model: SkinModel; width: number; height: number }
  | { source: typeof SOURCE; type: "error"; message: string };

/** Nachricht ohne `source` (wird beim Senden ergänzt). */
export type Unsourced<T> = T extends unknown ? Omit<T, "source"> : never;
