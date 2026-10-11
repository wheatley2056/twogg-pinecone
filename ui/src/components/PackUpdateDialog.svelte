<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, errorText } from "../lib/api";
  import { formatAgo, t } from "../lib/i18n.svelte";
  import type { Resolution, SnapshotInfo, UpdatePlan } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  interface Props {
    profileId: string;
    /** Ohne Ziel nur gesicherte Stände anzeigen. */
    target: { version: string | null; file: string | null } | null;
    onClose: () => void;
    onDone: () => void;
  }
  let { profileId, target, onClose, onDone }: Props = $props();

  let dialog: HTMLDialogElement;
  let plan = $state<UpdatePlan | null>(null);
  let choices = $state<Record<string, Resolution>>({});
  let snapshots = $state<SnapshotInfo[]>([]);
  let phase = $state<"planning" | "ready" | "applying" | "done" | "snapshots">("planning");
  let error = $state<string | null>(null);
  let rollbackTo = $state<SnapshotInfo | null>(null);

  const counts = $derived({
    add: plan?.changes.filter((c) => c.action === "add").length ?? 0,
    replace: plan?.changes.filter((c) => c.action === "replace").length ?? 0,
    remove: plan?.changes.filter((c) => c.action === "remove").length ?? 0,
  });

  onMount(async () => {
    dialog.showModal();
    const initial = untrack(() => target);
    snapshots = await api.packSnapshots(profileId).catch(() => []);
    if (!initial) {
      phase = "snapshots";
      return;
    }
    try {
      plan = await api.packPlanUpdate(profileId, initial.version, initial.file);
      choices = Object.fromEntries(plan.conflicts.map((c) => [c.path, c.default]));
      phase = "ready";
    } catch (e) {
      error = errorText(e);
      phase = "snapshots";
    }
  });

  async function apply() {
    phase = "applying";
    error = null;
    try {
      await api.packApplyUpdate(profileId, choices);
      snapshots = await api.packSnapshots(profileId).catch(() => []);
      phase = "done";
      onDone();
    } catch (e) {
      error = errorText(e);
      phase = "ready";
    }
  }

  async function rollback(s: SnapshotInfo) {
    rollbackTo = null;
    error = null;
    try {
      await api.packRollback(profileId, s.id);
      snapshots = await api.packSnapshots(profileId).catch(() => []);
      phase = "snapshots";
      plan = null;
      onDone();
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<dialog bind:this={dialog} oncancel={(e) => (e.preventDefault(), phase !== "applying" && onClose())}>
  <div class="dialog-body">
    <h2>{t("update.title")}</h2>
    {#if error}<p class="error">{error}</p>{/if}

    {#if phase === "planning"}
      <p class="muted">{t("update.planning")}</p>
    {:else if plan && (phase === "ready" || phase === "applying" || phase === "done")}
      <p class="summary">
        {t("update.summary", { name: plan.packName, from: plan.fromVersion, to: plan.toVersion, minecraft: plan.minecraft })}
      </p>
      <div class="stats">
        <span class="badge add">{t("update.add", { count: counts.add })}</span>
        <span class="badge">{t("update.replace", { count: counts.replace })}</span>
        <span class="badge remove">{t("update.remove", { count: counts.remove })}</span>
      </div>
      {#if plan.kept.length > 0}<p class="muted small">{t("update.kept", { count: plan.kept.length })}</p>{/if}

      {#if plan.conflicts.length > 0}
        <h3>{t("update.conflicts")}</h3>
        <p class="muted small">{t("update.conflictsHint")}</p>
        <ul class="conflicts">
          {#each plan.conflicts as c (c.path)}
            <li>
              <div class="path">
                <code>{c.path}</code>
                <span class="muted small">{t(`reason.${c.reason}`)}</span>
              </div>
              <div class="segmented" role="radiogroup" aria-label={c.path}>
                {#each [["keepMine", t("update.keepMine")], ["takePack", t("update.takePack")]] as [value, label] (value)}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={choices[c.path] === value}
                    class:selected={choices[c.path] === value}
                    disabled={phase !== "ready"}
                    onclick={() => (choices[c.path] = value as Resolution)}>{label}</button
                  >
                {/each}
              </div>
            </li>
          {/each}
        </ul>
      {/if}

      {#if plan.changes.length > 0}
        <details>
          <summary>{t("update.details")}</summary>
          <ul class="changes">
            {#each plan.changes as c (c.path)}
              <li><span class="action {c.action}">{c.action === "add" ? "+" : c.action === "remove" ? "−" : "↻"}</span><code>{c.path}</code></li>
            {/each}
          </ul>
        </details>
      {/if}
      {#if phase === "done"}<p class="notice">{t("update.done")}</p>{/if}
    {/if}

    {#if snapshots.length > 0 && phase !== "applying"}
      <h3>{t("update.snapshots")}</h3>
      <ul class="snapshots">
        {#each snapshots as s (s.id)}
          <li>
            <span>{t("update.snapshotLine", { ago: formatAgo(s.created), version: s.packVersion })}</span>
            <button class="ghost small" onclick={() => (rollbackTo = s)}>{t("update.rollback")}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <div class="dialog-actions">
    <button onclick={onClose} disabled={phase === "applying"}>{phase === "ready" ? t("common.cancel") : t("common.close")}</button>
    {#if phase === "ready" || phase === "applying"}
      <button class="primary" onclick={apply} disabled={phase === "applying"}>
        {phase === "applying" ? t("update.applying") : t("update.apply")}
      </button>
    {/if}
  </div>
</dialog>

{#if rollbackTo}
  {@const s = rollbackTo}
  <ConfirmDialog title={t("update.rollbackTitle")} confirmLabel={t("update.rollback")} onCancel={() => (rollbackTo = null)} onConfirm={() => rollback(s)}>
    <p>{t("update.rollbackText", { version: s.packVersion })}</p>
  </ConfirmDialog>
{/if}

<style>
  dialog {
    width: min(620px, calc(100vw - 32px));
  }

  .dialog-body {
    max-height: min(70vh, 640px);
    overflow-y: auto;
  }

  h3 {
    font-size: 14px;
    margin-top: 4px;
  }

  .summary {
    margin: 0;
    font-weight: 600;
  }

  .stats {
    display: flex;
    gap: 6px;
  }

  .badge.add {
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    color: var(--accent);
  }

  .badge.remove {
    background: color-mix(in srgb, var(--danger) 18%, transparent);
    color: var(--danger);
  }

  .small {
    font-size: 12px;
    margin: 0;
  }

  .notice {
    margin: 0;
    color: var(--accent);
  }

  .conflicts,
  .changes,
  .snapshots {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .conflicts li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    background: var(--surface-2);
  }

  .path {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  code {
    font-size: 12px;
    overflow-wrap: anywhere;
    user-select: text;
  }

  .segmented {
    display: flex;
    border: 1px solid var(--line-strong);
    overflow: hidden;
    flex-shrink: 0;
  }

  .segmented button {
    border: none;
    border-radius: 0;
    background: var(--bg);
    padding: 5px 10px;
    font-size: 12px;
  }

  .segmented button + button {
    border-left: 1px solid var(--line-strong);
  }

  .segmented button.selected {
    background: var(--accent);
    color: var(--ink);
    font-weight: 600;
  }

  details summary {
    cursor: pointer;
    color: var(--text-muted);
    font-size: 13px;
  }

  .changes {
    margin-top: 8px;
    max-height: 220px;
    overflow-y: auto;
    gap: 2px;
  }

  .changes li {
    display: flex;
    gap: 8px;
  }

  .action {
    width: 14px;
    text-align: center;
    font-weight: 700;
  }

  .action.add {
    color: var(--accent);
  }

  .action.remove {
    color: var(--danger);
  }

  .snapshots li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 13px;
  }
</style>
