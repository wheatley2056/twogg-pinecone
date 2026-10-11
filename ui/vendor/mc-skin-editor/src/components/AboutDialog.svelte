<script lang="ts">
  export let onClose: () => void

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose()
  }
</script>

<svelte:window on:keydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" on:click|self={onClose}>
  <div class="dialog" role="dialog" aria-modal="true" aria-label="About SkinCrafter">
    <img class="icon" src="/favicon.svg" alt="" width="40" height="40" />
    <h2>SkinCrafter</h2>
    <div class="version">Version {APP_VERSION}</div>
    <p class="copyright">© 2026 Eric Heikes and Charity Heikes</p>

    <div class="links">
      <a href="https://github.com/eheikes/mc-skin-editor/releases" target="_blank" rel="noopener noreferrer">
        Changelog
      </a>
      <a href="https://forms.gle/BCRm2EMzR4XJMRDZ7" target="_blank" rel="noopener noreferrer"> Submit feedback </a>
    </div>

    <div class="actions">
      <button type="button" class="primary" on:click={onClose}>Close</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .dialog {
    background: var(--panel-bg, #2a2a2a);
    color: var(--text, #eee);
    border-radius: 10px;
    padding: 24px;
    width: 280px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    text-align: center;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
  }
  .icon {
    margin-bottom: 4px;
  }
  .dialog h2 {
    margin: 0;
    font-size: 18px;
  }
  .version {
    font-size: 13px;
    opacity: 0.8;
  }
  .copyright {
    margin: 8px 0 0;
    font-size: 12px;
    opacity: 0.7;
  }
  .links {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 10px;
  }
  .links a {
    color: var(--accent, #3a7bd5);
    font-size: 13px;
    text-decoration: none;
  }
  .links a:hover {
    text-decoration: underline;
  }
  .actions {
    display: flex;
    justify-content: center;
    margin-top: 16px;
  }
  .actions button {
    padding: 7px 14px;
    border-radius: 6px;
    border: 1px solid var(--border, #555);
    background: var(--panel-bg-hover, #3a3a3a);
    color: var(--text, #eee);
    cursor: pointer;
  }
  .actions button.primary {
    background: var(--accent, #3a7bd5);
    border-color: var(--accent, #3a7bd5);
    color: white;
  }
</style>
